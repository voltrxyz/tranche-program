use crate::fuzz_accounts::FuzzAccounts;
use crate::types::*;
use borsh::{BorshDeserialize, BorshSerialize};
use trident_fuzz::fuzzing::*;

#[derive(TridentInstruction, Default)]
#[program_id("EvVxBE31HKE3QdRZXZB7gkkPTcMcAPKGTWJKAvoGuZoX")]
#[discriminator([48u8, 169u8, 76u8, 72u8, 229u8, 180u8, 55u8, 161u8])]
pub struct TransferAuthorityInstruction {
    pub accounts: TransferAuthorityInstructionAccounts,
    pub data: TransferAuthorityInstructionData,
}

/// Instruction Accounts
#[derive(Debug, Clone, TridentAccounts, Default)]
#[instruction_data(TransferAuthorityInstructionData)]
#[storage(FuzzAccounts)]
pub struct TransferAuthorityInstructionAccounts {
    #[account(mut)]
    pub tranche_config: TridentAccount,

    #[account(signer)]
    pub authority: TridentAccount,

    pub new_authority: TridentAccount,
}

/// Instruction Data
#[derive(Debug, BorshDeserialize, BorshSerialize, Clone, Default)]
pub struct TransferAuthorityInstructionData {}

/// Implementation of instruction setters for fuzzing
///
/// Provides methods to:
/// - Set instruction data during fuzzing
/// - Configure instruction accounts during fuzzing
/// - (Optional) Set remaining accounts during fuzzing
///
/// Docs: https://ackee.xyz/trident/docs/latest/start-fuzzing/writting-fuzz-test/
impl InstructionHooks for TransferAuthorityInstruction {
    type IxAccounts = FuzzAccounts;
}
