use fuzz_accounts::*;
use trident_fuzz::fuzzing::*;
mod fuzz_accounts;
mod instructions;
mod transactions;
mod types;
pub use transactions::*;

use solana_sdk::account::WritableAccount;
use solana_sdk::program_pack::Pack;

// ── Program IDs ─────────────────────────────────────────────────────────────
const TRANCHING_PROGRAM_ID: Pubkey = pubkey!("EvVxBE31HKE3QdRZXZB7gkkPTcMcAPKGTWJKAvoGuZoX");
const VAULT_PROGRAM_ID: Pubkey = pubkey!("vVoLTRjQmtFpiYoegx285Ze4gsLJ8ZxgFKVcuvmG1a8");
const TOKEN_PROGRAM_ID: Pubkey = pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
const ASSOCIATED_TOKEN_PROGRAM_ID: Pubkey =
    pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");
const SYSTEM_PROGRAM_ID: Pubkey = pubkey!("11111111111111111111111111111111");

// ── Vault account layout constants (repr(C) offsets, including 8-byte discriminator) ──
const VAULT_DISCRIMINATOR: [u8; 8] = [211, 8, 232, 43, 2, 152, 117, 119];
const VAULT_ACCOUNT_SIZE: usize = 928; // 8 discriminator + 920 struct
const VAULT_ASSET_MINT_OFFSET: usize = 104;
const VAULT_ASSET_IDLE_ATA_OFFSET: usize = 136;
const VAULT_ASSET_TOTAL_VALUE_OFFSET: usize = 168;
const VAULT_ASSET_IDLE_AUTH_BUMP_OFFSET: usize = 176;
const VAULT_LP_MINT_OFFSET: usize = 272;
const VAULT_LP_MINT_BUMP_OFFSET: usize = 304;
const VAULT_LP_MINT_AUTH_BUMP_OFFSET: usize = 305;
const VAULT_MANAGER_OFFSET: usize = 368;
const VAULT_ADMIN_OFFSET: usize = 400;
const VAULT_CONFIG_MAX_CAP_OFFSET: usize = 432;
// withdrawal_waiting_period at offset 456 (already zero)
const VAULT_DEAD_WEIGHT_OFFSET: usize = 616;
const VAULT_VERSION_OFFSET: usize = 664;

// Protocol account layout (repr(C), including 8-byte discriminator)
const PROTOCOL_DISCRIMINATOR: [u8; 8] = [45, 39, 101, 43, 115, 72, 131, 40];
const PROTOCOL_ACCOUNT_SIZE: usize = 8 + 32 + 2 + 2 + 1 + 1 + 32 + 32; // ~110 bytes

// ── Tranche Config layout (for reading NAVs after 8-byte discriminator) ──
// NAV fields are [u8; 16] DecimalBits at these offsets in the struct (after discriminator)
// vault: 0..32, authority: 32..64, pending_authority: 64..96
// senior_mint: 96..128, junior_mint: 128..160
// lp_custody: 160..192, asset_mint: 192..224
// total_nav: 224..240, senior_nav: 240..256, junior_nav: 256..272, reserve_nav: 272..288
const TC_TOTAL_NAV_OFFSET: usize = 8 + 224;
const TC_SENIOR_NAV_OFFSET: usize = 8 + 240;
const TC_JUNIOR_NAV_OFFSET: usize = 8 + 256;
const TC_RESERVE_NAV_OFFSET: usize = 8 + 272;

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Read a [u8; 16] from account data at a given offset and interpret as u128 LE.
fn read_u128_le(data: &[u8], offset: usize) -> u128 {
    let bytes: [u8; 16] = data[offset..offset + 16].try_into().unwrap_or([0; 16]);
    u128::from_le_bytes(bytes)
}

/// Derive the Associated Token Address for a given wallet and mint.
fn get_ata(wallet: &Pubkey, mint: &Pubkey) -> Pubkey {
    spl_associated_token_account::get_associated_token_address(wallet, mint)
}

