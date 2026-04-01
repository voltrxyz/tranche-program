use borsh::{BorshDeserialize, BorshSerialize};
use trident_fuzz::fuzzing::*;

/// File containing all custom types which can be used
/// in transactions and instructions or invariant checks.
///
/// You can define your own custom types here.

#[derive(Debug, BorshDeserialize, BorshSerialize, Clone)]
pub struct AprFeed {
    pub head: u16,

    pub count: u16,

    pub _padding: [u8; 4],

    pub observations: [AprObservation; 72],
}
impl Default for AprFeed {
    fn default() -> Self {
        Self {
            head: 0,
            count: 0,
            _padding: [0; 4],
            observations: core::array::from_fn(|_| AprObservation::default()),
        }
    }
}

#[derive(Debug, BorshDeserialize, BorshSerialize, Clone, Default)]
pub struct AprObservation {
    pub vault_nav: [u8; 16],

    pub vault_lp_supply: [u8; 16],

    pub timestamp: i64,

    pub _padding: [u8; 8],
}

#[derive(Debug, BorshDeserialize, BorshSerialize, Clone, Default)]
pub struct CancelWithdrawEvent {
    pub user: TridentPubkey,

    pub receipt_id: u64,

    pub tokens_returned: u64,
}

#[derive(Debug, BorshDeserialize, BorshSerialize, Clone, Default)]
pub struct CompleteWithdrawEvent {
    pub user: TridentPubkey,

    pub receipt_id: u64,

    pub is_senior: bool,

    pub final_value: u64,

    pub tokens_burned: u64,
}

#[derive(Debug, BorshDeserialize, BorshSerialize, Clone, Default)]
pub struct CrankAccountingEvent {
    pub delta: i128,

    pub senior_nav: u128,

    pub junior_nav: u128,

    pub reserve_nav: u128,

    pub total_nav: u128,

    pub base_apr: u16,

    pub coverage_ratio: u128,

    pub slot: u64,
}

#[derive(Debug, BorshDeserialize, BorshSerialize, Clone, Default)]
pub struct DepositJuniorEvent {
    pub user: TridentPubkey,

    pub asset_amount: u64,

    pub tokens_minted: u64,

    pub junior_nav: u128,

    pub coverage_ratio: u128,
}

#[derive(Debug, BorshDeserialize, BorshSerialize, Clone, Default)]
pub struct DepositSeniorEvent {
    pub user: TridentPubkey,

    pub asset_amount: u64,

    pub tokens_minted: u64,

    pub senior_nav: u128,

    pub coverage_ratio: u128,
}

#[derive(Debug, BorshDeserialize, BorshSerialize, Clone, Default)]
pub struct FeeConfiguration {
    pub manager_performance_fee: u16,

    pub admin_performance_fee: u16,

    pub manager_management_fee: u16,

    pub admin_management_fee: u16,

    pub redemption_fee: u16,

    pub issuance_fee: u16,

    pub protocol_performance_fee: u16,

    pub protocol_management_fee: u16,

    pub reserved: [u8; 32],
}

#[derive(Debug, BorshDeserialize, BorshSerialize, Clone, Default)]
pub struct FeeState {
    pub accumulated_lp_manager_fees: u64,

    pub accumulated_lp_admin_fees: u64,

    pub accumulated_lp_protocol_fees: u64,

    pub reserved: [u8; 16],
}

#[derive(Debug, BorshDeserialize, BorshSerialize, Clone, Default)]
pub struct FeeUpdate {
    pub last_performance_fee_update_ts: u64,

    pub last_management_fee_update_ts: u64,
}

#[derive(Debug, BorshDeserialize, BorshSerialize, Clone, Default)]
pub struct HighWaterMark {
    pub highest_asset_per_lp_decimal_bits: u128,

    pub last_updated_ts: u64,

    pub reserved: [u8; 8],
}

#[derive(Debug, BorshDeserialize, BorshSerialize, Clone, Default)]
pub struct InitializeTrancheConfigParams {
    pub apr_floor_bps: u16,

    pub risk_x_bps: u16,

    pub risk_y_bps: u16,

    pub risk_k: [u8; 16],

    pub reserve_fee_bps: u16,

    pub senior_exit_fee_bps: u16,

    pub junior_exit_fee_bps: u16,

    pub min_jr_sr_ratio_bps: u16,

    pub min_coverage_for_mint_bps: u16,

    pub min_coverage_buffer_bps: u16,

    pub senior_cooldown_seconds: u64,

