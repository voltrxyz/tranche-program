use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, TokenAccount};

use crate::constants::*;
use crate::error::TrancheError;
use crate::events::CrankAccountingEvent;
use crate::math::{
    decimal_bits_to_u32_if_integer, Decimal, DecimalExtra, SafeMath, BPS_PRECISION,
    SECONDS_PER_YEAR,
};
use crate::state::TrancheConfig;

/// Reads a `DecimalBits` field (little-endian `[u8; 16]`) back into a `Decimal`.
fn read_decimal(bits: &[u8; 16]) -> Decimal {
    Decimal::from_decimal_bits(u128::from_le_bytes(*bits))
}

/// Writes a `Decimal` value into its `DecimalBits` representation.
fn write_decimal(val: Decimal) -> [u8; 16] {
    val.to_decimal_bits().to_le_bytes()
}

fn apply_senior_target_accrual(
    old_index: Decimal,
    index_timestamp: i64,
    now: i64,
    senior_apr: Decimal,
    has_realized_gain: bool,
) -> Result<(Decimal, i64, Decimal)> {
    if !has_realized_gain {
        return Ok((old_index, index_timestamp, Decimal::ZERO));
    }

    let elapsed = now.saturating_sub(index_timestamp) as u64;
    let growth = if elapsed > 0 {
        senior_apr
            .safe_mul(Decimal::from_num(elapsed))?
            .safe_div(Decimal::from_num(SECONDS_PER_YEAR))?
    } else {
        Decimal::ZERO
    };

    let new_index = old_index.safe_mul(Decimal::ONE.safe_add(growth)?)?;
    Ok((new_index, now, growth))
}

#[derive(Accounts)]
pub struct CrankAccounting<'info> {
    /// Permissionless -- any fee payer can trigger accounting.
    pub payer: Signer<'info>,

    /// The TrancheConfig to update. PDA seeds bind it to the vault.
    #[account(
        mut,
        seeds = [TRANCHE_CONFIG_SEED, vault.key().as_ref()],
        bump,
    )]
    pub tranche_config: AccountLoader<'info, TrancheConfig>,

    /// The underlying Voltr Vault (read-only, no CPI).
    pub vault: AccountLoader<'info, voltr_vault::state::Vault>,

    /// Vault LP token mint -- used to read total supply for pricing.
    pub vault_lp_mint: Account<'info, Mint>,

    /// Our LP token custody account -- used to read our LP balance.
    pub vault_lp_custody: Account<'info, TokenAccount>,

    /// Senior tranche token mint -- supply used for auto-pause price check.
    pub senior_mint: Account<'info, Mint>,

    /// Junior tranche token mint -- supply used for auto-pause price check.
    pub junior_mint: Account<'info, Mint>,
}