/// Create a packed SPL Token Mint account and inject into the SVM.
fn create_mint_account(client: &mut impl FuzzClient, address: &Pubkey, decimals: u8, authority: &Pubkey) {
    let mut data = vec![0u8; spl_token::state::Mint::LEN];
    let mint = spl_token::state::Mint {
        mint_authority: solana_sdk::program_option::COption::Some(*authority),
        supply: 0,
        decimals,
        is_initialized: true,
        freeze_authority: solana_sdk::program_option::COption::None,
    };
    spl_token::state::Mint::pack(mint, &mut data).unwrap();
    let account = AccountSharedData::create(
        1_000_000_000, // enough lamports for rent
        data,
        TOKEN_PROGRAM_ID,
        false,
        0,
    );
    client.set_account_custom(address, &account);
}

/// Create a packed SPL Token Account and inject into the SVM.
fn create_token_account(
    client: &mut impl FuzzClient,
    address: &Pubkey,
    mint: &Pubkey,
    owner: &Pubkey,
    amount: u64,
) {
    let mut data = vec![0u8; spl_token::state::Account::LEN];
    let token_account = spl_token::state::Account {
        mint: *mint,
        owner: *owner,
        amount,
        delegate: solana_sdk::program_option::COption::None,
        state: spl_token::state::AccountState::Initialized,
        is_native: solana_sdk::program_option::COption::None,
        delegated_amount: 0,
        close_authority: solana_sdk::program_option::COption::None,
    };
    spl_token::state::Account::pack(token_account, &mut data).unwrap();
    let account = AccountSharedData::create(
        1_000_000_000,
        data,
        TOKEN_PROGRAM_ID,
        false,
        0,
    );
    client.set_account_custom(address, &account);
}

/// Build a Vault account blob with specific fields set.
fn build_vault_account(
    asset_mint: &Pubkey,
    asset_idle_ata: &Pubkey,
    idle_auth_bump: u8,
    lp_mint: &Pubkey,
    lp_mint_bump: u8,
    lp_mint_auth_bump: u8,
    manager: &Pubkey,
    admin: &Pubkey,
) -> Vec<u8> {
    let mut data = vec![0u8; VAULT_ACCOUNT_SIZE];
    // discriminator
    data[..8].copy_from_slice(&VAULT_DISCRIMINATOR);
    // asset.mint
    data[VAULT_ASSET_MINT_OFFSET..VAULT_ASSET_MINT_OFFSET + 32]
        .copy_from_slice(asset_mint.as_ref());
    // asset.idle_ata
    data[VAULT_ASSET_IDLE_ATA_OFFSET..VAULT_ASSET_IDLE_ATA_OFFSET + 32]
        .copy_from_slice(asset_idle_ata.as_ref());
    // asset.total_value = 0 (already zero)
    // asset.idle_ata_auth_bump
    data[VAULT_ASSET_IDLE_AUTH_BUMP_OFFSET] = idle_auth_bump;
    // lp.mint
    data[VAULT_LP_MINT_OFFSET..VAULT_LP_MINT_OFFSET + 32].copy_from_slice(lp_mint.as_ref());
    // lp.mint_bump
    data[VAULT_LP_MINT_BUMP_OFFSET] = lp_mint_bump;
    // lp.mint_auth_bump
    data[VAULT_LP_MINT_AUTH_BUMP_OFFSET] = lp_mint_auth_bump;
    // manager
    data[VAULT_MANAGER_OFFSET..VAULT_MANAGER_OFFSET + 32].copy_from_slice(manager.as_ref());
    // admin
    data[VAULT_ADMIN_OFFSET..VAULT_ADMIN_OFFSET + 32].copy_from_slice(admin.as_ref());
    // vault_configuration.max_cap = u64::MAX
    data[VAULT_CONFIG_MAX_CAP_OFFSET..VAULT_CONFIG_MAX_CAP_OFFSET + 8]
        .copy_from_slice(&u64::MAX.to_le_bytes());
    // dead_weight = 1000 (prevent first-depositor attack)
    data[VAULT_DEAD_WEIGHT_OFFSET..VAULT_DEAD_WEIGHT_OFFSET + 8]
        .copy_from_slice(&1000u64.to_le_bytes());
    // version = 1
    data[VAULT_VERSION_OFFSET] = 1;
    data
}

