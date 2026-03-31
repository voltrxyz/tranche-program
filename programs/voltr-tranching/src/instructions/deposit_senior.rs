use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, MintTo, Token, TokenAccount, Transfer};

use crate::constants::*;
use crate::error::TrancheError;
use crate::events::DepositSeniorEvent;
use crate::instructions::crank_accounting::{read_decimal, run_inline_crank, write_decimal};
use crate::math::{Decimal, DecimalExtra, SafeMath, BPS_PRECISION};
use crate::state::TrancheConfig;


#[derive(Accounts)]
pub struct DepositSenior<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        seeds = [TRANCHE_CONFIG_SEED, vault.key().as_ref()],
        bump,
    )]
    pub tranche_config: AccountLoader<'info, TrancheConfig>,

    /// CHECK: PDA used as CPI signer for Voltr Vault deposit and token mint;
    /// validated by seeds constraint.
    #[account(
        seeds = [TRANCHE_AUTHORITY_SEED, tranche_config.key().as_ref()],
        bump,
    )]
    pub tranche_authority: UncheckedAccount<'info>,

    /// Senior tranche token mint (mutable for mint_to).
    #[account(mut)]
    pub senior_mint: Account<'info, Mint>,

    /// Junior tranche token mint (read-only, needed for inline crank auto-pause check).
    pub junior_mint: Account<'info, Mint>,

    /// User's asset token account (source of deposit).
    #[account(
        mut,
        token::mint = asset_mint,
        token::authority = user,
    )]
    pub user_asset_ata: Account<'info, TokenAccount>,

    /// User's senior tranche token account (receives minted tokens).
    #[account(
        mut,
        token::mint = senior_mint,
        token::authority = user,
    )]
    pub user_senior_ata: Account<'info, TokenAccount>,

    /// Asset escrow owned by tranche_authority (intermediate for CPI deposit).
    #[account(
        mut,
        token::mint = asset_mint,
        token::authority = tranche_authority,
    )]
    pub asset_escrow: Account<'info, TokenAccount>,

    /// LP token custody owned by tranche_authority (receives LP from vault).
    #[account(mut)]
    pub vault_lp_custody: Account<'info, TokenAccount>,

    /// Underlying asset mint.
    pub asset_mint: Account<'info, Mint>,

    // ── Voltr Vault accounts (for inline crank + CPI) ────────────────────

    /// The underlying Voltr Vault.
    #[account(mut)]
    pub vault: AccountLoader<'info, voltr_vault::state::Vault>,

    /// Vault LP token mint (supply needed for inline crank; mutable for CPI).
    #[account(mut)]
    pub vault_lp_mint: Account<'info, Mint>,

    /// CHECK: Voltr protocol state PDA; validated by Voltr Vault program during CPI.
    pub protocol: UncheckedAccount<'info>,

    /// CHECK: Vault's idle asset ATA; validated by Voltr Vault program during CPI.
    #[account(mut)]
    pub vault_asset_idle_ata: UncheckedAccount<'info>,

    /// CHECK: Vault asset idle auth PDA; validated by Voltr Vault program during CPI.
    pub vault_asset_idle_auth: UncheckedAccount<'info>,

    /// CHECK: Vault LP mint auth PDA; validated by Voltr Vault program during CPI.
    pub vault_lp_mint_auth: UncheckedAccount<'info>,

    // ── Programs ─────────────────────────────────────────────────────────

    pub voltr_vault_program: Program<'info, voltr_vault::program::VoltrVault>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<DepositSenior>, amount: u64) -> Result<()> {
    // ── 1. Validate inputs ───────────────────────────────────────────────
    require!(amount > 0, TrancheError::ZeroAmount);

    // ── 2. Read vault state for inline crank ─────────────────────────────
    let vault = ctx.accounts.vault.load()?;
    let vault_total_value = vault.get_total_asset_value();
    let vault_lp_supply =
        vault.get_total_lp_supply_incl_fees(ctx.accounts.vault_lp_mint.supply)?;
    require!(
        vault.lp.mint == ctx.accounts.vault_lp_mint.key(),
        TrancheError::InvalidVault
    );
    drop(vault);

    let lp_balance_before = ctx.accounts.vault_lp_custody.amount;

    // ── 3. Load config and validate accounts ─────────────────────────────
    let mut config = ctx.accounts.tranche_config.load_mut()?;

    require!(config.paused & 0x01 == 0, TrancheError::SeniorDepositsPaused);
    require!(
        ctx.accounts.vault_lp_custody.key() == config.lp_custody,
        TrancheError::InvalidVault
    );
    require!(
        ctx.accounts.senior_mint.key() == config.senior_mint,
        TrancheError::InvalidVault
    );
    require!(
        ctx.accounts.junior_mint.key() == config.junior_mint,
        TrancheError::InvalidVault
    );
    require!(
        ctx.accounts.asset_mint.key() == config.asset_mint,
        TrancheError::InvalidVault
    );

    let now = Clock::get()?.unix_timestamp;
    let junior_supply = ctx.accounts.junior_mint.supply;

    // ── 4. Inline crank_accounting — sync NAV before pricing ─────────────
    let crank_result = run_inline_crank(
        &mut config,
        vault_total_value,
        vault_lp_supply,
        lp_balance_before,
        junior_supply,
        now,
    )?;

    // Re-check pause after crank: auto-pause on junior wipeout may have
    // set the senior deposit pause bit during the waterfall above.
    require!(config.paused & 0x01 == 0, TrancheError::SeniorDepositsPaused);

    // ── 5. Pre-CPI coverage check (conservative fast-fail using `amount`) ─
    // Uses `amount` (>= effective_deposit) so this is a lower bound on
    // the post-deposit coverage ratio; will never false-reject.
    let deposit_decimal = Decimal::from_num(amount);
    let post_total = crank_result.total_nav.safe_add(deposit_decimal)?;
    let post_senior = crank_result.senior_nav.safe_add(deposit_decimal)?;

    let lhs = post_total.safe_mul(Decimal::from_num(BPS_PRECISION))?;
    let rhs = post_senior.safe_mul(Decimal::from_num(config.min_coverage_buffer_bps))?;
    require!(lhs >= rhs, TrancheError::InsufficientCoverage);

    // Save values needed after CPI before dropping config
    let tranche_authority_bump = config.tranche_authority_bump;
    let senior_nav_before = crank_result.senior_nav;
    let senior_supply = ctx.accounts.senior_mint.supply;

    drop(config);

    let tranche_config_key = ctx.accounts.tranche_config.key();

    // ── 6. Transfer assets: user → asset_escrow ──────────────────────────
    token::transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.user_asset_ata.to_account_info(),
                to: ctx.accounts.asset_escrow.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        ),
        amount,
    )?;

    // ── 7. CPI: deposit_vault — escrow → vault ──────────────────────────
    let authority_seeds: &[&[u8]] = &[
        TRANCHE_AUTHORITY_SEED,
        tranche_config_key.as_ref(),
        &[tranche_authority_bump],
    ];

    let cpi_accounts = voltr_vault::cpi::accounts::DepositVault {
        user_transfer_authority: ctx.accounts.tranche_authority.to_account_info(),
        protocol: ctx.accounts.protocol.to_account_info(),
        vault: ctx.accounts.vault.to_account_info(),
        vault_asset_mint: ctx.accounts.asset_mint.to_account_info(),
        vault_lp_mint: ctx.accounts.vault_lp_mint.to_account_info(),
        user_asset_ata: ctx.accounts.asset_escrow.to_account_info(),
        vault_asset_idle_ata: ctx.accounts.vault_asset_idle_ata.to_account_info(),
        vault_asset_idle_auth: ctx.accounts.vault_asset_idle_auth.to_account_info(),
        user_lp_ata: ctx.accounts.vault_lp_custody.to_account_info(),
        vault_lp_mint_auth: ctx.accounts.vault_lp_mint_auth.to_account_info(),
        asset_token_program: ctx.accounts.token_program.to_account_info(),
        lp_token_program: ctx.accounts.token_program.to_account_info(),
        system_program: ctx.accounts.system_program.to_account_info(),
    };

    voltr_vault::cpi::deposit_vault(
        CpiContext::new_with_signer(
            ctx.accounts.voltr_vault_program.to_account_info(),
            cpi_accounts,
            &[authority_seeds],
        ),
        amount,
    )?;

    // ── 8. Compute effective deposit from actual LP delta ────────────────
    // The vault may have taken an issuance fee / dead-weight, so the LP
    // tokens we received represent less value than `amount`.  Use pre-CPI
    // vault pricing to convert the LP delta back into asset terms.
    ctx.accounts.vault_lp_custody.reload()?;
    let lp_gained = ctx
        .accounts
        .vault_lp_custody
        .amount
        .checked_sub(lp_balance_before)
        .ok_or(error!(TrancheError::MathOverflow))?;

    let effective_deposit = if vault_lp_supply == 0 {
        // First vault deposit — no existing pricing reference
        Decimal::from_num(amount)
    } else {
        Decimal::from_num(lp_gained).full_mul_int_ratio(vault_total_value, vault_lp_supply)
    };

    // ── 9. Calculate senior tokens to mint ───────────────────────────────
    let tokens_to_mint = if senior_supply == 0 {
        // First deposit: 1:1 with effective value
        effective_deposit.to_floor::<u64>()
    } else {
        // tokens = effective_deposit * senior_supply / senior_nav (round down)
        let ratio = Decimal::from_num(senior_supply).safe_div(senior_nav_before)?;
        effective_deposit.safe_mul(ratio)?.to_floor::<u64>()
    };

    // Reject dust deposits that round to zero shares
    require!(tokens_to_mint > 0, TrancheError::ZeroAmount);

    // ── 10. Mint senior tokens to user ───────────────────────────────────
    token::mint_to(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            MintTo {
                mint: ctx.accounts.senior_mint.to_account_info(),
                to: ctx.accounts.user_senior_ata.to_account_info(),
                authority: ctx.accounts.tranche_authority.to_account_info(),
            },
            &[authority_seeds],
        ),
        tokens_to_mint,
    )?;

    // ── 11. Update NAVs with effective deposit ───────────────────────────
    let mut config = ctx.accounts.tranche_config.load_mut()?;
    let new_senior_nav = read_decimal(&config.senior_nav).safe_add(effective_deposit)?;
    let new_total_nav = read_decimal(&config.total_nav).safe_add(effective_deposit)?;
    config.senior_nav = write_decimal(new_senior_nav);
    config.total_nav = write_decimal(new_total_nav);
    drop(config);

    // ── 12. Emit event ───────────────────────────────────────────────────
    let coverage_ratio = if new_senior_nav > Decimal::ZERO {
        new_total_nav
            .safe_div(new_senior_nav)?
            .to_decimal_bits()
    } else {
        0u128
    };

    emit!(DepositSeniorEvent {
        user: ctx.accounts.user.key(),
        asset_amount: amount,
        tokens_minted: tokens_to_mint,
        senior_nav: new_senior_nav.to_decimal_bits(),
        coverage_ratio,
    });

    Ok(())
}