    pub junior_cooldown_seconds: u64,

    pub max_observation_age_seconds: u64,
}

#[derive(Debug, BorshDeserialize, BorshSerialize, Clone, Default)]
pub struct InstantWithdrawEvent {
    pub user: TridentPubkey,

    pub is_senior: bool,

    pub asset_amount: u64,

    pub tokens_burned: u64,
}

#[derive(Debug, BorshDeserialize, BorshSerialize, Clone, Default)]
pub struct LockedProfitState {
    pub last_updated_locked_profit: u64,

    pub last_report: u64,
}

#[derive(Debug, BorshDeserialize, BorshSerialize, Clone, Default)]
pub struct RequestWithdrawEvent {
    pub user: TridentPubkey,

    pub receipt_id: u64,

    pub is_senior: bool,

    pub tokens_escrowed: u64,

    pub asset_value: u128,

    pub timestamp: i64,
}

#[derive(Debug, BorshDeserialize, BorshSerialize, Clone)]
pub struct TrancheConfig {
    pub vault: TridentPubkey,

    pub authority: TridentPubkey,

    pub pending_authority: TridentPubkey,

    pub senior_mint: TridentPubkey,

    pub junior_mint: TridentPubkey,

    pub lp_custody: TridentPubkey,

    pub asset_mint: TridentPubkey,

    pub total_nav: [u8; 16],

    pub senior_nav: [u8; 16],

    pub junior_nav: [u8; 16],

    pub reserve_nav: [u8; 16],

    pub senior_target_index: [u8; 16],

    pub index_timestamp: i64,

    pub apr_floor_bps: u16,

    pub risk_x_bps: u16,

    pub risk_y_bps: u16,

    pub _padding0: [u8; 2],

    pub risk_k: [u8; 16],

    pub base_apr_bps: u16,

    pub reserve_fee_bps: u16,

    pub senior_exit_fee_bps: u16,

    pub junior_exit_fee_bps: u16,

    pub min_jr_sr_ratio_bps: u16,

    pub min_coverage_for_mint_bps: u16,

    pub min_coverage_buffer_bps: u16,

    pub _padding1: [u8; 2],

    pub senior_cooldown_seconds: u64,

    pub junior_cooldown_seconds: u64,

    pub max_observation_age_seconds: u64,

    pub paused: u8,

    pub _padding2: [u8; 7],

    pub next_receipt_id: u64,

    pub tranche_config_bump: u8,

    pub tranche_authority_bump: u8,

    pub senior_mint_bump: u8,

    pub junior_mint_bump: u8,

    pub apr_feed_bump: u8,

    pub _padding3: [u8; 3],

    pub _reserved: [u8; 248],
}
impl Default for TrancheConfig {
    fn default() -> Self {
        Self {
            vault: TridentPubkey::default(),
            authority: TridentPubkey::default(),
            pending_authority: TridentPubkey::default(),
            senior_mint: TridentPubkey::default(),
            junior_mint: TridentPubkey::default(),
            lp_custody: TridentPubkey::default(),
            asset_mint: TridentPubkey::default(),
            total_nav: [0; 16],
            senior_nav: [0; 16],
            junior_nav: [0; 16],
            reserve_nav: [0; 16],
            senior_target_index: [0; 16],
            index_timestamp: 0,
            apr_floor_bps: 0,
            risk_x_bps: 0,
            risk_y_bps: 0,
            _padding0: [0; 2],
            risk_k: [0; 16],
            base_apr_bps: 0,
            reserve_fee_bps: 0,
            senior_exit_fee_bps: 0,
            junior_exit_fee_bps: 0,
            min_jr_sr_ratio_bps: 0,
            min_coverage_for_mint_bps: 0,
            min_coverage_buffer_bps: 0,
            _padding1: [0; 2],
            senior_cooldown_seconds: 0,
            junior_cooldown_seconds: 0,
            max_observation_age_seconds: 0,
            paused: 0,
            _padding2: [0; 7],
            next_receipt_id: 0,
            tranche_config_bump: 0,
            tranche_authority_bump: 0,
            senior_mint_bump: 0,
            junior_mint_bump: 0,
            apr_feed_bump: 0,
            _padding3: [0; 3],
            _reserved: [0; 248],
        }
    }
}

#[derive(Debug, BorshDeserialize, BorshSerialize, Clone)]
pub enum TrancheConfigField {
    AprFloorBps(u16),

    RiskXBps(u16),

    RiskYBps(u16),

    RiskK([u8; 16]),

    ReserveFeeBps(u16),

