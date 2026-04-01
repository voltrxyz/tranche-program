use crate::fuzz_accounts::FuzzAccounts;
use crate::types::*;
use borsh::{BorshDeserialize, BorshSerialize};
use trident_fuzz::fuzzing::*;

#[derive(TridentInstruction, Default)]
#[program_id("EvVxBE31HKE3QdRZXZB7gkkPTcMcAPKGTWJKAvoGuZoX")]
#[discriminator([218u8, 97u8, 124u8, 117u8, 212u8, 161u8, 218u8, 219u8])]
pub struct DepositSeniorInstruction {
    pub accounts: DepositSeniorInstructionAccounts,
    pub data: DepositSeniorInstructionData,
}

/// Instruction Accounts
#[derive(Debug, Clone, TridentAccounts, Default)]
#[instruction_data(DepositSeniorInstructionData)]
#[storage(FuzzAccounts)]
pub struct DepositSeniorInstructionAccounts {
    #[account(mut, signer)]
    pub user: TridentAccount,

    #[account(mut)]
    pub tranche_config: TridentAccount,

    pub tranche_authority: TridentAccount,

    #[account(mut)]
    pub senior_mint: TridentAccount,

    pub junior_mint: TridentAccount,

    #[account(mut)]
    pub user_asset_ata: TridentAccount,

    #[account(mut)]
    pub user_senior_ata: TridentAccount,

    #[account(mut)]
    pub asset_escrow: TridentAccount,

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

    pub vault_asset_idle_auth: TridentAccount,

    pub vault_lp_mint_auth: TridentAccount,

    #[account(address = "vVoLTRjQmtFpiYoegx285Ze4gsLJ8ZxgFKVcuvmG1a8")]
    pub voltr_vault_program: TridentAccount,

    #[account(address = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA")]
    pub token_program: TridentAccount,

    #[account(address = "11111111111111111111111111111111")]
    pub system_program: TridentAccount,
}

/// Instruction Data
#[derive(Debug, BorshDeserialize, BorshSerialize, Clone, Default)]
pub struct DepositSeniorInstructionData {
    pub amount: u64,
}

impl InstructionHooks for DepositSeniorInstruction {
    type IxAccounts = FuzzAccounts;

    fn set_accounts(&mut self, _trident: &mut Trident, fuzz_accounts: &mut FuzzAccounts) {
        self.accounts.user.set_address(fuzz_accounts.user_pubkey);
        self.accounts.tranche_config.set_address(fuzz_accounts.tranche_config_pubkey);
        self.accounts.tranche_authority.set_address(fuzz_accounts.tranche_authority_pubkey);
        self.accounts.senior_mint.set_address(fuzz_accounts.senior_mint_pubkey);
        self.accounts.junior_mint.set_address(fuzz_accounts.junior_mint_pubkey);
        self.accounts.user_asset_ata.set_address(fuzz_accounts.user_asset_ata_pubkey);
        self.accounts.user_senior_ata.set_address(fuzz_accounts.user_senior_ata_pubkey);
        self.accounts.asset_escrow.set_address(fuzz_accounts.asset_escrow_pubkey);
        self.accounts.vault_lp_custody.set_address(fuzz_accounts.vault_lp_custody_pubkey);
        self.accounts.asset_mint.set_address(fuzz_accounts.asset_mint_pubkey);
        self.accounts.vault.set_address(fuzz_accounts.vault_pubkey);
        self.accounts.vault_lp_mint.set_address(fuzz_accounts.vault_lp_mint_pubkey);
        self.accounts.protocol.set_address(fuzz_accounts.protocol_pubkey);
        self.accounts.vault_asset_idle_ata.set_address(fuzz_accounts.vault_asset_idle_ata_pubkey);
        self.accounts.vault_asset_idle_auth.set_address(fuzz_accounts.vault_asset_idle_auth_pubkey);
        self.accounts.vault_lp_mint_auth.set_address(fuzz_accounts.vault_lp_mint_auth_pubkey);
    }

    fn set_data(&mut self, trident: &mut Trident, _fuzz_accounts: &mut FuzzAccounts) {
        let pick: u8 = trident.gen_range(0..10);
        self.data.amount = match pick {
            0 => 0,
            1 => 1,
            2 => u64::MAX,
            _ => trident.gen_range(1..5_000_000_000u64),
        };
    }
}