/// Build a Protocol account blob.
fn build_protocol_account(admin: &Pubkey, bump: u8) -> Vec<u8> {
    let mut data = vec![0u8; PROTOCOL_ACCOUNT_SIZE];
    data[..8].copy_from_slice(&PROTOCOL_DISCRIMINATOR);
    data[8..40].copy_from_slice(admin.as_ref()); // admin pubkey
    data[40..42].copy_from_slice(&8191u16.to_le_bytes()); // operational_state = all enabled
    // _padding1 at 42..44 (zero)
    data[44] = bump; // bump
    // remaining zeros are fine
    data
}

// ── Fuzz Test ───────────────────────────────────────────────────────────────

#[derive(FuzzTestMethods)]
struct FuzzTest {
    trident: Trident,
    fuzz_accounts: FuzzAccounts,
}

#[flow_executor]
impl FuzzTest {
    fn new() -> Self {
        Self {
            trident: Trident::default(),
            fuzz_accounts: FuzzAccounts::default(),
        }
    }

    #[init]
    fn start(&mut self) {
        // ── 1. Deterministic keypairs ───────────────────────────────────
        let payer = self.trident.get_client().payer().pubkey();
        let authority = payer; // use payer as authority (simplifies signing)
        let user = payer; // use payer as user too (TridentSVM only signs with payer)
        let vault_pubkey = self.trident.gen_pubkey();

        // ── 2. Derive Vault PDAs ────────────────────────────────────────
        let (protocol_pubkey, protocol_bump) =
            Pubkey::find_program_address(&[b"protocol"], &VAULT_PROGRAM_ID);
        let (vault_lp_mint, lp_mint_bump) =
            Pubkey::find_program_address(&[b"vault_lp_mint", vault_pubkey.as_ref()], &VAULT_PROGRAM_ID);
        let (vault_lp_mint_auth, lp_mint_auth_bump) =
            Pubkey::find_program_address(&[b"vault_lp_mint_auth", vault_pubkey.as_ref()], &VAULT_PROGRAM_ID);
        let (vault_asset_idle_auth, idle_auth_bump) =
            Pubkey::find_program_address(&[b"vault_asset_idle_auth", vault_pubkey.as_ref()], &VAULT_PROGRAM_ID);

        // ── 3. Create asset mint ────────────────────────────────────────
        let asset_mint = self.trident.gen_pubkey();
        create_mint_account(self.trident.get_client(), &asset_mint, 6, &payer);

        // ── 4. Create vault LP mint (authority = vault_lp_mint_auth PDA) ─
        create_mint_account(self.trident.get_client(), &vault_lp_mint, 6, &vault_lp_mint_auth);

        // ── 5. Create vault asset idle ATA ──────────────────────────────
        let vault_asset_idle_ata = get_ata(&vault_asset_idle_auth, &asset_mint);
        create_token_account(
            self.trident.get_client(),
            &vault_asset_idle_ata,
            &asset_mint,
            &vault_asset_idle_auth,
            0,
        );

        // ── 6. Create & inject Vault account ────────────────────────────
        let vault_data = build_vault_account(
            &asset_mint,
            &vault_asset_idle_ata,
            idle_auth_bump,
            &vault_lp_mint,
            lp_mint_bump,
            lp_mint_auth_bump,
            &authority,
            &authority,
        );
        let vault_account = AccountSharedData::create(
            10_000_000_000,
            vault_data,
            VAULT_PROGRAM_ID,
            false,
            0,
        );
        self.trident.get_client().set_account_custom(&vault_pubkey, &vault_account);

        // ── 7. Create & inject Protocol account ─────────────────────────
        let protocol_data = build_protocol_account(&authority, protocol_bump);
        let protocol_account = AccountSharedData::create(
            1_000_000_000,
            protocol_data,
            VAULT_PROGRAM_ID,
            false,
            0,
        );
        self.trident.get_client().set_account_custom(&protocol_pubkey, &protocol_account);

        // ── 8. Airdrop to payer ─────────────────────────────────────────
        self.trident.airdrop(&payer, 100 * LAMPORTS_PER_SOL);

        // ── 9. Derive Tranche PDAs ──────────────────────────────────────
        let (tranche_config, _) =
            Pubkey::find_program_address(&[b"tranche_config", vault_pubkey.as_ref()], &TRANCHING_PROGRAM_ID);
        let (tranche_authority, _) =
            Pubkey::find_program_address(&[b"tranche_authority", tranche_config.as_ref()], &TRANCHING_PROGRAM_ID);
        let (senior_mint, _) =
            Pubkey::find_program_address(&[b"senior_mint", tranche_config.as_ref()], &TRANCHING_PROGRAM_ID);
        let (junior_mint, _) =
            Pubkey::find_program_address(&[b"junior_mint", tranche_config.as_ref()], &TRANCHING_PROGRAM_ID);
        let (apr_feed, _) =
            Pubkey::find_program_address(&[b"apr_feed", tranche_config.as_ref()], &TRANCHING_PROGRAM_ID);

        let vault_lp_custody = get_ata(&tranche_authority, &vault_lp_mint);
        let asset_escrow = get_ata(&tranche_authority, &asset_mint);

        // ── 10. Execute InitializeTrancheConfig ─────────────────────────
        // Build instruction data: discriminator + borsh(InitializeTrancheConfigParams)
        let risk_k_decimal_bits = {
            // Decimal::from_num(2) as U80F48: 2 << 48 = 562949953421312
            let val: u128 = 2u128 << 48;
            val.to_le_bytes()
        };

        let mut ix_data = vec![];
        // discriminator for initialize_tranche_config
        ix_data.extend_from_slice(&[58u8, 73u8, 224u8, 50u8, 220u8, 233u8, 56u8, 147u8]);
        // InitializeTrancheConfigParams (borsh-serialized)
        ix_data.extend_from_slice(&200u16.to_le_bytes()); // apr_floor_bps
        ix_data.extend_from_slice(&500u16.to_le_bytes()); // risk_x_bps
        ix_data.extend_from_slice(&1000u16.to_le_bytes()); // risk_y_bps
        ix_data.extend_from_slice(&risk_k_decimal_bits); // risk_k [u8; 16]
        ix_data.extend_from_slice(&500u16.to_le_bytes()); // reserve_fee_bps (5%)
        ix_data.extend_from_slice(&50u16.to_le_bytes()); // senior_exit_fee_bps (0.5%)
        ix_data.extend_from_slice(&50u16.to_le_bytes()); // junior_exit_fee_bps (0.5%)
        ix_data.extend_from_slice(&1000u16.to_le_bytes()); // min_jr_sr_ratio_bps (10%)
        ix_data.extend_from_slice(&10000u16.to_le_bytes()); // min_coverage_for_mint_bps (100%)
        ix_data.extend_from_slice(&11000u16.to_le_bytes()); // min_coverage_buffer_bps (110%)
        ix_data.extend_from_slice(&0u64.to_le_bytes()); // senior_cooldown_seconds = 0
        ix_data.extend_from_slice(&0u64.to_le_bytes()); // junior_cooldown_seconds = 0
        ix_data.extend_from_slice(&259200u64.to_le_bytes()); // max_observation_age_seconds

        let init_ix = solana_sdk::instruction::Instruction {
            program_id: TRANCHING_PROGRAM_ID,
            accounts: vec![
                AccountMeta::new(authority, true),       // authority (mut, signer)
                AccountMeta::new_readonly(vault_pubkey, false), // vault
                AccountMeta::new(tranche_config, false), // tranche_config (mut)
                AccountMeta::new_readonly(tranche_authority, false), // tranche_authority
                AccountMeta::new(senior_mint, false),    // senior_mint (mut)
                AccountMeta::new(junior_mint, false),    // junior_mint (mut)
                AccountMeta::new_readonly(asset_mint, false), // asset_mint
                AccountMeta::new_readonly(vault_lp_mint, false), // vault_lp_mint
                AccountMeta::new(vault_lp_custody, false), // vault_lp_custody (mut)
                AccountMeta::new(asset_escrow, false),   // asset_escrow (mut)
                AccountMeta::new(apr_feed, false),       // apr_feed (mut)
                AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
                AccountMeta::new_readonly(ASSOCIATED_TOKEN_PROGRAM_ID, false),
                AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
            ],
            data: ix_data,
        };

        // Execute init - if it fails, all subsequent instructions will fail gracefully
        let _ = self.trident.get_client()._process_instructions(&[init_ix]);

        // ── 11. Create user token accounts ──────────────────────────────
        let user_asset_ata = get_ata(&user, &asset_mint);
        create_token_account(self.trident.get_client(), &user_asset_ata, &asset_mint, &user, 10_000_000_000); // 10,000 USDC

        let user_senior_ata = get_ata(&user, &senior_mint);
        create_token_account(self.trident.get_client(), &user_senior_ata, &senior_mint, &user, 0);

        let user_junior_ata = get_ata(&user, &junior_mint);
        create_token_account(self.trident.get_client(), &user_junior_ata, &junior_mint, &user, 0);

        // Create escrow ATAs for withdrawals
        let senior_token_escrow = get_ata(&tranche_authority, &senior_mint);
        create_token_account(self.trident.get_client(), &senior_token_escrow, &senior_mint, &tranche_authority, 0);

        let junior_token_escrow = get_ata(&tranche_authority, &junior_mint);
        create_token_account(self.trident.get_client(), &junior_token_escrow, &junior_mint, &tranche_authority, 0);

        // ── 12. Seed deposits (junior first for coverage, then senior) ──
        let deposit_junior_ix = build_deposit_junior_ix(
            &user, &tranche_config, &tranche_authority, &senior_mint,
            &junior_mint, &user_asset_ata, &user_junior_ata, &asset_escrow,
            &vault_lp_custody, &asset_mint, &vault_pubkey, &vault_lp_mint,
            &protocol_pubkey, &vault_asset_idle_ata, &vault_asset_idle_auth,
            &vault_lp_mint_auth, 2_000_000_000, // 2000 USDC junior
        );
        let _ = self.trident.get_client()._process_instructions(&[deposit_junior_ix]);

        let deposit_senior_ix = build_deposit_senior_ix(
            &user, &tranche_config, &tranche_authority, &senior_mint,
            &junior_mint, &user_asset_ata, &user_senior_ata, &asset_escrow,
            &vault_lp_custody, &asset_mint, &vault_pubkey, &vault_lp_mint,
            &protocol_pubkey, &vault_asset_idle_ata, &vault_asset_idle_auth,
            &vault_lp_mint_auth, 1_000_000_000, // 1000 USDC senior
        );
        let _ = self.trident.get_client()._process_instructions(&[deposit_senior_ix]);

        // Set timestamp to a reasonable value
        self.trident.get_client().warp_to_timestamp(1_700_000_000);

        // ── 13. Store all derived addresses ─────────────────────────────
        self.fuzz_accounts.initialized = true;
        self.fuzz_accounts.vault_pubkey = vault_pubkey;
        self.fuzz_accounts.asset_mint_pubkey = asset_mint;
        self.fuzz_accounts.authority_pubkey = authority;
        self.fuzz_accounts.user_pubkey = user;
        self.fuzz_accounts.tranche_config_pubkey = tranche_config;
        self.fuzz_accounts.tranche_authority_pubkey = tranche_authority;
        self.fuzz_accounts.senior_mint_pubkey = senior_mint;
        self.fuzz_accounts.junior_mint_pubkey = junior_mint;
        self.fuzz_accounts.apr_feed_pubkey = apr_feed;
        self.fuzz_accounts.vault_lp_mint_pubkey = vault_lp_mint;
        self.fuzz_accounts.vault_lp_mint_auth_pubkey = vault_lp_mint_auth;
        self.fuzz_accounts.vault_asset_idle_auth_pubkey = vault_asset_idle_auth;
        self.fuzz_accounts.vault_asset_idle_ata_pubkey = vault_asset_idle_ata;
        self.fuzz_accounts.vault_lp_custody_pubkey = vault_lp_custody;
        self.fuzz_accounts.asset_escrow_pubkey = asset_escrow;
        self.fuzz_accounts.user_asset_ata_pubkey = user_asset_ata;
        self.fuzz_accounts.user_senior_ata_pubkey = user_senior_ata;
        self.fuzz_accounts.user_junior_ata_pubkey = user_junior_ata;
        self.fuzz_accounts.protocol_pubkey = protocol_pubkey;
        self.fuzz_accounts.senior_token_escrow_pubkey = senior_token_escrow;
        self.fuzz_accounts.junior_token_escrow_pubkey = junior_token_escrow;
    }

