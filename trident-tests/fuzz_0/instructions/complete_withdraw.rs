use crate::fuzz_accounts::FuzzAccounts;
use crate::types::*;
use borsh::{BorshDeserialize, BorshSerialize};
use trident_fuzz::fuzzing::*;

#[derive(TridentInstruction, Default)]
#[program_id("EvVxBE31HKE3QdRZXZB7gkkPTcMcAPKGTWJKAvoGuZoX")]
#[discriminator([172u8, 129u8, 141u8, 17u8, 95u8, 253u8, 251u8, 98u8])]
pub struct CompleteWithdrawInstruction {
    pub accounts: CompleteWithdrawInstructionAccounts,
    pub data: CompleteWithdrawInstructionData,
}

/// Instruction Accounts
#[derive(Debug, Clone, TridentAccounts, Default)]
#[instruction_data(CompleteWithdrawInstructionData)]
#[storage(FuzzAccounts)]
pub struct CompleteWithdrawInstructionAccounts {
    #[account(mut, signer)]
    pub user: TridentAccount,

    #[account(mut)]
    pub tranche_config: TridentAccount,

    pub tranche_authority: TridentAccount,

    #[account(mut)]
    pub senior_mint: TridentAccount,

    #[account(mut)]
    pub junior_mint: TridentAccount,

    #[account(mut)]
    pub tranche_token_escrow: TridentAccount,

    #[account(mut)]
    pub asset_escrow: TridentAccount,

    #[account(mut)]
    pub user_asset_ata: TridentAccount,

    #[account(mut)]
    pub vault_lp_custody: TridentAccount,

    pub asset_mint: TridentAccount,

    #[account(mut)]
    pub vault: TridentAccount,

    #[account(mut)]
    pub vault_lp_mint: TridentAccount,

    pub protocol: TridentAccount,

    #[account(mut)]
    pub vault_asset_idle_ata: TridentAccount,

    #[account(mut)]
    pub vault_asset_idle_auth: TridentAccount,

    #[account(mut)]
    pub withdraw_receipt: TridentAccount,

    #[account(address = "vVoLTRjQmtFpiYoegx285Ze4gsLJ8ZxgFKVcuvmG1a8")]
    pub voltr_vault_program: TridentAccount,

    #[account(address = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA")]
    pub token_program: TridentAccount,

    #[account(address = "11111111111111111111111111111111")]
    pub system_program: TridentAccount,
}

/// Instruction Data
#[derive(Debug, BorshDeserialize, BorshSerialize, Clone, Default)]
pub struct CompleteWithdrawInstructionData {}

/// Implementation of instruction setters for fuzzing
///
/// Provides methods to:
/// - Set instruction data during fuzzing
/// - Configure instruction accounts during fuzzing
/// - (Optional) Set remaining accounts during fuzzing
///
/// Docs: https://ackee.xyz/trident/docs/latest/start-fuzzing/writting-fuzz-test/
impl InstructionHooks for CompleteWithdrawInstruction {
    type IxAccounts = FuzzAccounts;
}