pub fn handler(ctx: Context<CrankAccounting>) -> Result<()> {
    // ── 1. Read vault state (no CPI) ─────────────────────────────────────
    let vault = ctx.accounts.vault.load()?;
    let vault_total_value = vault.get_total_asset_value();
    let vault_lp_supply = vault.get_total_lp_supply_incl_fees(ctx.accounts.vault_lp_mint.supply)?;

    require!(
        vault.lp.mint == ctx.accounts.vault_lp_mint.key(),
        TrancheError::InvalidVault
    );
    drop(vault);

    let our_lp_balance = ctx.accounts.vault_lp_custody.amount;

    // ── 2. Load config and validate remaining accounts ────────────────────
    let mut config = ctx.accounts.tranche_config.load_mut()?;

    require!(
        ctx.accounts.vault_lp_custody.key() == config.lp_custody,
        TrancheError::InvalidVault
    );
    require!(
        ctx.accounts.senior_mint.key() == config.senior_mint,
        TrancheError::InvalidVault
    );
    require!(
        ctx.accounts.junior_mint.key() == config.junior_mint,
        TrancheError::InvalidVault
    );

    // ── 3. Compute current NAV from vault state ──────────────────────────
    let current_nav = if vault_lp_supply == 0 {
        Decimal::ZERO
    } else {
        Decimal::from_num(our_lp_balance).full_mul_int_ratio(vault_total_value, vault_lp_supply)
    };

    let total_nav = read_decimal(&config.total_nav);
    let mut senior_nav = read_decimal(&config.senior_nav);
    let mut junior_nav = read_decimal(&config.junior_nav);
    let mut reserve_nav = read_decimal(&config.reserve_nav);

    let now = Clock::get()?.unix_timestamp;

    // ── 4. Compute delta and apply waterfall ─────────────────────────────
    let delta: i128;

    if current_nav >= total_nav {
        // ── GAIN PATH ────────────────────────────────────────────────
        let gain = current_nav.safe_sub(total_nav)?;
        delta = gain.to_decimal_bits() as i128;

        let senior_apr = if gain > Decimal::ZERO {
            compute_senior_apr(&config, senior_nav, current_nav)?
        } else {
            Decimal::ZERO
        };
        let (new_index, new_index_timestamp, growth) = apply_senior_target_accrual(
            read_decimal(&config.senior_target_index),
            config.index_timestamp,
            now,
            senior_apr,
            gain > Decimal::ZERO,
        )?;
        config.senior_target_index = write_decimal(new_index);
        config.index_timestamp = new_index_timestamp;

        if gain > Decimal::ZERO {
            // Step 1: Reserve fee (protocol-favoring)
            let reserve_gain =
                gain.mul_int_ratio(config.reserve_fee_bps as u128, BPS_PRECISION as u128);

            // Step 2: Net gain after reserve fee
            let net_gain = gain.safe_sub(reserve_gain)?;

            // Step 5: Senior target gain = senior_nav * growth
            let senior_target_gain = senior_nav.safe_mul(growth)?;
            let senior_gain = if senior_target_gain < net_gain {
                senior_target_gain
            } else {
                net_gain
            };

            // Step 6: Junior gets residual
            let junior_gain = net_gain.safe_sub(senior_gain)?;

            // Apply gains
            senior_nav = senior_nav.safe_add(senior_gain)?;
            junior_nav = junior_nav.safe_add(junior_gain)?;
            reserve_nav = reserve_nav.safe_add(reserve_gain)?;
        }
    } else {
        // ── LOSS PATH ────────────────────────────────────────────────
        let loss = total_nav.safe_sub(current_nav)?;
        delta = -(loss.to_decimal_bits() as i128);

        let mut remaining = loss;

        // 1. Junior absorbs first
        let junior_loss = if remaining < junior_nav {
            remaining
        } else {
            junior_nav
        };
        junior_nav = junior_nav.safe_sub(junior_loss)?;
        remaining = remaining.safe_sub(junior_loss)?;

        // 2. Reserve absorbs next
        if remaining > Decimal::ZERO {
            let reserve_loss = if remaining < reserve_nav {
                remaining
            } else {
                reserve_nav
            };
            reserve_nav = reserve_nav.safe_sub(reserve_loss)?;
            remaining = remaining.safe_sub(reserve_loss)?;
        }

        // 3. Senior absorbs last
        if remaining > Decimal::ZERO {
            let senior_loss = if remaining < senior_nav {
                remaining
            } else {
                senior_nav
            };
            senior_nav = senior_nav.safe_sub(senior_loss)?;
        }
    }

    // ── 5. Write updated NAVs ────────────────────────────────────────────
    let new_total_nav = senior_nav.safe_add(junior_nav)?.safe_add(reserve_nav)?;
    config.total_nav = write_decimal(new_total_nav);
    config.senior_nav = write_decimal(senior_nav);
    config.junior_nav = write_decimal(junior_nav);
    config.reserve_nav = write_decimal(reserve_nav);

    // ── 6. Auto-pause on junior shortfall ────────────────────────────────
    // If junior tokens exist but junior NAV is zero, junior price-per-share
    // has collapsed -- pause deposits to protect new depositors.
    if junior_nav == Decimal::ZERO && ctx.accounts.junior_mint.supply > 0 {
        config.paused |= 0x01 | 0x02;
    }

    // ── 7. Build event fields before dropping config ─────────────────────
    let base_apr = config.base_apr_bps;
    let coverage_ratio = if senior_nav > Decimal::ZERO {
        new_total_nav.safe_div(senior_nav)?.to_decimal_bits()
    } else {
        0u128
    };

    drop(config);

    // ── 8. Emit event ────────────────────────────────────────────────────
    let slot = Clock::get()?.slot;

    emit!(CrankAccountingEvent {
        delta,
        senior_nav: senior_nav.to_decimal_bits(),
        junior_nav: junior_nav.to_decimal_bits(),
        reserve_nav: reserve_nav.to_decimal_bits(),
        total_nav: new_total_nav.to_decimal_bits(),
        base_apr,
        coverage_ratio,
        slot,
    });

    Ok(())
}

/// Computes the senior APR using the risk premium formula:
/// `max(apr_floor, base_apr * (1 - risk_premium))`
/// where `risk_premium = risk_x + risk_y * (senior_tvl / total_tvl) ^ risk_k`
fn compute_senior_apr(
    config: &TrancheConfig,
    senior_nav: Decimal,
    total_nav: Decimal,
) -> Result<Decimal> {
    let floor = Decimal::from_bps(config.apr_floor_bps as u64);

    if total_nav == Decimal::ZERO || senior_nav == Decimal::ZERO {
        return Ok(floor);
    }

    let sr_ratio = senior_nav.safe_div(total_nav)?;
    let risk_k_exp = decimal_bits_to_u32_if_integer(config.risk_k)
        .ok_or(error!(TrancheError::InvalidRiskConfig))?;

    let sr_ratio_pow = sr_ratio
        .checked_pow(risk_k_exp)
        .ok_or(error!(TrancheError::MathOverflow))?;

    let risk_premium = Decimal::from_bps(config.risk_x_bps as u64)
        .safe_add(Decimal::from_bps(config.risk_y_bps as u64).safe_mul(sr_ratio_pow)?)?;

    let base_apr = Decimal::from_bps(config.base_apr_bps as u64);

    let computed = if risk_premium >= Decimal::ONE {
        Decimal::ZERO
    } else {
        base_apr.safe_mul(Decimal::ONE.safe_sub(risk_premium)?)?
    };

    Ok(if computed > floor { computed } else { floor })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_accrual_keeps_timestamp_when_no_gain_is_realized() {
        let old_index = Decimal::ONE;
        let senior_apr = Decimal::from_percent(10u64);

        let (new_index, new_timestamp, growth) =
            apply_senior_target_accrual(old_index, 100, 200, senior_apr, false).unwrap();

        assert_eq!(new_index, old_index);
        assert_eq!(new_timestamp, 100);
        assert_eq!(growth, Decimal::ZERO);
    }

    #[test]
    fn target_accrual_compounds_full_elapsed_window_on_gain() {
        let old_index = Decimal::ONE;
        let senior_apr = Decimal::from_percent(10u64);
        let now = SECONDS_PER_YEAR as i64;

        let (new_index, new_timestamp, growth) =
            apply_senior_target_accrual(old_index, 0, now, senior_apr, true).unwrap();

        assert_eq!(growth, Decimal::from_percent(10u64));
        assert_eq!(new_index, Decimal::from_percent(110u64));
        assert_eq!(new_timestamp, now);
    }
}