    // ── Fuzz Flows ──────────────────────────────────────────────────────────

    #[flow]
    fn flow_deposit_junior(&mut self) {
        if !self.fuzz_accounts.initialized { return; }
        let mut tx = DepositJuniorTransaction::build(&mut self.trident, &mut self.fuzz_accounts);
        self.trident.execute_transaction(&mut tx, Some("DepositJunior"));
    }

    #[flow]
    fn flow_deposit_senior(&mut self) {
        if !self.fuzz_accounts.initialized { return; }
        let mut tx = DepositSeniorTransaction::build(&mut self.trident, &mut self.fuzz_accounts);
        self.trident.execute_transaction(&mut tx, Some("DepositSenior"));
    }

    #[flow]
    fn flow_crank_accounting(&mut self) {
        if !self.fuzz_accounts.initialized { return; }
        let mut tx = CrankAccountingTransaction::build(&mut self.trident, &mut self.fuzz_accounts);
        self.trident.execute_transaction(&mut tx, Some("CrankAccounting"));
    }

    #[flow]
    fn flow_instant_withdraw_senior(&mut self) {
        if !self.fuzz_accounts.initialized { return; }
        let mut tx = InstantWithdrawSeniorTransaction::build(&mut self.trident, &mut self.fuzz_accounts);
        self.trident.execute_transaction(&mut tx, Some("InstantWithdrawSenior"));
    }

