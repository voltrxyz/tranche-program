use anchor_lang::error_code;

#[error_code]
pub enum TrancheError {
    #[msg("Senior deposits are paused")]
    SeniorDepositsPaused,

    #[msg("Junior deposits are paused")]
    JuniorDepositsPaused,

    #[msg("Senior withdrawals are paused")]
    SeniorWithdrawalsPaused,

    #[msg("Junior withdrawals are paused")]
    JuniorWithdrawalsPaused,

    #[msg("Insufficient coverage ratio for senior deposit")]
    InsufficientCoverage,

    #[msg("Junior withdrawal would breach minimum junior/senior ratio")]
    JuniorRatioBreach,

    #[msg("Withdrawal cooldown period has not elapsed")]
    CooldownNotElapsed,

    #[msg("Amount must be greater than zero")]
    ZeroAmount,

    #[msg("Math overflow in checked arithmetic")]
    MathOverflow,

    #[msg("Caller is not the authority")]
    Unauthorized,

    #[msg("Fee parameter exceeds maximum allowed value")]
    InvalidFee,

    #[msg("Junior tranche shortfall detected, auto-pause triggered")]
    JuniorShortfall,

    #[msg("Vault has non-zero withdrawal waiting period (unsupported in v1)")]
    VaultWaitingPeriodNotZero,

    #[msg("Vault account mismatch or invalid owner")]
    InvalidVault,

    #[msg("Invalid pause flags: only bits 0-3 are valid")]
    InvalidPauseFlags,
}
