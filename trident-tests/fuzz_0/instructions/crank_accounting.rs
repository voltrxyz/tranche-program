use crate::fuzz_accounts::FuzzAccounts;
use crate::types::*;
use borsh::{BorshDeserialize, BorshSerialize};
use trident_fuzz::fuzzing::*;

#[derive(TridentInstruction, Default)]
#[program_id("EvVxBE31HKE3QdRZXZB7gkkPTcMcAPKGTWJKAvoGuZoX")]
#[discriminator([64u8, 223u8, 74u8, 30u8, 9u8, 166u8, 183u8, 26u8])]
pub struct CrankAccountingInstruction {
    pub accounts: CrankAccountingInstructionAccounts,
    pub data: CrankAccountingInstructionData,
}

/// Instruction Accounts
#[derive(Debug, Clone, TridentAccounts, Default)]
#[instruction_data(CrankAccountingInstructionData)]
#[storage(FuzzAccounts)]
pub struct CrankAccountingInstructionAccounts {
    #[account(signer)]
    pub payer: TridentAccount,

    #[account(mut)]
    pub tranche_config: TridentAccount,

    pub vault: TridentAccount,

    pub vault_lp_mint: TridentAccount,

    pub vault_lp_custody: TridentAccount,

    pub senior_mint: TridentAccount,

    pub junior_mint: TridentAccount,
}

/// Instruction Data
#[derive(Debug, BorshDeserialize, BorshSerialize, Clone, Default)]
pub struct CrankAccountingInstructionData {}

impl InstructionHooks for CrankAccountingInstruction {
    type IxAccounts = FuzzAccounts;

    fn set_accounts(&mut self, _trident: &mut Trident, fuzz_accounts: &mut FuzzAccounts) {
        self.accounts.payer.set_address(fuzz_accounts.authority_pubkey);
        self.accounts.tranche_config.set_address(fuzz_accounts.tranche_config_pubkey);
        self.accounts.vault.set_address(fuzz_accounts.vault_pubkey);
        self.accounts.vault_lp_mint.set_address(fuzz_accounts.vault_lp_mint_pubkey);
        self.accounts.vault_lp_custody.set_address(fuzz_accounts.vault_lp_custody_pubkey);
        self.accounts.senior_mint.set_address(fuzz_accounts.senior_mint_pubkey);
        self.accounts.junior_mint.set_address(fuzz_accounts.junior_mint_pubkey);
    }
}