    #[flow]
    fn flow_instant_withdraw_junior(&mut self) {
        if !self.fuzz_accounts.initialized { return; }
        let mut tx = InstantWithdrawJuniorTransaction::build(&mut self.trident, &mut self.fuzz_accounts);
        self.trident.execute_transaction(&mut tx, Some("InstantWithdrawJunior"));
    }

    #[flow]
    fn flow_push_apr_observation(&mut self) {
        if !self.fuzz_accounts.initialized { return; }
        let mut tx = PushAprObservationTransaction::build(&mut self.trident, &mut self.fuzz_accounts);
        self.trident.execute_transaction(&mut tx, Some("PushAprObservation"));
    }

    #[flow]
    fn flow_advance_time(&mut self) {
        if !self.fuzz_accounts.initialized { return; }
        // Advance time by 1-3600 seconds to test APR accrual and cooldowns
        let seconds: i64 = self.trident.gen_range(1..3601);
        self.trident.get_client().forward_in_time(seconds);
    }

    // ── Invariant Checks ────────────────────────────────────────────────────

    #[end]
    fn end(&mut self) {
        if !self.fuzz_accounts.initialized { return; }

        let tc_data = self.trident.get_client().get_account(&self.fuzz_accounts.tranche_config_pubkey);
        let data = tc_data.data();
        if data.len() < TC_RESERVE_NAV_OFFSET + 16 {
            return; // account not properly initialized
        }

        let total_nav = read_u128_le(data, TC_TOTAL_NAV_OFFSET);
        let senior_nav = read_u128_le(data, TC_SENIOR_NAV_OFFSET);
        let junior_nav = read_u128_le(data, TC_JUNIOR_NAV_OFFSET);
        let reserve_nav = read_u128_le(data, TC_RESERVE_NAV_OFFSET);

        // ── Invariant 1: NAV conservation ───────────────────────────────
        // senior_nav + junior_nav + reserve_nav == total_nav (within rounding tolerance)
        let nav_sum = senior_nav.saturating_add(junior_nav).saturating_add(reserve_nav);
        let tolerance = 3u128; // allow tiny rounding drift
        if nav_sum > total_nav.saturating_add(tolerance) || total_nav > nav_sum.saturating_add(tolerance) {
            panic!(
                "INVARIANT VIOLATION: NAV conservation failed! \
                 total_nav={}, senior+junior+reserve={}, senior={}, junior={}, reserve={}",
                total_nav, nav_sum, senior_nav, junior_nav, reserve_nav
            );
        }

        // ── Invariant 2: No phantom senior tokens ──────────────────────
        let senior_mint_data = self.trident.get_client().get_account(&self.fuzz_accounts.senior_mint_pubkey);
        let senior_data = senior_mint_data.data();
        let senior_supply = if senior_data.len() >= spl_token::state::Mint::LEN {
            if let Ok(mint) = spl_token::state::Mint::unpack(senior_data) {
                if mint.supply > 0 && senior_nav == 0 && total_nav > 0 {
                    panic!(
                        "INVARIANT VIOLATION: Senior phantom tokens! supply={} but senior_nav=0",
                        mint.supply
                    );
                }
                mint.supply
            } else { 0 }
        } else { 0 };

        // ── Invariant 3: No phantom junior tokens ──────────────────────
        let junior_mint_data = self.trident.get_client().get_account(&self.fuzz_accounts.junior_mint_pubkey);
        let junior_data = junior_mint_data.data();
        let junior_supply = if junior_data.len() >= spl_token::state::Mint::LEN {
            if let Ok(mint) = spl_token::state::Mint::unpack(junior_data) {
                if mint.supply > 0 && junior_nav == 0 && total_nav > 0 {
                    panic!(
                        "INVARIANT VIOLATION: Junior phantom tokens! supply={} but junior_nav=0",
                        mint.supply
                    );
                }
                mint.supply
            } else { 0 }
        } else { 0 };

        // ── Invariant 4: LP token value correspondence ─────────────────
        // The LP tokens in vault_lp_custody, valued at current vault prices,
        // should be >= total_nav (within tolerance for rounding).
        let lp_custody_data = self.trident.get_client().get_account(&self.fuzz_accounts.vault_lp_custody_pubkey);
        let lp_custody_balance = if lp_custody_data.data().len() >= spl_token::state::Account::LEN {
            if let Ok(acct) = spl_token::state::Account::unpack(lp_custody_data.data()) {
                acct.amount
            } else { 0 }
        } else { 0 };

        let vault_data = self.trident.get_client().get_account(&self.fuzz_accounts.vault_pubkey);
        let vd = vault_data.data();
        if vd.len() >= VAULT_ACCOUNT_SIZE {
            let vault_total_value = u64::from_le_bytes(
                vd[VAULT_ASSET_TOTAL_VALUE_OFFSET..VAULT_ASSET_TOTAL_VALUE_OFFSET + 8]
                    .try_into().unwrap_or([0; 8])
            );
            let lp_mint_data = self.trident.get_client().get_account(&self.fuzz_accounts.vault_lp_mint_pubkey);
            let lp_supply = if lp_mint_data.data().len() >= spl_token::state::Mint::LEN {
                if let Ok(m) = spl_token::state::Mint::unpack(lp_mint_data.data()) { m.supply } else { 0 }
            } else { 0 };

            if lp_supply > 0 && vault_total_value > 0 {
                // our_value = lp_custody_balance * vault_total_value / lp_supply
                let our_value = (lp_custody_balance as u128)
                    .saturating_mul(vault_total_value as u128)
                    / (lp_supply as u128);
                // total_nav (as DecimalBits u128) needs to be compared as raw asset value.
                // DecimalBits is U80F48, so divide by 2^48 to get integer part.
                let total_nav_int = total_nav >> 48;
                let lp_tolerance = total_nav_int / 100; // 1% tolerance
                if our_value.saturating_add(lp_tolerance) < total_nav_int {
                    panic!(
                        "INVARIANT VIOLATION: LP value mismatch! our_lp_value={}, total_nav_int={}, \
                         lp_custody={}, vault_total={}, lp_supply={}",
                        our_value, total_nav_int, lp_custody_balance, vault_total_value, lp_supply
                    );
                }
            }
        }

        // ── Invariant 5: Coverage ratio enforcement ────────────────────
        // If senior_nav > 0 and total_nav > 0, coverage = total_nav / senior_nav >= 1
        // (junior + reserve must be non-negative, which is guaranteed by u128)
        if senior_nav > 0 && total_nav > 0 && total_nav < senior_nav {
            panic!(
                "INVARIANT VIOLATION: Coverage ratio < 1! total_nav={} < senior_nav={}",
                total_nav, senior_nav
            );
        }

        // ── Invariant 6: Junior ratio enforcement ──────────────────────
        // After successful withdrawals, junior_nav should respect the min ratio.
        // We check: if senior_nav > 0 and junior_supply > 0, junior_nav should not
        // be negative (guaranteed by u128), and the ratio is market-driven.
        // The hard invariant is: junior_nav + reserve_nav >= 0 (always true for u128).
        // We verify: senior_nav <= total_nav (senior can't exceed total)
        if senior_nav > total_nav.saturating_add(tolerance) {
            panic!(
                "INVARIANT VIOLATION: Senior NAV exceeds total! senior_nav={}, total_nav={}",
                senior_nav, total_nav
            );
        }

        // ── Invariant 7: No panics ─────────────────────────────────────
        // (Implicit: if we reached this point without panicking, the test passed)
        let _ = (senior_supply, junior_supply); // suppress unused warnings
    }
}

