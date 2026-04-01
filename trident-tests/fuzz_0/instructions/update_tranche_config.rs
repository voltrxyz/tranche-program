use crate::fuzz_accounts::FuzzAccounts;
use crate::types::*;
use borsh::{BorshDeserialize, BorshSerialize};
use trident_fuzz::fuzzing::*;

#[derive(TridentInstruction, Default)]
#[program_id("EvVxBE31HKE3QdRZXZB7gkkPTcMcAPKGTWJKAvoGuZoX")]
#[discriminator([17u8, 66u8, 9u8, 47u8, 165u8, 108u8, 26u8, 255u8])]
pub struct UpdateTrancheConfigInstruction {
    pub accounts: UpdateTrancheConfigInstructionAccounts,
    pub data: UpdateTrancheConfigInstructionData,
}

/// Instruction Accounts
#[derive(Debug, Clone, TridentAccounts, Default)]
#[instruction_data(UpdateTrancheConfigInstructionData)]
#[storage(FuzzAccounts)]
pub struct UpdateTrancheConfigInstructionAccounts {
    #[account(signer)]
    pub authority: TridentAccount,

    #[account(mut)]
    pub tranche_config: TridentAccount,
}

/// Instruction Data
#[derive(Debug, BorshDeserialize, BorshSerialize, Clone, Default)]
pub struct UpdateTrancheConfigInstructionData {
    pub field: TrancheConfigField,
}

/// Implementation of instruction setters for fuzzing
///
/// Provides methods to:
/// - Set instruction data during fuzzing
/// - Configure instruction accounts during fuzzing
/// - (Optional) Set remaining accounts during fuzzing
///
/// Docs: https://ackee.xyz/trident/docs/latest/start-fuzzing/writting-fuzz-test/
impl InstructionHooks for UpdateTrancheConfigInstruction {
    type IxAccounts = FuzzAccounts;
}
