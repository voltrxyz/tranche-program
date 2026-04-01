use crate::fuzz_accounts::FuzzAccounts;
use crate::types::*;
use borsh::{BorshDeserialize, BorshSerialize};
use trident_fuzz::fuzzing::*;

#[derive(TridentInstruction, Default)]
#[program_id("EvVxBE31HKE3QdRZXZB7gkkPTcMcAPKGTWJKAvoGuZoX")]
#[discriminator([102u8, 61u8, 43u8, 73u8, 244u8, 77u8, 154u8, 112u8])]
pub struct PushAprObservationInstruction {
    pub accounts: PushAprObservationInstructionAccounts,
    pub data: PushAprObservationInstructionData,
}

/// Instruction Accounts
#[derive(Debug, Clone, TridentAccounts, Default)]
#[instruction_data(PushAprObservationInstructionData)]
#[storage(FuzzAccounts)]
pub struct PushAprObservationInstructionAccounts {
    #[account(mut)]
    pub tranche_config: TridentAccount,

    #[account(mut)]
    pub apr_feed: TridentAccount,

    pub vault: TridentAccount,

    pub vault_lp_mint: TridentAccount,
}

/// Instruction Data
#[derive(Debug, BorshDeserialize, BorshSerialize, Clone, Default)]
pub struct PushAprObservationInstructionData {}

impl InstructionHooks for PushAprObservationInstruction {
    type IxAccounts = FuzzAccounts;

    fn set_accounts(&mut self, _trident: &mut Trident, fuzz_accounts: &mut FuzzAccounts) {
        self.accounts.tranche_config.set_address(fuzz_accounts.tranche_config_pubkey);
        self.accounts.apr_feed.set_address(fuzz_accounts.apr_feed_pubkey);
        self.accounts.vault.set_address(fuzz_accounts.vault_pubkey);
        self.accounts.vault_lp_mint.set_address(fuzz_accounts.vault_lp_mint_pubkey);
    }
}