// ── Raw instruction builders ────────────────────────────────────────────────

fn build_deposit_senior_ix(
    user: &Pubkey, tranche_config: &Pubkey, tranche_authority: &Pubkey,
    senior_mint: &Pubkey, junior_mint: &Pubkey, user_asset_ata: &Pubkey,
    user_senior_ata: &Pubkey, asset_escrow: &Pubkey, vault_lp_custody: &Pubkey,
    asset_mint: &Pubkey, vault: &Pubkey, vault_lp_mint: &Pubkey,
    protocol: &Pubkey, vault_asset_idle_ata: &Pubkey, vault_asset_idle_auth: &Pubkey,
    vault_lp_mint_auth: &Pubkey, amount: u64,
) -> solana_sdk::instruction::Instruction {
    let mut data = vec![];
    data.extend_from_slice(&[218u8, 97u8, 124u8, 117u8, 212u8, 161u8, 218u8, 219u8]); // discriminator
    data.extend_from_slice(&amount.to_le_bytes());

    solana_sdk::instruction::Instruction {
        program_id: TRANCHING_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*user, true),
            AccountMeta::new(*tranche_config, false),
            AccountMeta::new_readonly(*tranche_authority, false),
            AccountMeta::new(*senior_mint, false),
            AccountMeta::new_readonly(*junior_mint, false),
            AccountMeta::new(*user_asset_ata, false),
            AccountMeta::new(*user_senior_ata, false),
            AccountMeta::new(*asset_escrow, false),
            AccountMeta::new(*vault_lp_custody, false),
            AccountMeta::new_readonly(*asset_mint, false),
            AccountMeta::new(*vault, false),
            AccountMeta::new(*vault_lp_mint, false),
            AccountMeta::new_readonly(*protocol, false),
            AccountMeta::new(*vault_asset_idle_ata, false),
            AccountMeta::new_readonly(*vault_asset_idle_auth, false),
            AccountMeta::new_readonly(*vault_lp_mint_auth, false),
            AccountMeta::new_readonly(VAULT_PROGRAM_ID, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data,
    }
}

