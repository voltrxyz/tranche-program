use anchor_lang::prelude::*;

/// Little-endian bytes of `Decimal::to_decimal_bits()`.
///
/// Zero-copy accounts store Decimal values as bytes instead of `u128` so their
/// alignment stays compatible with Anchor's 8-byte discriminator offset.
pub type DecimalBits = [u8; 16];

/// Maximum number of APR observations in the ring buffer (~3 days at hourly intervals).
pub const MAX_APR_OBSERVATIONS: usize = 72;

// ─── TrancheConfig (zero-copy) ────────────────────────────────────────────────
//
// PDA: [b"tranche_config", vault.key()]
//
// All Decimal fields in zero-copy accounts store the little-endian bytes of
// `Decimal::to_decimal_bits()`. Reconstruct them with
// `Decimal::from_decimal_bits(u128::from_le_bytes(field))` before arithmetic.

#[account(zero_copy(unsafe))]
#[repr(C)]
#[derive(Debug, PartialEq, Eq)]
pub struct TrancheConfig {
    // ── Identity ──────────────────────────────────────────────────
    /// Underlying Voltr Vault address.
    pub vault: Pubkey,
    /// Admin authority.
    pub authority: Pubkey,
    /// Pending authority for 2-step transfer.
    pub pending_authority: Pubkey,

    // ── Tranche Mints ─────────────────────────────────────────────
    /// Senior tranche token mint.
    pub senior_mint: Pubkey,
    /// Junior tranche token mint.
    pub junior_mint: Pubkey,

    // ── Voltr Integration ─────────────────────────────────────────
    /// ATA holding vault LP tokens (custody).
    pub lp_custody: Pubkey,
    /// Underlying asset mint (e.g., USDC).
    pub asset_mint: Pubkey,

    // ── NAV Tracking (Decimal bits via `to_decimal_bits()`) ──────
    /// Total net asset value.
    pub total_nav: DecimalBits,
    /// Senior tranche NAV.
    pub senior_nav: DecimalBits,
    /// Junior tranche NAV.
    pub junior_nav: DecimalBits,
    /// Protocol reserve NAV (accumulated performance fees).
    pub reserve_nav: DecimalBits,

    // ── Senior Target ─────────────────────────────────────────────
    /// Compound growth index (Decimal bits, init: `Decimal::ONE`).
    pub senior_target_index: DecimalBits,
    /// Last index update timestamp.
    pub index_timestamp: i64,

    // ── APR Parameters ────────────────────────────────────────────
    /// Minimum senior APR in basis points.
    pub apr_floor_bps: u16,
    /// Risk premium constant X in basis points.
    pub risk_x_bps: u16,
    /// Risk premium constant Y in basis points.
    pub risk_y_bps: u16,
    /// Explicit padding for stable layout.
    pub _padding0: [u8; 2],
    /// Risk premium exponent K (Decimal bits).
    pub risk_k: DecimalBits,
    /// Current vault base APR in basis points.
    pub base_apr_bps: u16,

    // ── Fee Parameters ────────────────────────────────────────────
    /// Performance fee in basis points (max 2000 = 20%).
    pub reserve_fee_bps: u16,
    /// Senior withdrawal fee in basis points (max 100 = 1%).
    pub senior_exit_fee_bps: u16,
    /// Junior withdrawal fee in basis points (max 100 = 1%).
    pub junior_exit_fee_bps: u16,

    // ── Coverage Thresholds ───────────────────────────────────────
    /// Minimum junior/senior ratio in basis points.
    pub min_jr_sr_ratio_bps: u16,
    /// Minimum coverage ratio to allow senior deposits in basis points.
    pub min_coverage_for_mint_bps: u16,
    /// Coverage buffer for senior minting in basis points.
    pub min_coverage_buffer_bps: u16,
    /// Explicit padding for stable layout.
    pub _padding1: [u8; 2],

