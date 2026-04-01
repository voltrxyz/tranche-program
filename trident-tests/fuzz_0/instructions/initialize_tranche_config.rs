use crate::fuzz_accounts::FuzzAccounts;
use crate::types::*;
use borsh::{BorshDeserialize, BorshSerialize};
use trident_fuzz::fuzzing::*;

#[derive(TridentInstruction, Default)]
#[program_id("EvVxBE31HKE3QdRZXZB7gkkPTcMcAPKGTWJKAvoGuZoX")]
#[discriminator([58u8, 73u8, 224u8, 50u8, 220u8, 233u8, 56u8, 147u8])]
pub struct InitializeTrancheConfigInstruction {
    pub accounts: InitializeTrancheConfigInstructionAccounts,
    pub data: InitializeTrancheConfigInstructionData,
}

/// Instruction Accounts
#[derive(Debug, Clone, TridentAccounts, Default)]
#[instruction_data(InitializeTrancheConfigInstructionData)]
#[storage(FuzzAccounts)]
pub struct InitializeTrancheConfigInstructionAccounts {
    #[account(mut, signer)]
    pub authority: TridentAccount,

    pub vault: TridentAccount,

    #[account(mut)]
    pub tranche_config: TridentAccount,

    pub tranche_authority: TridentAccount,

    #[account(mut)]
    pub senior_mint: TridentAccount,

    #[account(mut)]
    pub junior_mint: TridentAccount,

    pub asset_mint: TridentAccount,

    pub vault_lp_mint: TridentAccount,

    #[account(mut)]
    pub vault_lp_custody: TridentAccount,

    #[account(mut)]
    pub asset_escrow: TridentAccount,

    #[account(mut)]
    pub apr_feed: TridentAccount,

    #[account(address = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA")]
    pub token_program: TridentAccount,

    #[account(address = "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL")]
    pub associated_token_program: TridentAccount,

    #[account(address = "11111111111111111111111111111111")]
    pub system_program: TridentAccount,
}

/// Instruction Data
#[derive(Debug, BorshDeserialize, BorshSerialize, Clone, Default)]
pub struct InitializeTrancheConfigInstructionData {
    pub params: InitializeTrancheConfigParams,
}

/// Implementation of instruction setters for fuzzing
///
/// Provides methods to:
/// - Set instruction data during fuzzing
/// - Configure instruction accounts during fuzzing
/// - (Optional) Set remaining accounts during fuzzing
///
/// Docs: https://ackee.xyz/trident/docs/latest/start-fuzzing/writting-fuzz-test/
impl InstructionHooks for InitializeTrancheConfigInstruction {
    type IxAccounts = FuzzAccounts;
}