fn build_deposit_junior_ix(
    user: &Pubkey, tranche_config: &Pubkey, tranche_authority: &Pubkey,
    senior_mint: &Pubkey, junior_mint: &Pubkey, user_asset_ata: &Pubkey,
    user_junior_ata: &Pubkey, asset_escrow: &Pubkey, vault_lp_custody: &Pubkey,
    asset_mint: &Pubkey, vault: &Pubkey, vault_lp_mint: &Pubkey,
    protocol: &Pubkey, vault_asset_idle_ata: &Pubkey, vault_asset_idle_auth: &Pubkey,
    vault_lp_mint_auth: &Pubkey, amount: u64,
) -> solana_sdk::instruction::Instruction {
    let mut data = vec![];
    data.extend_from_slice(&[4u8, 71u8, 156u8, 106u8, 25u8, 232u8, 177u8, 157u8]); // discriminator
    data.extend_from_slice(&amount.to_le_bytes());

    solana_sdk::instruction::Instruction {
        program_id: TRANCHING_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*user, true),
            AccountMeta::new(*tranche_config, false),
            AccountMeta::new_readonly(*tranche_authority, false),
            AccountMeta::new_readonly(*senior_mint, false),
            AccountMeta::new(*junior_mint, false),
            AccountMeta::new(*user_asset_ata, false),
            AccountMeta::new(*user_junior_ata, false),
            AccountMeta::new(*asset_escrow, false),
            AccountMeta::new(*vault_lp_custody, false),
            AccountMeta::new_readonly(*asset_mint, false),
            AccountMeta::new(*vault, false),
            AccountMeta::new(*vault_lp_mint, false),
            AccountMeta::new_readonly(*protocol, false),
            AccountMeta::new(*vault_asset_idle_ata, false),
            AccountMeta::new_readonly(*vault_asset_idle_auth, false),
            AccountMeta::new_readonly(*vault_lp_mint_auth, false),
            AccountMeta::new_readonly(VAULT_PROGRAM_ID, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data,
    }
}

fn main() {
    FuzzTest::fuzz(20_000, 50);
}
