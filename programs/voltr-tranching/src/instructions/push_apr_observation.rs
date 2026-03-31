use anchor_lang::prelude::*;
use anchor_spl::token::Mint;

use crate::constants::*;
use crate::error::TrancheError;
use crate::math::{Decimal, DecimalExtra, SafeMath, SECONDS_PER_YEAR};
use crate::state::{AprFeed, TrancheConfig, MAX_APR_OBSERVATIONS};

#[derive(Accounts)]
pub struct PushAprObservation<'info> {
    #[account(
        mut,
        seeds = [TRANCHE_CONFIG_SEED, tranche_config.load()?.vault.as_ref()],
        bump = tranche_config.load()?.tranche_config_bump,
    )]
    pub tranche_config: AccountLoader<'info, TrancheConfig>,

    #[account(
        mut,
        seeds = [APR_FEED_SEED, tranche_config.key().as_ref()],
        bump = tranche_config.load()?.apr_feed_bump,
    )]
    pub apr_feed: AccountLoader<'info, AprFeed>,

    #[account(
        address = tranche_config.load()?.vault @ TrancheError::InvalidVault,
    )]
    pub vault: AccountLoader<'info, voltr_vault::state::Vault>,

    #[account(
        address = vault.load()?.lp.mint @ TrancheError::InvalidVault,
    )]
    pub vault_lp_mint: Account<'info, Mint>,
}

pub fn handle_push_apr_observation(ctx: Context<PushAprObservation>) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;

    // ── Read vault state ──────────────────────────────────────────
    let vault = ctx.accounts.vault.load()?;
    let vault_nav = vault.asset.total_value;
    let vault_lp_supply = ctx.accounts.vault_lp_mint.supply;
    drop(vault);

    // ── Push observation to ring buffer ───────────────────────────
    let mut feed = ctx.accounts.apr_feed.load_mut()?;

    let nav_decimal = Decimal::from_num(vault_nav);
    let supply_decimal = Decimal::from_num(vault_lp_supply);

    let write_idx = feed.head as usize;
    feed.observations[write_idx].vault_nav = nav_decimal.to_decimal_bits().to_le_bytes();
    feed.observations[write_idx].vault_lp_supply = supply_decimal.to_decimal_bits().to_le_bytes();
    feed.observations[write_idx].timestamp = now;
    feed.observations[write_idx]._padding = [0u8; 8];

    // Advance head, wrap at capacity
    feed.head = ((feed.head as usize + 1) % MAX_APR_OBSERVATIONS) as u16;

    // Increment count, cap at capacity
    if (feed.count as usize) < MAX_APR_OBSERVATIONS {
        feed.count += 1;
    }

    // ── Derive base APR from oldest and newest observations ───────
    // Need at least 2 observations to compute APR
    if feed.count >= 2 {
        // Newest observation is the one we just wrote
        let newest_idx = write_idx;

        // Oldest observation: if buffer is full, oldest is at head (just wrapped);
        // otherwise oldest is at index 0
        let oldest_idx = if (feed.count as usize) >= MAX_APR_OBSERVATIONS {
            feed.head as usize // head now points to the oldest after wrap
        } else {
            0
        };

        let newest = &feed.observations[newest_idx];
        let oldest = &feed.observations[oldest_idx];

        let time_window = newest.timestamp.saturating_sub(oldest.timestamp);

        // Only compute APR if we have a meaningful time window (> 0 seconds)
        if time_window > 0 {
            let newest_nav =
                Decimal::from_decimal_bits(u128::from_le_bytes(newest.vault_nav));
            let newest_supply =
                Decimal::from_decimal_bits(u128::from_le_bytes(newest.vault_lp_supply));
            let oldest_nav =
                Decimal::from_decimal_bits(u128::from_le_bytes(oldest.vault_nav));
            let oldest_supply =
                Decimal::from_decimal_bits(u128::from_le_bytes(oldest.vault_lp_supply));

            // Guard against zero LP supply
            if newest_supply > Decimal::ZERO && oldest_supply > Decimal::ZERO {
                let nav_per_lp_now = newest_nav.safe_div(newest_supply)?;
                let nav_per_lp_oldest = oldest_nav.safe_div(oldest_supply)?;

                // If nav_per_lp has grown, compute annualized rate
                if nav_per_lp_now > nav_per_lp_oldest {
                    // growth = nav_per_lp_now / nav_per_lp_oldest - 1
                    let growth = nav_per_lp_now
                        .safe_div(nav_per_lp_oldest)?
                        .safe_sub(Decimal::ONE)?;

                    // annualized = growth * SECONDS_PER_YEAR / time_window
                    let annualized = growth
                        .safe_mul(Decimal::from_num(SECONDS_PER_YEAR))?
                        .safe_div(Decimal::from_num(time_window as u64))?;

                    // Convert to bps (annualized * 10_000)
                    let apr_bps: u64 = annualized.to_bps().unwrap_or(0);

                    // Cap at u16::MAX to avoid overflow
                    let apr_bps_u16 = if apr_bps > u16::MAX as u64 {
                        u16::MAX
                    } else {
                        apr_bps as u16
                    };

                    drop(feed);
                    let mut config = ctx.accounts.tranche_config.load_mut()?;
                    config.base_apr_bps = apr_bps_u16;
                    return Ok(());
                }

                // NAV per LP declined or flat — APR is 0
                drop(feed);
                let mut config = ctx.accounts.tranche_config.load_mut()?;
                config.base_apr_bps = 0;
                return Ok(());
            }
        }
    }

    // First observation or zero time window — keep last known APR (no update)
    Ok(())
}