    SeniorExitFeeBps(u16),

    JuniorExitFeeBps(u16),

    MinJrSrRatioBps(u16),

    MinCoverageForMintBps(u16),

    MinCoverageBufferBps(u16),

    SeniorCooldownSeconds(u64),

    JuniorCooldownSeconds(u64),
}
impl Default for TrancheConfigField {
    fn default() -> Self {
        Self::AprFloorBps(0)
    }
}

#[derive(Debug, BorshDeserialize, BorshSerialize, Clone, Default)]
pub struct UpdateTrancheConfigEvent {
    pub authority: TridentPubkey,

    pub field_index: u8,

    pub slot: u64,
}

#[derive(Debug, BorshDeserialize, BorshSerialize, Clone)]
pub struct Vault {
    pub name: [u8; 32],

    pub description: [u8; 64],

    pub asset: VaultAsset,

    pub lp: VaultLp,

    pub pending_admin: TridentPubkey,

    pub manager: TridentPubkey,

    pub admin: TridentPubkey,

    pub vault_configuration: VaultConfiguration,

    pub fee_configuration: FeeConfiguration,

    pub fee_update: FeeUpdate,

    pub fee_state: FeeState,

    pub dead_weight: u64,

    pub high_water_mark: HighWaterMark,

    pub last_updated_ts: u64,

    pub version: u8,

    pub _padding0: [u8; 7],

    pub locked_profit_state: LockedProfitState,

    pub reserved: [u8; 240],
}
impl Default for Vault {
    fn default() -> Self {
        Self {
            name: [0; 32],
            description: [0; 64],
            asset: VaultAsset::default(),
            lp: VaultLp::default(),
            pending_admin: TridentPubkey::default(),
            manager: TridentPubkey::default(),
            admin: TridentPubkey::default(),
            vault_configuration: VaultConfiguration::default(),
            fee_configuration: FeeConfiguration::default(),
            fee_update: FeeUpdate::default(),
            fee_state: FeeState::default(),
            dead_weight: 0,
            high_water_mark: HighWaterMark::default(),
            last_updated_ts: 0,
            version: 0,
            _padding0: [0; 7],
            locked_profit_state: LockedProfitState::default(),
            reserved: [0; 240],
        }
    }
}

#[derive(Debug, BorshDeserialize, BorshSerialize, Clone)]
pub struct VaultAsset {
    pub mint: TridentPubkey,

    pub idle_ata: TridentPubkey,

    pub total_value: u64,

    pub idle_ata_auth_bump: u8,

    pub reserved: [u8; 95],
}
impl Default for VaultAsset {
    fn default() -> Self {
        Self {
            mint: TridentPubkey::default(),
            idle_ata: TridentPubkey::default(),
            total_value: 0,
            idle_ata_auth_bump: 0,
            reserved: [0; 95],
        }
    }
}

#[derive(Debug, BorshDeserialize, BorshSerialize, Clone)]
pub struct VaultConfiguration {
    pub max_cap: u64,

    pub start_at_ts: u64,

    pub locked_profit_degradation_duration: u64,

    pub withdrawal_waiting_period: u64,

    pub disabled_operations: u16,

    pub reserved: [u8; 46],
}
impl Default for VaultConfiguration {
    fn default() -> Self {
        Self {
            max_cap: 0,
            start_at_ts: 0,
            locked_profit_degradation_duration: 0,
            withdrawal_waiting_period: 0,
            disabled_operations: 0,
            reserved: [0; 46],
        }
    }
}

#[derive(Debug, BorshDeserialize, BorshSerialize, Clone, Default)]
pub struct VaultLp {
    pub mint: TridentPubkey,

    pub mint_bump: u8,

    pub mint_auth_bump: u8,

    pub reserved: [u8; 30],
}

#[derive(Debug, BorshDeserialize, BorshSerialize, Clone, Default)]
pub struct WithdrawReceipt {
    pub tranche_config: TridentPubkey,

    pub user: TridentPubkey,

    pub receipt_id: u64,

    pub is_senior: bool,

    pub tranche_tokens_escrowed: u64,

    pub asset_amount_at_request: u128,

    pub withdrawable_from_ts: i64,

    pub bump: u8,
}

#[derive(Debug, BorshDeserialize, BorshSerialize, Clone, Default)]
pub struct WithdrawReserveEvent {
    pub authority: TridentPubkey,

    pub amount_withdrawn: u64,

    pub reserve_nav_after: u128,

    pub total_nav_after: u128,
}
