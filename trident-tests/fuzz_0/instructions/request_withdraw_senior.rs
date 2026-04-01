use crate::fuzz_accounts::FuzzAccounts;
use crate::types::*;
use borsh::{BorshDeserialize, BorshSerialize};
use trident_fuzz::fuzzing::*;

#[derive(TridentInstruction, Default)]
#[program_id("EvVxBE31HKE3QdRZXZB7gkkPTcMcAPKGTWJKAvoGuZoX")]
#[discriminator([223u8, 124u8, 240u8, 197u8, 61u8, 0u8, 209u8, 64u8])]
pub struct RequestWithdrawSeniorInstruction {
    pub accounts: RequestWithdrawSeniorInstructionAccounts,
    pub data: RequestWithdrawSeniorInstructionData,
}

/// Instruction Accounts
#[derive(Debug, Clone, TridentAccounts, Default)]
#[instruction_data(RequestWithdrawSeniorInstructionData)]
#[storage(FuzzAccounts)]
pub struct RequestWithdrawSeniorInstructionAccounts {
    #[account(mut, signer)]
    pub user: TridentAccount,

    #[account(mut)]
    pub tranche_config: TridentAccount,

    pub tranche_authority: TridentAccount,

    pub senior_mint: TridentAccount,

    pub junior_mint: TridentAccount,

    #[account(mut)]
    pub user_senior_ata: TridentAccount,

    #[account(mut)]
    pub senior_token_escrow: TridentAccount,

    pub vault_lp_custody: TridentAccount,

    pub vault: TridentAccount,

    pub vault_lp_mint: TridentAccount,

    #[account(mut)]
    pub withdraw_receipt: TridentAccount,

    #[account(address = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA")]
    pub token_program: TridentAccount,

    #[account(address = "11111111111111111111111111111111")]
    pub system_program: TridentAccount,
}

/// Instruction Data
#[derive(Debug, BorshDeserialize, BorshSerialize, Clone, Default)]
pub struct RequestWithdrawSeniorInstructionData {
    pub amount: u64,

    pub is_token_amount: bool,
}

/// Implementation of instruction setters for fuzzing
///
/// Provides methods to:
/// - Set instruction data during fuzzing
/// - Configure instruction accounts during fuzzing
/// - (Optional) Set remaining accounts during fuzzing
///
/// Docs: https://ackee.xyz/trident/docs/latest/start-fuzzing/writting-fuzz-test/
impl InstructionHooks for RequestWithdrawSeniorInstruction {
    type IxAccounts = FuzzAccounts;
}
