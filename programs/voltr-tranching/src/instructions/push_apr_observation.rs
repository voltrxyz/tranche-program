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
    // Use total LP supply including accrued fee LP and dead weight,
    // matching crank_accounting and the vault's own share pricing.
    let vault = ctx.accounts.vault.load()?;
    let vault_nav = vault.asset.total_value;
    let vault_lp_supply =
        vault.get_total_lp_supply_incl_fees(ctx.accounts.vault_lp_mint.supply)?;
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

    // ── Derive base APR ─────────────────────────────────────────────
    // Need at least 2 observations to compute APR.
    if feed.count < 2 {
        return Ok(());
    }

    let max_age = ctx.accounts.tranche_config.load()?.max_observation_age_seconds;

    // Find the best oldest candidate by scanning populated observations
    // in chronological order. Skip entries that are stale, have zero
    // LP supply, or share the same timestamp as the newest sample.
    let newest = &feed.observations[write_idx];
    let newest_ts = newest.timestamp;

    let best_oldest_idx = find_best_oldest_observation(
        &feed.observations,
        feed.count as usize,
        feed.head as usize,
        newest_ts,
        max_age,
    );

    let best_oldest_idx = match best_oldest_idx {
        Some(idx) => idx,
        None => return Ok(()), // No valid candidate — keep last known APR
    };

    let oldest = &feed.observations[best_oldest_idx];
    let time_window = newest_ts.saturating_sub(oldest.timestamp);

    if time_window <= 0 {
        return Ok(());
    }

    let newest_nav = Decimal::from_decimal_bits(u128::from_le_bytes(newest.vault_nav));
    let newest_supply =
        Decimal::from_decimal_bits(u128::from_le_bytes(newest.vault_lp_supply));
    let oldest_nav = Decimal::from_decimal_bits(u128::from_le_bytes(oldest.vault_nav));
    let oldest_supply =
        Decimal::from_decimal_bits(u128::from_le_bytes(oldest.vault_lp_supply));

    // Guard against zero LP supply (newest was already validated by caller,
    // but double-check for safety)
    if newest_supply == Decimal::ZERO || oldest_supply == Decimal::ZERO {
        return Ok(());
    }

    let nav_per_lp_now = newest_nav.safe_div(newest_supply)?;
    let nav_per_lp_oldest = oldest_nav.safe_div(oldest_supply)?;

    let apr_bps_u16 = if nav_per_lp_now > nav_per_lp_oldest {
        // growth = nav_per_lp_now / nav_per_lp_oldest - 1
        let growth = nav_per_lp_now
            .safe_div(nav_per_lp_oldest)?
            .safe_sub(Decimal::ONE)?;

        // annualized = growth * SECONDS_PER_YEAR / time_window
        let annualized = growth
            .safe_mul(Decimal::from_num(SECONDS_PER_YEAR))?
            .safe_div(Decimal::from_num(time_window as u64))?;

        // Convert to bps, cap at u16::MAX
        let apr_bps: u64 = annualized.to_bps().unwrap_or(0);
        if apr_bps > u16::MAX as u64 {
            u16::MAX
        } else {
            apr_bps as u16
        }
    } else {
        // NAV per LP declined or flat — APR is 0
        0u16
    };

    drop(feed);
    let mut config = ctx.accounts.tranche_config.load_mut()?;
    config.base_apr_bps = apr_bps_u16;
    Ok(())
}

use crate::state::AprObservation;