    // ── Withdrawal Config ─────────────────────────────────────────
    /// Senior withdrawal cooldown in seconds.
    pub senior_cooldown_seconds: u64,
    /// Junior withdrawal cooldown in seconds.
    pub junior_cooldown_seconds: u64,

    // ── APR Feed Config ─────────────────────────────────────────
    /// Maximum age (seconds) of the oldest observation before the feed is
    /// considered stale. When stale, `push_apr_observation` preserves the
    /// last known `base_apr_bps` instead of annualizing over a too-old window.
    pub max_observation_age_seconds: u64,

    // ── Operational ───────────────────────────────────────────────
    /// Bitflag pause state (0x01=senior deposits, 0x02=junior deposits,
    /// 0x04=senior withdrawals, 0x08=junior withdrawals).
    pub paused: u8,
    /// Padding to align `next_receipt_id` to 8-byte boundary.
    pub _padding2: [u8; 7],
    /// Auto-incrementing receipt counter.
    pub next_receipt_id: u64,

    // ── PDA Bumps ─────────────────────────────────────────────────
    /// Bump for `tranche_config` PDA.
    pub tranche_config_bump: u8,
    /// Bump for `tranche_authority` PDA (CPI signer).
    pub tranche_authority_bump: u8,
    /// Bump for `senior_mint` PDA.
    pub senior_mint_bump: u8,
    /// Bump for `junior_mint` PDA.
    pub junior_mint_bump: u8,
    /// Bump for `apr_feed` PDA.
    pub apr_feed_bump: u8,
    /// Padding after bumps.
    pub _padding3: [u8; 3],

    // ── Reserved ──────────────────────────────────────────────────
    /// Reserved space for future expansion.
    pub _reserved: [u8; 248],
}

// ─── WithdrawReceipt (standard account) ───────────────────────────────────────
//
// PDA: [b"withdraw_receipt", tranche_config.key(), receipt_id.to_le_bytes()]

#[account]
#[derive(Debug, PartialEq, Eq)]
pub struct WithdrawReceipt {
    /// Parent TrancheConfig.
    pub tranche_config: Pubkey,
    /// Withdrawing user.
    pub user: Pubkey,
    /// Unique, auto-incremented receipt ID.
    pub receipt_id: u64,
    /// `true` for senior tranche, `false` for junior.
    pub is_senior: bool,
    /// Tranche tokens held in escrow.
    pub tranche_tokens_escrowed: u64,
    /// NAV snapshot at request time (Decimal bits) — value cap for cooldown gaming prevention.
    pub asset_amount_at_request: u128,
    /// Cooldown expiry timestamp.
    pub withdrawable_from_ts: i64,
    /// PDA bump.
    pub bump: u8,
}

impl WithdrawReceipt {
    /// Fixed account size (excluding 8-byte Anchor discriminator).
    pub const INIT_SPACE: usize = 32 +    // tranche_config (Pubkey)
        32 +    // user (Pubkey)
        8 +     // receipt_id (u64)
        1 +     // is_senior (bool)
        8 +     // tranche_tokens_escrowed (u64)
        16 +    // asset_amount_at_request (u128)
        8 +     // withdrawable_from_ts (i64)
        1; // bump (u8)
}

// ─── AprFeed (zero-copy) ─────────────────────────────────────────────────────
//
// PDA: [b"apr_feed", tranche_config.key()]
//
// Ring buffer of 72 observations (~3 days at hourly intervals).
// `head` points to the next write position; `count` tracks how many are filled.

#[account(zero_copy(unsafe))]
#[repr(C)]
#[derive(Debug, PartialEq, Eq)]
pub struct AprFeed {
    /// Ring buffer head pointer (next write index).
    pub head: u16,
    /// Number of observations stored (max `MAX_APR_OBSERVATIONS`).
    pub count: u16,
    /// Padding to align observations to 8-byte boundary.
    pub _padding: [u8; 4],
    /// Circular buffer of APR observations.
    pub observations: [AprObservation; MAX_APR_OBSERVATIONS],
}

