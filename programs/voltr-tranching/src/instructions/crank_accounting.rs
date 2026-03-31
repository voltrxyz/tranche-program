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

/// Result of a waterfall distribution step.
#[derive(Debug, Clone, Copy)]
pub(crate) struct WaterfallResult {
    pub senior_nav: Decimal,
    pub junior_nav: Decimal,
    pub reserve_nav: Decimal,
}

/// Applies the gain waterfall: reserve fee first, then senior target, then junior residual.
pub(crate) fn apply_gain_waterfall(
    gain: Decimal,
    senior_nav: Decimal,
    junior_nav: Decimal,
    reserve_nav: Decimal,
    reserve_fee_bps: u16,
    growth: Decimal,
) -> Result<WaterfallResult> {
    if gain == Decimal::ZERO {
        return Ok(WaterfallResult {
            senior_nav,
            junior_nav,
            reserve_nav,
        });
    }

    // Step 1: Reserve fee
    let reserve_gain =
        gain.mul_int_ratio(reserve_fee_bps as u128, BPS_PRECISION as u128);

    // Step 2: Net gain after reserve fee
    let net_gain = gain.safe_sub(reserve_gain)?;

    // Step 3: Senior target gain = senior_nav * growth
    let senior_target_gain = senior_nav.safe_mul(growth)?;
    let senior_gain = if senior_target_gain < net_gain {
        senior_target_gain
    } else {
        net_gain
    };

    // Step 4: Junior gets residual
    let junior_gain = net_gain.safe_sub(senior_gain)?;

    Ok(WaterfallResult {
        senior_nav: senior_nav.safe_add(senior_gain)?,
        junior_nav: junior_nav.safe_add(junior_gain)?,
        reserve_nav: reserve_nav.safe_add(reserve_gain)?,
    })
}

