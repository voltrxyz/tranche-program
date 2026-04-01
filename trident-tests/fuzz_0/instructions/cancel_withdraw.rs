use crate::fuzz_accounts::FuzzAccounts;
use crate::types::*;
use borsh::{BorshDeserialize, BorshSerialize};
use trident_fuzz::fuzzing::*;

#[derive(TridentInstruction, Default)]
#[program_id("EvVxBE31HKE3QdRZXZB7gkkPTcMcAPKGTWJKAvoGuZoX")]
#[discriminator([112u8, 53u8, 226u8, 58u8, 158u8, 30u8, 37u8, 168u8])]
pub struct CancelWithdrawInstruction {
    pub accounts: CancelWithdrawInstructionAccounts,
    pub data: CancelWithdrawInstructionData,
}

/// Instruction Accounts
#[derive(Debug, Clone, TridentAccounts, Default)]
#[instruction_data(CancelWithdrawInstructionData)]
#[storage(FuzzAccounts)]
pub struct CancelWithdrawInstructionAccounts {
    #[account(mut, signer)]
    pub user: TridentAccount,

    pub tranche_config: TridentAccount,

    pub tranche_authority: TridentAccount,

    #[account(mut)]
    pub withdraw_receipt: TridentAccount,

    pub tranche_mint: TridentAccount,

    #[account(mut)]
    pub tranche_token_escrow: TridentAccount,

    #[account(mut)]
    pub user_tranche_ata: TridentAccount,

    #[account(address = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA")]
    pub token_program: TridentAccount,
}

/// Instruction Data
#[derive(Debug, BorshDeserialize, BorshSerialize, Clone, Default)]
pub struct CancelWithdrawInstructionData {}

/// Implementation of instruction setters for fuzzing
///
/// Provides methods to:
/// - Set instruction data during fuzzing
/// - Configure instruction accounts during fuzzing
/// - (Optional) Set remaining accounts during fuzzing
///
/// Docs: https://ackee.xyz/trident/docs/latest/start-fuzzing/writting-fuzz-test/
impl InstructionHooks for CancelWithdrawInstruction {
    type IxAccounts = FuzzAccounts;
}