/// A single APR observation snapshot.
#[account(zero_copy(unsafe))]
#[repr(C)]
#[derive(Debug, Default, PartialEq, Eq)]
pub struct AprObservation {
    /// Vault NAV at observation time (Decimal bits).
    pub vault_nav: DecimalBits,
    /// Vault LP token supply at observation time (Decimal bits).
    pub vault_lp_supply: DecimalBits,
    /// Unix timestamp of the observation.
    pub timestamp: i64,
    /// Reserved padding for future expansion and stable 48-byte slots.
    pub _padding: [u8; 8],
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem;

    // ─── Layout & Size Tests ──────────────────────────────────────

    #[test]
    fn tranche_config_is_repr_c() {
        // 7 Pubkeys (224) + 6 DecimalBits fields (96) + index_timestamp (8) +
        // APR bps fields (6) + _padding0 (2) + base_apr_bps (2) + fee bps (6) +
        // coverage bps (6) + _padding1 (2) + cooldowns (16) +
        // max_observation_age_seconds (8) + paused (1) +
        // _padding2 (7) + next_receipt_id (8) + bumps (5) + _padding3 (3) +
        // _reserved (248)
        assert_eq!(mem::size_of::<TrancheConfig>(), 648);
        assert_eq!(mem::align_of::<TrancheConfig>(), 8);
    }

    #[test]
    fn withdraw_receipt_init_space_matches() {
        // INIT_SPACE should match the sum of serialized Borsh fields.
        // For standard Anchor accounts, Borsh serialization is used.
        assert_eq!(WithdrawReceipt::INIT_SPACE, 106);
    }

    #[test]
    fn apr_feed_is_repr_c() {
        assert_eq!(mem::size_of::<AprFeed>(), 3464);
        assert_eq!(mem::align_of::<AprFeed>(), 8);
    }

    #[test]
    fn apr_observation_size() {
        // DecimalBits (16) + DecimalBits (16) + i64 (8) + _padding (8) = 48 bytes
        assert_eq!(mem::size_of::<AprObservation>(), 48);
        assert_eq!(mem::align_of::<AprObservation>(), 8);
    }

    #[test]
    fn apr_feed_observation_capacity() {
        // 72 observations * 48 bytes = 3456 bytes for the array
        let obs_size = mem::size_of::<AprObservation>() * MAX_APR_OBSERVATIONS;
        assert_eq!(obs_size, 3456);
    }

    // ─── Reserved Space ───────────────────────────────────────────

    #[test]
    fn tranche_config_has_reserved_space() {
        // The _reserved field should be exactly 256 bytes.
        let config = unsafe { mem::zeroed::<TrancheConfig>() };
        assert_eq!(config._reserved.len(), 248);
    }

    // ─── Zero Initialization ──────────────────────────────────────

    #[test]
    fn tranche_config_zeroed_is_valid() {
        // Zero-copy accounts start zeroed; ensure this doesn't panic.
        let config = unsafe { mem::zeroed::<TrancheConfig>() };
        assert_eq!(config.paused, 0);
        assert_eq!(config.next_receipt_id, 0);
        assert_eq!(config.total_nav, [0; 16]);
        assert_eq!(config.senior_nav, [0; 16]);
        assert_eq!(config.junior_nav, [0; 16]);
        assert_eq!(config.reserve_nav, [0; 16]);
    }

    #[test]
    fn apr_feed_zeroed_is_valid() {
        let feed = unsafe { mem::zeroed::<AprFeed>() };
        assert_eq!(feed.head, 0);
        assert_eq!(feed.count, 0);
        for obs in &feed.observations {
            assert_eq!(obs.vault_nav, [0; 16]);
            assert_eq!(obs.vault_lp_supply, [0; 16]);
            assert_eq!(obs.timestamp, 0);
        }
    }
}
