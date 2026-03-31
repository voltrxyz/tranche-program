use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{Mint, Token, TokenAccount},
};

use crate::constants::*;
use crate::error::TrancheError;
use crate::math::{decimal_bits_to_u32_if_integer, Decimal, DecimalExtra};
use crate::state::{AprFeed, TrancheConfig};

#[derive(AnchorDeserialize, AnchorSerialize)]
pub struct InitializeTrancheConfigParams {
    pub apr_floor_bps: u16,
    pub risk_x_bps: u16,
    pub risk_y_bps: u16,
    pub risk_k: [u8; 16],
    pub reserve_fee_bps: u16,
    pub senior_exit_fee_bps: u16,
    pub junior_exit_fee_bps: u16,
    pub min_jr_sr_ratio_bps: u16,
    pub min_coverage_for_mint_bps: u16,
    pub min_coverage_buffer_bps: u16,
    pub senior_cooldown_seconds: u64,
    pub junior_cooldown_seconds: u64,
    pub max_observation_age_seconds: u64,
}

#[derive(Accounts)]
pub struct InitializeTrancheConfig<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    pub vault: AccountLoader<'info, voltr_vault::state::Vault>,

    #[account(
        init,
        payer = authority,
        space = 8 + std::mem::size_of::<TrancheConfig>(),
        seeds = [TRANCHE_CONFIG_SEED, vault.key().as_ref()],
        bump,
    )]
    pub tranche_config: AccountLoader<'info, TrancheConfig>,

    /// CHECK: PDA used only as CPI signer; validated by seeds constraint.
    #[account(
        seeds = [TRANCHE_AUTHORITY_SEED, tranche_config.key().as_ref()],
        bump,
    )]
    pub tranche_authority: UncheckedAccount<'info>,

    #[account(
        init,
        payer = authority,
        mint::decimals = asset_mint.decimals,
        mint::authority = tranche_authority,
        seeds = [SENIOR_MINT_SEED, tranche_config.key().as_ref()],
        bump,
    )]
    pub senior_mint: Account<'info, Mint>,

    #[account(
        init,
        payer = authority,
        mint::decimals = asset_mint.decimals,
        mint::authority = tranche_authority,
        seeds = [JUNIOR_MINT_SEED, tranche_config.key().as_ref()],
        bump,
    )]
    pub junior_mint: Account<'info, Mint>,

    pub asset_mint: Account<'info, Mint>,

    pub vault_lp_mint: Account<'info, Mint>,

    #[account(
        init,
        payer = authority,
        associated_token::mint = vault_lp_mint,
        associated_token::authority = tranche_authority,
    )]
    pub vault_lp_custody: Account<'info, TokenAccount>,

    #[account(
        init,
        payer = authority,
        associated_token::mint = asset_mint,
        associated_token::authority = tranche_authority,
    )]
    pub asset_escrow: Account<'info, TokenAccount>,

    #[account(
        init,
        payer = authority,
        space = 8 + std::mem::size_of::<AprFeed>(),
        seeds = [APR_FEED_SEED, tranche_config.key().as_ref()],
        bump,
    )]
    pub apr_feed: AccountLoader<'info, AprFeed>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

pub fn handle_initialize_tranche_config(
    ctx: Context<InitializeTrancheConfig>,
    params: InitializeTrancheConfigParams,
) -> Result<()> {
    // ── Validate vault ──────────────────────────────────────────────
    let vault = ctx.accounts.vault.load()?;
    require!(
        vault.vault_configuration.withdrawal_waiting_period == 0,
        TrancheError::VaultWaitingPeriodNotZero
    );
    require!(
        vault.asset.mint == ctx.accounts.asset_mint.key(),
        TrancheError::InvalidVault
    );
    require!(
        vault.lp.mint == ctx.accounts.vault_lp_mint.key(),
        TrancheError::InvalidVault
    );
    drop(vault);

    // ── Validate fee parameters ─────────────────────────────────────
    require!(params.reserve_fee_bps <= MAX_RESERVE_FEE_BPS, TrancheError::InvalidFee);
    require!(params.senior_exit_fee_bps <= MAX_EXIT_FEE_BPS, TrancheError::InvalidFee);
    require!(params.junior_exit_fee_bps <= MAX_EXIT_FEE_BPS, TrancheError::InvalidFee);
    require!(
        decimal_bits_to_u32_if_integer(params.risk_k).is_some(),
        TrancheError::InvalidRiskConfig
    );

    // ── Initialize TrancheConfig ────────────────────────────────────
    let mut config = ctx.accounts.tranche_config.load_init()?;

    // Identity
    config.vault = ctx.accounts.vault.key();
    config.authority = ctx.accounts.authority.key();
    config.pending_authority = Pubkey::default();

    // Tranche mints
    config.senior_mint = ctx.accounts.senior_mint.key();
    config.junior_mint = ctx.accounts.junior_mint.key();

    // Voltr integration
    config.lp_custody = ctx.accounts.vault_lp_custody.key();
    config.asset_mint = ctx.accounts.asset_mint.key();

    // NAV tracking — all zero at init (already zeroed by account init)

    // Senior target index = Decimal::ONE
    let one_bits: u128 = Decimal::ONE.to_decimal_bits();
    config.senior_target_index = one_bits.to_le_bytes();
    config.index_timestamp = Clock::get()?.unix_timestamp;

    // APR parameters
    config.apr_floor_bps = params.apr_floor_bps;
    config.risk_x_bps = params.risk_x_bps;
    config.risk_y_bps = params.risk_y_bps;
    config.risk_k = params.risk_k;
    config.base_apr_bps = 0;

    // Fee parameters
    config.reserve_fee_bps = params.reserve_fee_bps;
    config.senior_exit_fee_bps = params.senior_exit_fee_bps;
    config.junior_exit_fee_bps = params.junior_exit_fee_bps;

    // Coverage thresholds
    config.min_jr_sr_ratio_bps = params.min_jr_sr_ratio_bps;
    config.min_coverage_for_mint_bps = params.min_coverage_for_mint_bps;
    config.min_coverage_buffer_bps = params.min_coverage_buffer_bps;

    // Withdrawal config
    config.senior_cooldown_seconds = params.senior_cooldown_seconds;
    config.junior_cooldown_seconds = params.junior_cooldown_seconds;

    // APR feed config
    config.max_observation_age_seconds = params.max_observation_age_seconds;

    // Operational
    config.paused = 0;
    config.next_receipt_id = 0;

    // PDA bumps
    config.tranche_config_bump = ctx.bumps.tranche_config;
    config.tranche_authority_bump = ctx.bumps.tranche_authority;
    config.senior_mint_bump = ctx.bumps.senior_mint;
    config.junior_mint_bump = ctx.bumps.junior_mint;
    config.apr_feed_bump = ctx.bumps.apr_feed;

    Ok(())
}