/// Scans populated observations in chronological order and returns the index
/// of the earliest valid candidate for APR computation.
///
/// A candidate is valid when:
/// - Its timestamp is > 0 (not uninitialized)
/// - Its timestamp differs from the newest observation's timestamp
/// - Its vault LP supply is nonzero
/// - Its age relative to `newest_ts` is <= `max_age` (if `max_age > 0`)
fn find_best_oldest_observation(
    observations: &[AprObservation; MAX_APR_OBSERVATIONS],
    count: usize,
    head: usize,
    newest_ts: i64,
    max_age: u64,
) -> Option<usize> {
    // Iterate in chronological order (oldest first).
    // The oldest entry is at index `head` when the buffer has wrapped,
    // otherwise at index 0.
    let start = if count >= MAX_APR_OBSERVATIONS {
        head
    } else {
        0
    };

    for i in 0..count {
        let idx = (start + i) % MAX_APR_OBSERVATIONS;
        let obs = &observations[idx];

        // Skip uninitialized
        if obs.timestamp == 0 {
            continue;
        }

        // Skip if same timestamp as newest (zero time window)
        if obs.timestamp >= newest_ts {
            continue;
        }

        // Skip zero LP supply
        let supply = u128::from_le_bytes(obs.vault_lp_supply);
        if supply == 0 {
            continue;
        }

        // Check staleness
        let age = newest_ts.saturating_sub(obs.timestamp) as u64;
        if max_age > 0 && age > max_age {
            continue; // Too old — try next (more recent) observation
        }

        return Some(idx);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: write an observation into the ring buffer at the current head,
    /// advance head, and increment count.
    fn push_obs(feed: &mut AprFeed, nav: u64, lp_supply: u64, timestamp: i64) {
        let idx = feed.head as usize;
        feed.observations[idx].vault_nav =
            Decimal::from_num(nav).to_decimal_bits().to_le_bytes();
        feed.observations[idx].vault_lp_supply =
            Decimal::from_num(lp_supply).to_decimal_bits().to_le_bytes();
        feed.observations[idx].timestamp = timestamp;
        feed.observations[idx]._padding = [0u8; 8];
        feed.head = ((feed.head as usize + 1) % MAX_APR_OBSERVATIONS) as u16;
        if (feed.count as usize) < MAX_APR_OBSERVATIONS {
            feed.count += 1;
        }
    }

    fn zeroed_feed() -> AprFeed {
        unsafe { std::mem::zeroed::<AprFeed>() }
    }

    // ═══════════════════════════════════════════════════════════════════
    // find_best_oldest_observation
    // ═══════════════════════════════════════════════════════════════════

    #[test]
    fn skips_zero_supply_first_observation_and_finds_valid_one() {
        // Issue 2 regression: first observation has zero LP supply.
        // Old code would always pick index 0, never computing APR until
        // the buffer wrapped. New code skips it and picks the next valid one.
        let mut feed = zeroed_feed();

        // Observation 0: zero LP supply (e.g., empty vault at init)
        push_obs(&mut feed, 0, 0, 1000);
        // Observation 1: valid
        push_obs(&mut feed, 1_000_000, 1_000, 2000);
        // Observation 2: newest
        push_obs(&mut feed, 1_050_000, 1_000, 3000);

        let newest_ts = 3000;
        let result = find_best_oldest_observation(
            &feed.observations,
            feed.count as usize,
            feed.head as usize,
            newest_ts,
            0, // no max_age limit
        );

        // Should pick index 1 (the first valid observation), NOT index 0
        assert_eq!(result, Some(1));
    }

    #[test]
    fn stale_oldest_skipped_fresh_observations_used() {
        // Issue 2 regression: oldest observation is stale (age > max_age).
        // Fresh observations following it should be picked instead.
        let mut feed = zeroed_feed();

        let max_age = 7200u64; // 2 hours

        // Observation 0: very old (stale — age 99000 > 7200)
        push_obs(&mut feed, 1_000_000, 1_000, 1000);
        // Observation 1: also stale (age 98000 > 7200)
        push_obs(&mut feed, 1_010_000, 1_000, 2000);
        // Observation 2: fresh (age 3000 < 7200)
        push_obs(&mut feed, 1_030_000, 1_000, 97_000);
        // Observation 3: newest
        push_obs(&mut feed, 1_050_000, 1_000, 100_000);

        let newest_ts = 100_000;
        let result = find_best_oldest_observation(
            &feed.observations,
            feed.count as usize,
            feed.head as usize,
            newest_ts,
            max_age,
        );

        // Index 0 is 99000s old (stale), index 1 is 98000s old (stale),
        // index 2 is 3000s old (fresh) — should be picked
        assert_eq!(result, Some(2));
    }

    #[test]
    fn all_stale_returns_none() {
        let mut feed = zeroed_feed();

        // All observations are older than max_age
        push_obs(&mut feed, 1_000_000, 1_000, 1000);
        push_obs(&mut feed, 1_010_000, 1_000, 2000);
        // Newest
        push_obs(&mut feed, 1_050_000, 1_000, 100_000);

        let result = find_best_oldest_observation(
            &feed.observations,
            feed.count as usize,
            feed.head as usize,
            100_000,
            3600, // 1 hour max — all older observations are > 1h old
        );

        assert_eq!(result, None);
    }

    #[test]
    fn no_max_age_picks_earliest_valid() {
        let mut feed = zeroed_feed();

        push_obs(&mut feed, 1_000_000, 1_000, 1000);
        push_obs(&mut feed, 1_010_000, 1_000, 2000);
        push_obs(&mut feed, 1_050_000, 1_000, 3000);

        let result = find_best_oldest_observation(
            &feed.observations,
            feed.count as usize,
            feed.head as usize,
            3000,
            0, // no max_age limit
        );

        assert_eq!(result, Some(0));
    }

    #[test]
    fn wrapped_buffer_picks_correct_oldest() {
        let mut feed = zeroed_feed();

        // Fill the entire buffer to force wrap
        for i in 0..MAX_APR_OBSERVATIONS {
            push_obs(
                &mut feed,
                1_000_000 + (i as u64 * 1000),
                1_000,
                (i as i64 + 1) * 3600,
            );
        }

        // Push one more to overwrite index 0
        let newest_ts = (MAX_APR_OBSERVATIONS as i64 + 1) * 3600;
        push_obs(&mut feed, 1_100_000, 1_000, newest_ts);

        // After wrap: head=1, oldest should be at index 1 (head after push)
        let result = find_best_oldest_observation(
            &feed.observations,
            feed.count as usize,
            feed.head as usize,
            newest_ts,
            0,
        );

        // The observation at head (index 1 after the second wrap) is the oldest
        assert_eq!(result, Some(feed.head as usize));
    }

    #[test]
    fn skips_observation_with_same_timestamp_as_newest() {
        let mut feed = zeroed_feed();

        push_obs(&mut feed, 1_000_000, 1_000, 1000);
        push_obs(&mut feed, 1_010_000, 1_000, 2000);
        // Same timestamp as "newest" — should be skipped
        push_obs(&mut feed, 1_050_000, 1_000, 2000);

        let result = find_best_oldest_observation(
            &feed.observations,
            feed.count as usize,
            feed.head as usize,
            2000,
            0,
        );

        assert_eq!(result, Some(0));
    }

    #[test]
    fn single_observation_returns_none() {
        let mut feed = zeroed_feed();
        push_obs(&mut feed, 1_000_000, 1_000, 1000);

        let result = find_best_oldest_observation(
            &feed.observations,
            feed.count as usize,
            feed.head as usize,
            1000,
            0,
        );

        // Only one observation — it has same timestamp as newest, so None
        assert_eq!(result, None);
    }

    #[test]
    fn mixed_zero_supply_and_stale_finds_first_valid() {
        // Combines both issues: zero supply entries AND stale entries
        let mut feed = zeroed_feed();
        let max_age = 7200u64;

        // Index 0: zero supply
        push_obs(&mut feed, 1_000_000, 0, 1000);
        // Index 1: stale (age = 8000 > 7200)
        push_obs(&mut feed, 1_000_000, 1_000, 2000);
        // Index 2: zero supply
        push_obs(&mut feed, 1_000_000, 0, 5000);
        // Index 3: valid (age = 3000 < 7200, nonzero supply)
        push_obs(&mut feed, 1_030_000, 1_000, 7000);
        // Index 4: newest
        push_obs(&mut feed, 1_050_000, 1_000, 10000);

        let result = find_best_oldest_observation(
            &feed.observations,
            feed.count as usize,
            feed.head as usize,
            10000,
            max_age,
        );

        assert_eq!(result, Some(3));
    }
}
