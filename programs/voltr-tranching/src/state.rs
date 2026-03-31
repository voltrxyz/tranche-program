use anchor_lang::prelude::*;

/// Maximum number of APR observations in the ring buffer (~3 days at hourly intervals).
pub const MAX_APR_OBSERVATIONS: usize = 72;

// ─── TrancheConfig (zero-copy) ────────────────────────────────────────────────
//
// PDA: [b"tranche_config", vault.key()]
//
// All `u128` NAV and index fields store `Decimal` (U80F48) values via
// `to_decimal_bits()`. Reconstruct with `Decimal::from_decimal_bits(field)`
// before arithmetic.

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
    pub total_nav: u128,
    /// Senior tranche NAV.
    pub senior_nav: u128,
    /// Junior tranche NAV.
    pub junior_nav: u128,
    /// Protocol reserve NAV (accumulated performance fees).
    pub reserve_nav: u128,

    // ── Senior Target ─────────────────────────────────────────────
    /// Compound growth index (Decimal bits, init: `Decimal::ONE`).
    pub senior_target_index: u128,
    /// Last index update timestamp.
    pub index_timestamp: i64,

    // ── APR Parameters ────────────────────────────────────────────
    /// Minimum senior APR in basis points.
    pub apr_floor_bps: u16,
    /// Risk premium constant X in basis points.
    pub risk_x_bps: u16,
    /// Risk premium constant Y in basis points.
    pub risk_y_bps: u16,
    /// Padding to align `risk_k` to 8-byte boundary.
    pub _padding0: [u8; 2],
    /// Risk premium exponent K (Decimal bits).
    pub risk_k: u128,
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
    /// Padding to align next field to 8-byte boundary.
    pub _padding1: [u8; 2],

    // ── Withdrawal Config ─────────────────────────────────────────
    /// Senior withdrawal cooldown in seconds.
    pub senior_cooldown_seconds: u64,
    /// Junior withdrawal cooldown in seconds.
    pub junior_cooldown_seconds: u64,

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
    pub _reserved: [u8; 256],
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
    pub const INIT_SPACE: usize =
        32 +    // tranche_config (Pubkey)
        32 +    // user (Pubkey)
        8 +     // receipt_id (u64)
        1 +     // is_senior (bool)
        8 +     // tranche_tokens_escrowed (u64)
        16 +    // asset_amount_at_request (u128)
        8 +     // withdrawable_from_ts (i64)
        1;      // bump (u8)
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
    pub vault_nav: u128,
    /// Vault LP token supply at observation time (Decimal bits).
    pub vault_lp_supply: u128,
    /// Unix timestamp of the observation.
    pub timestamp: i64,
    /// Padding to maintain 16-byte alignment.
    pub _padding: [u8; 8],
}

// ─── PDA Seed Constants ──────────────────────────────────────────────────────

pub const TRANCHE_CONFIG_SEED: &[u8] = b"tranche_config";
pub const TRANCHE_AUTHORITY_SEED: &[u8] = b"tranche_authority";
pub const SENIOR_MINT_SEED: &[u8] = b"senior_mint";
pub const JUNIOR_MINT_SEED: &[u8] = b"junior_mint";
pub const WITHDRAW_RECEIPT_SEED: &[u8] = b"withdraw_receipt";
pub const APR_FEED_SEED: &[u8] = b"apr_feed";

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem;

    // ─── Layout & Size Tests ──────────────────────────────────────

    #[test]
    fn tranche_config_is_repr_c() {
        // Ensure the struct compiles and has a deterministic size.
        let size = mem::size_of::<TrancheConfig>();
        // 7 Pubkeys (224) + 4 u128 NAVs (64) + senior_target_index u128 (16) +
        // index_timestamp i64 (8) + APR bps fields (6) + _padding0 (2) +
        // risk_k u128 (16) + base_apr_bps u16 (2) + fee bps (6) +
        // coverage bps (6) + _padding1 (2) + cooldowns (16) +
        // paused u8 (1) + _padding2 (7) + next_receipt_id u64 (8) +
        // bumps (5) + _padding3 (3) + _reserved (256)
        assert!(size > 0, "TrancheConfig has non-zero size");
        // Ensure 8-byte alignment
        assert_eq!(size % 8, 0, "TrancheConfig should be 8-byte aligned");
    }

    #[test]
    fn withdraw_receipt_init_space_matches() {
        // INIT_SPACE should match the sum of serialized Borsh fields.
        // For standard Anchor accounts, Borsh serialization is used.
        assert_eq!(WithdrawReceipt::INIT_SPACE, 106);
    }

    #[test]
    fn apr_feed_is_repr_c() {
        let size = mem::size_of::<AprFeed>();
        assert!(size > 0, "AprFeed has non-zero size");
        assert_eq!(size % 8, 0, "AprFeed should be 8-byte aligned");
    }

    #[test]
    fn apr_observation_size() {
        // u128 (16) + u128 (16) + i64 (8) + _padding (8) = 48 bytes
        assert_eq!(mem::size_of::<AprObservation>(), 48);
    }

    #[test]
    fn apr_feed_observation_capacity() {
        // 72 observations * 48 bytes = 3456 bytes for the array
        let obs_size = mem::size_of::<AprObservation>() * MAX_APR_OBSERVATIONS;
        assert_eq!(obs_size, 3456);
    }

    // ─── PDA Seed Uniqueness ──────────────────────────────────────

    #[test]
    fn pda_seeds_are_distinct() {
        let seeds: Vec<&[u8]> = vec![
            TRANCHE_CONFIG_SEED,
            TRANCHE_AUTHORITY_SEED,
            SENIOR_MINT_SEED,
            JUNIOR_MINT_SEED,
            WITHDRAW_RECEIPT_SEED,
            APR_FEED_SEED,
        ];
        for (i, a) in seeds.iter().enumerate() {
            for (j, b) in seeds.iter().enumerate() {
                if i != j {
                    assert_ne!(a, b, "Seeds at index {} and {} collide", i, j);
                }
            }
        }
    }

    // ─── Reserved Space ───────────────────────────────────────────

    #[test]
    fn tranche_config_has_reserved_space() {
        // The _reserved field should be exactly 256 bytes.
        let config = unsafe { mem::zeroed::<TrancheConfig>() };
        assert_eq!(config._reserved.len(), 256);
    }

    // ─── Zero Initialization ──────────────────────────────────────

    #[test]
    fn tranche_config_zeroed_is_valid() {
        // Zero-copy accounts start zeroed; ensure this doesn't panic.
        let config = unsafe { mem::zeroed::<TrancheConfig>() };
        assert_eq!(config.paused, 0);
        assert_eq!(config.next_receipt_id, 0);
        assert_eq!(config.total_nav, 0);
        assert_eq!(config.senior_nav, 0);
        assert_eq!(config.junior_nav, 0);
        assert_eq!(config.reserve_nav, 0);
    }

    #[test]
    fn apr_feed_zeroed_is_valid() {
        let feed = unsafe { mem::zeroed::<AprFeed>() };
        assert_eq!(feed.head, 0);
        assert_eq!(feed.count, 0);
        for obs in &feed.observations {
            assert_eq!(obs.vault_nav, 0);
            assert_eq!(obs.vault_lp_supply, 0);
            assert_eq!(obs.timestamp, 0);
        }
    }
}