/// Applies the loss waterfall: junior absorbs first, then reserve, then senior.
pub(crate) fn apply_loss_waterfall(
    loss: Decimal,
    senior_nav: Decimal,
    junior_nav: Decimal,
    reserve_nav: Decimal,
) -> Result<WaterfallResult> {
    let mut remaining = loss;
    let mut jr = junior_nav;
    let mut res = reserve_nav;
    let mut sr = senior_nav;

    // 1. Junior absorbs first
    let junior_loss = if remaining < jr { remaining } else { jr };
    jr = jr.safe_sub(junior_loss)?;
    remaining = remaining.safe_sub(junior_loss)?;

    // 2. Reserve absorbs next
    if remaining > Decimal::ZERO {
        let reserve_loss = if remaining < res { remaining } else { res };
        res = res.safe_sub(reserve_loss)?;
        remaining = remaining.safe_sub(reserve_loss)?;
    }

    // 3. Senior absorbs last
    if remaining > Decimal::ZERO {
        let senior_loss = if remaining < sr { remaining } else { sr };
        sr = sr.safe_sub(senior_loss)?;
    }

    Ok(WaterfallResult {
        senior_nav: sr,
        junior_nav: jr,
        reserve_nav: res,
    })
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
            let result = apply_gain_waterfall(
                gain,
                senior_nav,
                junior_nav,
                reserve_nav,
                config.reserve_fee_bps,
                growth,
            )?;
            senior_nav = result.senior_nav;
            junior_nav = result.junior_nav;
            reserve_nav = result.reserve_nav;
        }
    } else {
        // ── LOSS PATH ────────────────────────────────────────────────
        let loss = total_nav.safe_sub(current_nav)?;
        delta = -(loss.to_decimal_bits() as i128);

        let result = apply_loss_waterfall(loss, senior_nav, junior_nav, reserve_nav)?;
        senior_nav = result.senior_nav;
        junior_nav = result.junior_nav;
        reserve_nav = result.reserve_nav;
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

    // ─── Helper: build a minimal TrancheConfig for compute_senior_apr tests ──
    fn make_config(
        apr_floor_bps: u16,
        risk_x_bps: u16,
        risk_y_bps: u16,
        risk_k: u32,
        base_apr_bps: u16,
        reserve_fee_bps: u16,
    ) -> TrancheConfig {
        let mut config = unsafe { std::mem::zeroed::<TrancheConfig>() };
        config.apr_floor_bps = apr_floor_bps;
        config.risk_x_bps = risk_x_bps;
        config.risk_y_bps = risk_y_bps;
        config.risk_k = Decimal::from_num(risk_k).to_decimal_bits().to_le_bytes();
        config.base_apr_bps = base_apr_bps;
        config.reserve_fee_bps = reserve_fee_bps;
        config
    }

    /// Assert two Decimals are equal within a tolerance (for rounding).
    fn assert_decimal_approx(actual: Decimal, expected: Decimal, tolerance: Decimal, label: &str) {
        let diff = if actual > expected {
            actual.safe_sub(expected).unwrap()
        } else {
            expected.safe_sub(actual).unwrap()
        };
        assert!(
            diff <= tolerance,
            "{label}: expected ~{expected}, got {actual} (diff {diff} > tolerance {tolerance})"
        );
    }

    /// Rounding tolerance: 1 unit (sub-cent for 6-decimal USDC).
    fn tolerance() -> Decimal {
        Decimal::from_num(1u64)
    }

    // ═══════════════════════════════════════════════════════════════════
    // Senior Target Accrual
    // ═══════════════════════════════════════════════════════════════════

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

    #[test]
    fn target_accrual_zero_elapsed_gives_zero_growth() {
        let old_index = Decimal::ONE;
        let senior_apr = Decimal::from_percent(10u64);

        let (new_index, _, growth) =
            apply_senior_target_accrual(old_index, 100, 100, senior_apr, true).unwrap();

        assert_eq!(growth, Decimal::ZERO);
        assert_eq!(new_index, Decimal::ONE);
    }

    #[test]
    fn target_accrual_compounds_over_multiple_cranks() {
        let senior_apr = Decimal::from_percent(10u64);
        let half_year = (SECONDS_PER_YEAR / 2) as i64;

        // First half-year: index goes from 1.0 → 1.05
        let (index_1, ts_1, g1) =
            apply_senior_target_accrual(Decimal::ONE, 0, half_year, senior_apr, true).unwrap();

        assert_decimal_approx(g1, Decimal::from_percent(5u64), tolerance(), "growth_1");
        assert_decimal_approx(index_1, Decimal::from_percent(105u64), tolerance(), "index_1");

        // Second half-year: index goes from 1.05 → 1.05 * 1.05 = 1.1025
        let (index_2, _, g2) =
            apply_senior_target_accrual(index_1, ts_1, half_year * 2, senior_apr, true).unwrap();

        assert_decimal_approx(g2, Decimal::from_percent(5u64), tolerance(), "growth_2");
        let expected_index_2 = Decimal::from_num(11025u64)
            .safe_div(Decimal::from_num(10000u64))
            .unwrap();
        assert_decimal_approx(index_2, expected_index_2, tolerance(), "index_2");
    }

    // ═══════════════════════════════════════════════════════════════════
    // Risk Premium / Senior APR Formula
    // ═══════════════════════════════════════════════════════════════════

    #[test]
    fn compute_senior_apr_returns_floor_when_total_nav_zero() {
        let config = make_config(200, 500, 3000, 2, 1000, 500);
        let apr = compute_senior_apr(&config, Decimal::ZERO, Decimal::ZERO).unwrap();
        assert_eq!(apr, Decimal::from_bps(200u64));
    }

    #[test]
    fn compute_senior_apr_returns_floor_when_senior_nav_zero() {
        let config = make_config(200, 500, 3000, 2, 1000, 500);
        let apr =
            compute_senior_apr(&config, Decimal::ZERO, Decimal::from_num(1_000_000u64)).unwrap();
        assert_eq!(apr, Decimal::from_bps(200u64));
    }

    #[test]
    fn compute_senior_apr_basic_risk_premium() {
        // sr_ratio = 0.7, risk_k = 2
        // risk_premium = risk_x + risk_y * 0.7^2 = 5% + 30% * 0.49 = 5% + 14.7% = 19.7%
        // senior_apr = max(2%, 10% * (1 - 0.197)) = max(2%, 8.03%) = 8.03%
        let config = make_config(200, 500, 3000, 2, 1000, 500);
        let sr = Decimal::from_num(700_000u64);
        let total = Decimal::from_num(1_000_000u64);
        let apr = compute_senior_apr(&config, sr, total).unwrap();

        let expected = Decimal::from_bps(803u64); // 8.03%
        assert_decimal_approx(apr, expected, Decimal::from_bps(1u64), "basic_risk_premium");
    }

    #[test]
    fn compute_senior_apr_floor_kicks_in() {
        // High risk premium pushing computed APR below floor
        // risk_x=50%, risk_y=50%, risk_k=1, sr_ratio=0.7
        // risk_premium = 50% + 50% * 0.7 = 85%
        // computed = 10% * (1 - 0.85) = 1.5% < floor=3%
        let config = make_config(300, 5000, 5000, 1, 1000, 500);
        let sr = Decimal::from_num(700_000u64);
        let total = Decimal::from_num(1_000_000u64);
        let apr = compute_senior_apr(&config, sr, total).unwrap();

        assert_eq!(apr, Decimal::from_bps(300u64)); // floor = 3%
    }

    #[test]
    fn compute_senior_apr_risk_premium_exceeds_one() {
        // risk_x=80%, risk_y=80%, risk_k=1, sr_ratio=0.7
        // risk_premium = 80% + 80% * 0.7 = 136% > 100%
        // computed = 0, but floor = 2%
        let config = make_config(200, 8000, 8000, 1, 1000, 500);
        let sr = Decimal::from_num(700_000u64);
        let total = Decimal::from_num(1_000_000u64);
        let apr = compute_senior_apr(&config, sr, total).unwrap();

        assert_eq!(apr, Decimal::from_bps(200u64)); // floor
    }

    #[test]
    fn compute_senior_apr_varies_with_sr_ratio() {
        // Lower sr_ratio → lower risk_premium → higher senior APR
        let config = make_config(200, 500, 3000, 2, 1000, 500);

        // sr_ratio = 0.3: risk = 5% + 30% * 0.09 = 7.7%
        // apr = 10% * 92.3% = 9.23%
        let apr_low = compute_senior_apr(
            &config,
            Decimal::from_num(300_000u64),
            Decimal::from_num(1_000_000u64),
        )
        .unwrap();

        // sr_ratio = 0.9: risk = 5% + 30% * 0.81 = 29.3%
        // apr = 10% * 70.7% = 7.07%
        let apr_high = compute_senior_apr(
            &config,
            Decimal::from_num(900_000u64),
            Decimal::from_num(1_000_000u64),
        )
        .unwrap();

        assert!(
            apr_low > apr_high,
            "Lower sr_ratio should yield higher senior APR: {apr_low} vs {apr_high}"
        );
    }

    #[test]
    fn compute_senior_apr_risk_k_exponent_affects_curve() {
        let sr = Decimal::from_num(700_000u64);
        let total = Decimal::from_num(1_000_000u64);

        // k=1: risk_premium = 5% + 30% * 0.7 = 26%
        let config_k1 = make_config(200, 500, 3000, 1, 1000, 500);
        let apr_k1 = compute_senior_apr(&config_k1, sr, total).unwrap();

        // k=2: risk_premium = 5% + 30% * 0.49 = 19.7%
        let config_k2 = make_config(200, 500, 3000, 2, 1000, 500);
        let apr_k2 = compute_senior_apr(&config_k2, sr, total).unwrap();

        // k=3: risk_premium = 5% + 30% * 0.343 = 15.29%
        let config_k3 = make_config(200, 500, 3000, 3, 1000, 500);
        let apr_k3 = compute_senior_apr(&config_k3, sr, total).unwrap();

        // Higher k → lower risk_premium → higher senior APR (for sr_ratio < 1)
        assert!(apr_k3 > apr_k2, "k=3 should give higher APR than k=2");
        assert!(apr_k2 > apr_k1, "k=2 should give higher APR than k=1");
    }

    // ═══════════════════════════════════════════════════════════════════
    // Gain Waterfall
    // ═══════════════════════════════════════════════════════════════════

    #[test]
    fn gain_waterfall_zero_delta_navs_unchanged() {
        let sr = Decimal::from_num(700_000u64);
        let jr = Decimal::from_num(300_000u64);
        let res = Decimal::ZERO;

        let result = apply_gain_waterfall(Decimal::ZERO, sr, jr, res, 500, Decimal::ZERO).unwrap();

        assert_eq!(result.senior_nav, sr);
        assert_eq!(result.junior_nav, jr);
        assert_eq!(result.reserve_nav, res);
    }

    /// PRD Test Vector 1: Normal +10% gain
    /// Setup: 1M total, 700K senior, 300K junior, 5% reserve fee, 7% growth
    #[test]
    fn gain_waterfall_prd_normal_10_percent() {
        let sr = Decimal::from_num(700_000u64);
        let jr = Decimal::from_num(300_000u64);
        let res = Decimal::ZERO;
        let gain = Decimal::from_num(100_000u64);
        let growth = Decimal::from_percent(7u64); // 7% senior target

        let result = apply_gain_waterfall(gain, sr, jr, res, 500, growth).unwrap();

        let tol = tolerance();
        assert_decimal_approx(
            result.reserve_nav,
            Decimal::from_num(5_000u64),
            tol,
            "reserve",
        );
        assert_decimal_approx(
            result.senior_nav,
            Decimal::from_num(749_000u64),
            tol,
            "senior",
        );
        assert_decimal_approx(
            result.junior_nav,
            Decimal::from_num(346_000u64),
            tol,
            "junior",
        );

        // NAV invariant
        let total = result
            .senior_nav
            .safe_add(result.junior_nav)
            .unwrap()
            .safe_add(result.reserve_nav)
            .unwrap();
        let expected_total = sr
            .safe_add(jr)
            .unwrap()
            .safe_add(res)
            .unwrap()
            .safe_add(gain)
            .unwrap();
        assert_decimal_approx(total, expected_total, tol, "nav_invariant");
    }

    /// PRD Test Vector 2: High +15% gain
    #[test]
    fn gain_waterfall_prd_high_15_percent() {
        let sr = Decimal::from_num(700_000u64);
        let jr = Decimal::from_num(300_000u64);
        let res = Decimal::ZERO;
        let gain = Decimal::from_num(150_000u64);
        let growth = Decimal::from_percent(7u64);

        let result = apply_gain_waterfall(gain, sr, jr, res, 500, growth).unwrap();

        let tol = tolerance();
        assert_decimal_approx(
            result.reserve_nav,
            Decimal::from_num(7_500u64),
            tol,
            "reserve",
        );
        assert_decimal_approx(
            result.senior_nav,
            Decimal::from_num(749_000u64),
            tol,
            "senior",
        );
        assert_decimal_approx(
            result.junior_nav,
            Decimal::from_num(393_500u64),
            tol,
            "junior",
        );

        // NAV invariant
        let total = result
            .senior_nav
            .safe_add(result.junior_nav)
            .unwrap()
            .safe_add(result.reserve_nav)
            .unwrap();
        let expected_total = sr.safe_add(jr).unwrap().safe_add(gain).unwrap();
        assert_decimal_approx(total, expected_total, tol, "nav_invariant");
    }

    /// PRD Test Vector 3: Low +3% gain
    /// With 2.85% senior APR (from risk formula with 3% base), growth = 2.85%
    #[test]
    fn gain_waterfall_prd_low_3_percent() {
        let sr = Decimal::from_num(700_000u64);
        let jr = Decimal::from_num(300_000u64);
        let res = Decimal::ZERO;
        let gain = Decimal::from_num(30_000u64);
        // 2.85% senior target rate gives: 700K * 0.0285 = 19,950
        let growth = Decimal::from_bps(285u64); // 2.85%

        let result = apply_gain_waterfall(gain, sr, jr, res, 500, growth).unwrap();

        let tol = tolerance();
        assert_decimal_approx(
            result.reserve_nav,
            Decimal::from_num(1_500u64),
            tol,
            "reserve",
        );
        assert_decimal_approx(
            result.senior_nav,
            Decimal::from_num(719_950u64),
            tol,
            "senior",
        );
        assert_decimal_approx(
            result.junior_nav,
            Decimal::from_num(308_550u64),
            tol,
            "junior",
        );

        // NAV invariant
        let total = result
            .senior_nav
            .safe_add(result.junior_nav)
            .unwrap()
            .safe_add(result.reserve_nav)
            .unwrap();
        let expected_total = sr.safe_add(jr).unwrap().safe_add(gain).unwrap();
        assert_decimal_approx(total, expected_total, tol, "nav_invariant");
    }

    /// Gain smaller than senior target — all net gain goes to senior, zero to junior
    #[test]
    fn gain_waterfall_small_gain_all_to_senior() {
        let sr = Decimal::from_num(700_000u64);
        let jr = Decimal::from_num(300_000u64);
        let res = Decimal::ZERO;
        let gain = Decimal::from_num(10_000u64); // Very small gain
        let growth = Decimal::from_percent(7u64); // senior_target = 49K >> net_gain

        let result = apply_gain_waterfall(gain, sr, jr, res, 500, growth).unwrap();

        let tol = tolerance();
        // Reserve = 10K * 5% = 500
        assert_decimal_approx(
            result.reserve_nav,
            Decimal::from_num(500u64),
            tol,
            "reserve",
        );
        // Senior gets all of net_gain = 9,500 (capped at net_gain since target=49K > 9.5K)
        assert_decimal_approx(
            result.senior_nav,
            Decimal::from_num(709_500u64),
            tol,
            "senior",
        );
        // Junior gets zero
        assert_decimal_approx(result.junior_nav, jr, tol, "junior");

        // NAV invariant
        let total = result
            .senior_nav
            .safe_add(result.junior_nav)
            .unwrap()
            .safe_add(result.reserve_nav)
            .unwrap();
        let expected_total = sr.safe_add(jr).unwrap().safe_add(gain).unwrap();
        assert_decimal_approx(total, expected_total, tol, "nav_invariant");
    }

    /// Zero reserve fee — all gain distributed to tranches
    #[test]
    fn gain_waterfall_zero_reserve_fee() {
        let sr = Decimal::from_num(700_000u64);
        let jr = Decimal::from_num(300_000u64);
        let res = Decimal::ZERO;
        let gain = Decimal::from_num(100_000u64);
        let growth = Decimal::from_percent(7u64);

        let result = apply_gain_waterfall(gain, sr, jr, res, 0, growth).unwrap();

        let tol = tolerance();
        assert_eq!(result.reserve_nav, Decimal::ZERO);
        // senior_target = 700K * 7% = 49K, net_gain = 100K
        assert_decimal_approx(
            result.senior_nav,
            Decimal::from_num(749_000u64),
            tol,
            "senior",
        );
        // junior = 100K - 49K = 51K
        assert_decimal_approx(
            result.junior_nav,
            Decimal::from_num(351_000u64),
            tol,
            "junior",
        );
    }

    /// Gain with existing reserve balance — reserve accumulates
    #[test]
    fn gain_waterfall_accumulates_reserve() {
        let sr = Decimal::from_num(700_000u64);
        let jr = Decimal::from_num(300_000u64);
        let res = Decimal::from_num(10_000u64); // existing reserve
        let gain = Decimal::from_num(100_000u64);
        let growth = Decimal::from_percent(7u64);

        let result = apply_gain_waterfall(gain, sr, jr, res, 500, growth).unwrap();

        let tol = tolerance();
        // Reserve = existing 10K + 5K new = 15K
        assert_decimal_approx(
            result.reserve_nav,
            Decimal::from_num(15_000u64),
            tol,
            "reserve",
        );
    }

    /// Zero growth rate — all net gain to junior (senior target = 0)
    #[test]
    fn gain_waterfall_zero_growth_all_to_junior() {
        let sr = Decimal::from_num(700_000u64);
        let jr = Decimal::from_num(300_000u64);
        let res = Decimal::ZERO;
        let gain = Decimal::from_num(100_000u64);

        let result = apply_gain_waterfall(gain, sr, jr, res, 500, Decimal::ZERO).unwrap();

        let tol = tolerance();
        // Reserve = 5K
        assert_decimal_approx(
            result.reserve_nav,
            Decimal::from_num(5_000u64),
            tol,
            "reserve",
        );
        // Senior gets 0 (growth=0 means target=0)
        assert_eq!(result.senior_nav, sr);
        // Junior gets all net = 95K
        assert_decimal_approx(
            result.junior_nav,
            Decimal::from_num(395_000u64),
            tol,
            "junior",
        );
    }

    // ═══════════════════════════════════════════════════════════════════
    // Loss Waterfall
    // ═══════════════════════════════════════════════════════════════════

    /// PRD Test Vector 4: Loss -5%
    #[test]
    fn loss_waterfall_prd_loss_5_percent() {
        let sr = Decimal::from_num(700_000u64);
        let jr = Decimal::from_num(300_000u64);
        let res = Decimal::ZERO;
        let loss = Decimal::from_num(50_000u64);

        let result = apply_loss_waterfall(loss, sr, jr, res).unwrap();

        let tol = tolerance();
        // Junior absorbs all 50K (300K > 50K)
        assert_decimal_approx(
            result.junior_nav,
            Decimal::from_num(250_000u64),
            tol,
            "junior",
        );
        // Senior untouched
        assert_decimal_approx(result.senior_nav, sr, tol, "senior");
        // Reserve untouched
        assert_eq!(result.reserve_nav, Decimal::ZERO);

        // NAV invariant: total after = total before - loss
        let total = result
            .senior_nav
            .safe_add(result.junior_nav)
            .unwrap()
            .safe_add(result.reserve_nav)
            .unwrap();
        let expected = sr
            .safe_add(jr)
            .unwrap()
            .safe_add(res)
            .unwrap()
            .safe_sub(loss)
            .unwrap();
        assert_decimal_approx(total, expected, tol, "nav_invariant");
    }

    /// PRD Test Vector 5: Large loss -40%
    #[test]
    fn loss_waterfall_prd_large_loss_40_percent() {
        let sr = Decimal::from_num(700_000u64);
        let jr = Decimal::from_num(300_000u64);
        let res = Decimal::ZERO;
        let loss = Decimal::from_num(400_000u64);

        let result = apply_loss_waterfall(loss, sr, jr, res).unwrap();

        let tol = tolerance();
        // Junior wiped out: absorbs 300K
        assert_decimal_approx(result.junior_nav, Decimal::ZERO, tol, "junior");
        // Reserve: nothing to absorb (was 0)
        assert_eq!(result.reserve_nav, Decimal::ZERO);
        // Senior absorbs remaining: 400K - 300K = 100K → 700K - 100K = 600K
        assert_decimal_approx(
            result.senior_nav,
            Decimal::from_num(600_000u64),
            tol,
            "senior",
        );

        // NAV invariant
        let total = result
            .senior_nav
            .safe_add(result.junior_nav)
            .unwrap()
            .safe_add(result.reserve_nav)
            .unwrap();
        let expected = sr.safe_add(jr).unwrap().safe_sub(loss).unwrap();
        assert_decimal_approx(total, expected, tol, "nav_invariant");
    }

    /// Loss waterfall priority: junior → reserve → senior
    #[test]
    fn loss_waterfall_priority_junior_reserve_senior() {
        let sr = Decimal::from_num(700_000u64);
        let jr = Decimal::from_num(100_000u64);
        let res = Decimal::from_num(50_000u64);
        let loss = Decimal::from_num(200_000u64);

        let result = apply_loss_waterfall(loss, sr, jr, res).unwrap();

        let tol = tolerance();
        // Junior absorbs first 100K → wiped
        assert_decimal_approx(result.junior_nav, Decimal::ZERO, tol, "junior");
        // Reserve absorbs next 50K → wiped
        assert_decimal_approx(result.reserve_nav, Decimal::ZERO, tol, "reserve");
        // Senior absorbs remaining 50K → 650K
        assert_decimal_approx(
            result.senior_nav,
            Decimal::from_num(650_000u64),
            tol,
            "senior",
        );
    }

    /// Loss exactly equals junior NAV — junior wiped, senior untouched
    #[test]
    fn loss_waterfall_exact_junior_wipeout() {
        let sr = Decimal::from_num(700_000u64);
        let jr = Decimal::from_num(300_000u64);
        let res = Decimal::ZERO;
        let loss = Decimal::from_num(300_000u64);

        let result = apply_loss_waterfall(loss, sr, jr, res).unwrap();

        assert_eq!(result.junior_nav, Decimal::ZERO);
        assert_eq!(result.senior_nav, sr);
        assert_eq!(result.reserve_nav, Decimal::ZERO);
    }

    /// Junior fully wiped — loss spills to reserve then senior
    #[test]
    fn loss_waterfall_spills_through_all_tranches() {
        let sr = Decimal::from_num(500_000u64);
        let jr = Decimal::from_num(200_000u64);
        let res = Decimal::from_num(100_000u64);
        // Loss exceeds junior + reserve → hits senior
        let loss = Decimal::from_num(400_000u64);

        let result = apply_loss_waterfall(loss, sr, jr, res).unwrap();

        let tol = tolerance();
        assert_decimal_approx(result.junior_nav, Decimal::ZERO, tol, "junior");
        assert_decimal_approx(result.reserve_nav, Decimal::ZERO, tol, "reserve");
        // Senior: 500K - (400K - 200K - 100K) = 500K - 100K = 400K
        assert_decimal_approx(
            result.senior_nav,
            Decimal::from_num(400_000u64),
            tol,
            "senior",
        );
    }

    /// Catastrophic loss — everything wiped to zero
    #[test]
    fn loss_waterfall_total_wipeout() {
        let sr = Decimal::from_num(700_000u64);
        let jr = Decimal::from_num(300_000u64);
        let res = Decimal::ZERO;
        let loss = Decimal::from_num(1_000_000u64);

        let result = apply_loss_waterfall(loss, sr, jr, res).unwrap();

        assert_eq!(result.junior_nav, Decimal::ZERO);
        assert_eq!(result.reserve_nav, Decimal::ZERO);
        assert_eq!(result.senior_nav, Decimal::ZERO);
    }

    /// Small loss absorbed entirely by junior
    #[test]
    fn loss_waterfall_small_loss_junior_only() {
        let sr = Decimal::from_num(700_000u64);
        let jr = Decimal::from_num(300_000u64);
        let res = Decimal::from_num(10_000u64);
        let loss = Decimal::from_num(1_000u64);

        let result = apply_loss_waterfall(loss, sr, jr, res).unwrap();

        let tol = tolerance();
        assert_decimal_approx(
            result.junior_nav,
            Decimal::from_num(299_000u64),
            tol,
            "junior",
        );
        assert_eq!(result.senior_nav, sr);
        assert_eq!(result.reserve_nav, res);
    }

    // ═══════════════════════════════════════════════════════════════════
    // NAV Invariant
    // ═══════════════════════════════════════════════════════════════════

    /// After any gain waterfall: new_total == old_total + gain
    #[test]
    fn nav_invariant_gain_waterfall() {
        let sr = Decimal::from_num(700_000u64);
        let jr = Decimal::from_num(300_000u64);
        let res = Decimal::from_num(5_000u64);
        let gain = Decimal::from_num(77_777u64);
        let growth = Decimal::from_percent(3u64);

        let result = apply_gain_waterfall(gain, sr, jr, res, 500, growth).unwrap();

        let new_total = result
            .senior_nav
            .safe_add(result.junior_nav)
            .unwrap()
            .safe_add(result.reserve_nav)
            .unwrap();
        let expected = sr
            .safe_add(jr)
            .unwrap()
            .safe_add(res)
            .unwrap()
            .safe_add(gain)
            .unwrap();
        assert_decimal_approx(new_total, expected, tolerance(), "nav_invariant_gain");
    }

    /// After any loss waterfall: new_total == old_total - loss
    #[test]
    fn nav_invariant_loss_waterfall() {
        let sr = Decimal::from_num(500_000u64);
        let jr = Decimal::from_num(200_000u64);
        let res = Decimal::from_num(50_000u64);
        let loss = Decimal::from_num(333_333u64);

        let result = apply_loss_waterfall(loss, sr, jr, res).unwrap();

        let new_total = result
            .senior_nav
            .safe_add(result.junior_nav)
            .unwrap()
            .safe_add(result.reserve_nav)
            .unwrap();
        let expected = sr
            .safe_add(jr)
            .unwrap()
            .safe_add(res)
            .unwrap()
            .safe_sub(loss)
            .unwrap();
        assert_decimal_approx(new_total, expected, tolerance(), "nav_invariant_loss");
    }

    // ═══════════════════════════════════════════════════════════════════
    // Edge Cases
    // ═══════════════════════════════════════════════════════════════════

    /// First crank after init — all NAVs are zero, delta is zero
    #[test]
    fn first_crank_zero_navs() {
        let sr = Decimal::ZERO;
        let jr = Decimal::ZERO;
        let res = Decimal::ZERO;

        // Gain waterfall with zero gain
        let gain_result =
            apply_gain_waterfall(Decimal::ZERO, sr, jr, res, 500, Decimal::ZERO).unwrap();
        assert_eq!(gain_result.senior_nav, Decimal::ZERO);
        assert_eq!(gain_result.junior_nav, Decimal::ZERO);
        assert_eq!(gain_result.reserve_nav, Decimal::ZERO);

        // Loss waterfall with zero loss
        let loss_result = apply_loss_waterfall(Decimal::ZERO, sr, jr, res).unwrap();
        assert_eq!(loss_result.senior_nav, Decimal::ZERO);
        assert_eq!(loss_result.junior_nav, Decimal::ZERO);
        assert_eq!(loss_result.reserve_nav, Decimal::ZERO);
    }

    /// Max reserve fee (20%) — large reserve cut
    #[test]
    fn gain_waterfall_max_reserve_fee() {
        let sr = Decimal::from_num(700_000u64);
        let jr = Decimal::from_num(300_000u64);
        let res = Decimal::ZERO;
        let gain = Decimal::from_num(100_000u64);
        let growth = Decimal::from_percent(7u64);

        let result = apply_gain_waterfall(gain, sr, jr, res, 2000, growth).unwrap();

        let tol = tolerance();
        // Reserve = 100K * 20% = 20K
        assert_decimal_approx(
            result.reserve_nav,
            Decimal::from_num(20_000u64),
            tol,
            "reserve",
        );
        // Net = 80K, senior target = 49K, so senior gets 49K
        assert_decimal_approx(
            result.senior_nav,
            Decimal::from_num(749_000u64),
            tol,
            "senior",
        );
        // Junior = 80K - 49K = 31K
        assert_decimal_approx(
            result.junior_nav,
            Decimal::from_num(331_000u64),
            tol,
            "junior",
        );
    }

    /// Very large values — ensure no overflow via Decimal precision
    #[test]
    fn gain_waterfall_large_values() {
        let sr = Decimal::from_num(700_000_000_000u64); // 700B
        let jr = Decimal::from_num(300_000_000_000u64); // 300B
        let res = Decimal::ZERO;
        let gain = Decimal::from_num(100_000_000_000u64); // 100B
        let growth = Decimal::from_percent(7u64);

        let result = apply_gain_waterfall(gain, sr, jr, res, 500, growth).unwrap();

        let tol = Decimal::from_num(1u64);
        // Scale: same ratios as normal test vector
        assert_decimal_approx(
            result.reserve_nav,
            Decimal::from_num(5_000_000_000u64),
            tol,
            "reserve",
        );
        assert_decimal_approx(
            result.senior_nav,
            Decimal::from_num(749_000_000_000u64),
            tol,
            "senior",
        );
        assert_decimal_approx(
            result.junior_nav,
            Decimal::from_num(346_000_000_000u64),
            tol,
            "junior",
        );
    }

    /// Senior has zero NAV — all net gain to junior
    #[test]
    fn gain_waterfall_zero_senior_nav() {
        let sr = Decimal::ZERO;
        let jr = Decimal::from_num(1_000_000u64);
        let res = Decimal::ZERO;
        let gain = Decimal::from_num(50_000u64);
        let growth = Decimal::from_percent(7u64);

        let result = apply_gain_waterfall(gain, sr, jr, res, 500, growth).unwrap();

        let tol = tolerance();
        // senior_target = 0 * 7% = 0, so senior gets 0
        assert_eq!(result.senior_nav, Decimal::ZERO);
        // Reserve = 2500
        assert_decimal_approx(
            result.reserve_nav,
            Decimal::from_num(2_500u64),
            tol,
            "reserve",
        );
        // Junior gets all net = 47,500
        assert_decimal_approx(
            result.junior_nav,
            Decimal::from_num(1_047_500u64),
            tol,
            "junior",
        );
    }

    /// Junior has zero NAV — loss goes straight to reserve then senior
    #[test]
    fn loss_waterfall_zero_junior_nav() {
        let sr = Decimal::from_num(700_000u64);
        let jr = Decimal::ZERO;
        let res = Decimal::from_num(50_000u64);
        let loss = Decimal::from_num(100_000u64);

        let result = apply_loss_waterfall(loss, sr, jr, res).unwrap();

        let tol = tolerance();
        assert_eq!(result.junior_nav, Decimal::ZERO);
        // Reserve absorbs 50K
        assert_decimal_approx(result.reserve_nav, Decimal::ZERO, tol, "reserve");
        // Senior absorbs remaining 50K → 650K
        assert_decimal_approx(
            result.senior_nav,
            Decimal::from_num(650_000u64),
            tol,
            "senior",
        );
    }

    /// Reserve absorbs partial loss (between junior and senior)
    #[test]
    fn loss_waterfall_reserve_absorbs_partial() {
        let sr = Decimal::from_num(700_000u64);
        let jr = Decimal::from_num(100_000u64);
        let res = Decimal::from_num(200_000u64);
        // Loss = 250K: junior absorbs 100K, reserve absorbs 150K (out of 200K)
        let loss = Decimal::from_num(250_000u64);

        let result = apply_loss_waterfall(loss, sr, jr, res).unwrap();

        let tol = tolerance();
        assert_decimal_approx(result.junior_nav, Decimal::ZERO, tol, "junior");
        assert_decimal_approx(
            result.reserve_nav,
            Decimal::from_num(50_000u64),
            tol,
            "reserve",
        );
        assert_decimal_approx(result.senior_nav, sr, tol, "senior");
    }
}
