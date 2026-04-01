use crate::fuzz_accounts::FuzzAccounts;
use crate::types::*;
use borsh::{BorshDeserialize, BorshSerialize};
use trident_fuzz::fuzzing::*;

#[derive(TridentInstruction, Default)]
#[program_id("EvVxBE31HKE3QdRZXZB7gkkPTcMcAPKGTWJKAvoGuZoX")]
#[discriminator([107u8, 86u8, 198u8, 91u8, 33u8, 12u8, 107u8, 160u8])]
pub struct AcceptAuthorityInstruction {
    pub accounts: AcceptAuthorityInstructionAccounts,
    pub data: AcceptAuthorityInstructionData,
}

/// Instruction Accounts
#[derive(Debug, Clone, TridentAccounts, Default)]
#[instruction_data(AcceptAuthorityInstructionData)]
#[storage(FuzzAccounts)]
pub struct AcceptAuthorityInstructionAccounts {
    #[account(mut)]
    pub tranche_config: TridentAccount,

    #[account(signer)]
    pub pending_authority: TridentAccount,
}

/// Instruction Data
#[derive(Debug, BorshDeserialize, BorshSerialize, Clone, Default)]
pub struct AcceptAuthorityInstructionData {}

/// Implementation of instruction setters for fuzzing
///
/// Provides methods to:
/// - Set instruction data during fuzzing
/// - Configure instruction accounts during fuzzing
/// - (Optional) Set remaining accounts during fuzzing
///
/// Docs: https://ackee.xyz/trident/docs/latest/start-fuzzing/writting-fuzz-test/
impl InstructionHooks for AcceptAuthorityInstruction {
    type IxAccounts = FuzzAccounts;
}
