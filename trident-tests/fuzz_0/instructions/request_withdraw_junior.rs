use crate::fuzz_accounts::FuzzAccounts;
use crate::types::*;
use borsh::{BorshDeserialize, BorshSerialize};
use trident_fuzz::fuzzing::*;

#[derive(TridentInstruction, Default)]
#[program_id("EvVxBE31HKE3QdRZXZB7gkkPTcMcAPKGTWJKAvoGuZoX")]
#[discriminator([253u8, 39u8, 216u8, 54u8, 119u8, 138u8, 171u8, 42u8])]
pub struct RequestWithdrawJuniorInstruction {
    pub accounts: RequestWithdrawJuniorInstructionAccounts,
    pub data: RequestWithdrawJuniorInstructionData,
}

/// Instruction Accounts
#[derive(Debug, Clone, TridentAccounts, Default)]
#[instruction_data(RequestWithdrawJuniorInstructionData)]
#[storage(FuzzAccounts)]
pub struct RequestWithdrawJuniorInstructionAccounts {
    #[account(mut, signer)]
    pub user: TridentAccount,

    #[account(mut)]
    pub tranche_config: TridentAccount,

    pub tranche_authority: TridentAccount,

    pub senior_mint: TridentAccount,

    pub junior_mint: TridentAccount,

    #[account(mut)]
    pub user_junior_ata: TridentAccount,

    #[account(mut)]
    pub junior_token_escrow: TridentAccount,

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
pub struct RequestWithdrawJuniorInstructionData {
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
impl InstructionHooks for RequestWithdrawJuniorInstruction {
    type IxAccounts = FuzzAccounts;
}
