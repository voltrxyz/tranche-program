use anchor_lang::prelude::*;
use anchor_spl::token::{self, Burn, Mint, Token, TokenAccount, Transfer};

use crate::constants::*;
use crate::error::TrancheError;
use crate::events::InstantWithdrawEvent;
use crate::instructions::crank_accounting::{read_decimal, run_inline_crank, write_decimal};
use crate::math::{Decimal, DecimalExtra, SafeMath, BPS_PRECISION};
use crate::state::TrancheConfig;

#[derive(Accounts)]
pub struct InstantWithdrawSenior<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        seeds = [TRANCHE_CONFIG_SEED, vault.key().as_ref()],
        bump,
    )]
    pub tranche_config: AccountLoader<'info, TrancheConfig>,

    /// CHECK: PDA used as CPI signer for Voltr Vault withdrawal and asset transfer;
    /// validated by seeds constraint.
    #[account(
        seeds = [TRANCHE_AUTHORITY_SEED, tranche_config.key().as_ref()],
        bump,
    )]
    pub tranche_authority: UncheckedAccount<'info>,

    /// Senior tranche token mint (mutable for burn).
    #[account(mut)]
    pub senior_mint: Account<'info, Mint>,

    /// Junior tranche token mint (read-only, needed for inline crank auto-pause check).
    pub junior_mint: Account<'info, Mint>,

    /// User's senior tranche token account (source of tokens to burn).
    #[account(
        mut,
        token::mint = senior_mint,
        token::authority = user,
    )]
    pub user_senior_ata: Account<'info, TokenAccount>,

    /// User's asset token account (receives withdrawn assets).
    #[account(
        mut,
        token::authority = user,
    )]
    pub user_asset_ata: Account<'info, TokenAccount>,

    /// Asset escrow owned by tranche_authority (receives assets from vault CPI,
    /// then transfers to user).
    #[account(
        mut,
        token::authority = tranche_authority,
    )]
    pub asset_escrow: Account<'info, TokenAccount>,

    /// LP token custody owned by tranche_authority (LP tokens burned via CPI).
    #[account(mut)]
    pub vault_lp_custody: Account<'info, TokenAccount>,

    /// Underlying asset mint.
    pub asset_mint: Account<'info, Mint>,

    // -- Voltr Vault accounts (for inline crank + CPI) ----------------------

    /// The underlying Voltr Vault.
    #[account(mut)]
    pub vault: AccountLoader<'info, voltr_vault::state::Vault>,

    /// Vault LP token mint (supply needed for inline crank; mutable for CPI burn).
    #[account(mut)]
    pub vault_lp_mint: Account<'info, Mint>,

    /// CHECK: Voltr protocol state PDA; validated by Voltr Vault program during CPI.
    pub protocol: UncheckedAccount<'info>,

    /// CHECK: Vault's idle asset ATA; validated by Voltr Vault program during CPI.
    #[account(mut)]
    pub vault_asset_idle_ata: UncheckedAccount<'info>,

    /// CHECK: Vault asset idle auth PDA; validated by Voltr Vault program during CPI.
    #[account(mut)]
    pub vault_asset_idle_auth: UncheckedAccount<'info>,

    // -- Programs -----------------------------------------------------------

    pub voltr_vault_program: Program<'info, voltr_vault::program::VoltrVault>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn handler(
    ctx: Context<InstantWithdrawSenior>,
    amount: u64,
    is_token_amount: bool,
) -> Result<()> {
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

    let lp_balance = ctx.accounts.vault_lp_custody.amount;

    // ── 3. Load config and validate ──────────────────────────────────────
    let mut config = ctx.accounts.tranche_config.load_mut()?;

    // Instant withdraw requires cooldown == 0
    require!(
        config.senior_cooldown_seconds == 0,
        TrancheError::CooldownMustBeZero
    );
    require!(
        config.paused & 0x04 == 0,
        TrancheError::SeniorWithdrawalsPaused
    );
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
        lp_balance,
        junior_supply,
        now,
    )?;

    // Re-check pause after crank (auto-pause may have been set)
    require!(
        config.paused & 0x04 == 0,
        TrancheError::SeniorWithdrawalsPaused
    );

    // ── 5. Calculate token amount and asset value ────────────────────────
    let senior_supply = ctx.accounts.senior_mint.supply;
    let senior_nav = crank_result.senior_nav;

    require!(senior_supply > 0, TrancheError::ZeroAmount);
    require!(senior_nav > Decimal::ZERO, TrancheError::ZeroAmount);

    let (token_amount, asset_value) = if is_token_amount {
        let asset_val = Decimal::from_num(amount).full_mul_int_ratio(
            senior_nav.to_decimal_bits(),
            u128::from(senior_supply),
        );
        (amount, asset_val)
    } else {
        let tokens = Decimal::from_num(amount).full_mul_int_ratio(
            u128::from(senior_supply),
            senior_nav.to_decimal_bits(),
        );
        let token_amt: u64 = tokens.to_ceil(); // round up tokens burned (protocol-favoring)
        let asset_val = Decimal::from_num(token_amt).full_mul_int_ratio(
            senior_nav.to_decimal_bits(),
            u128::from(senior_supply),
        );
        (token_amt, asset_val)
    };

    require!(token_amount > 0, TrancheError::ZeroAmount);

    // ── 6. Apply exit fee (rounded up — protocol-favoring) ───────────────
    let fee = asset_value
        .safe_mul(Decimal::from_num(config.senior_exit_fee_bps))?
        .safe_div_ceil(Decimal::from_num(BPS_PRECISION))?;
    let net_asset = asset_value.safe_sub(fee)?;

    // Transfer fee from senior NAV to reserve NAV (internal accounting)
    let new_senior_nav = read_decimal(&config.senior_nav).safe_sub(fee)?;
    config.senior_nav = write_decimal(new_senior_nav);
    let new_reserve_nav = read_decimal(&config.reserve_nav).safe_add(fee)?;
    config.reserve_nav = write_decimal(new_reserve_nav);

    let net_asset_u64: u64 = net_asset.to_floor(); // round down (protocol-favoring)
    require!(net_asset_u64 > 0, TrancheError::ZeroAmount);

    // Save values needed after dropping config
    let tranche_authority_bump = config.tranche_authority_bump;
    drop(config);

    let tranche_config_key = ctx.accounts.tranche_config.key();
    let authority_seeds: &[&[u8]] = &[
        TRANCHE_AUTHORITY_SEED,
        tranche_config_key.as_ref(),
        &[tranche_authority_bump],
    ];

    // ── 7. CPI: instant_withdraw_vault — redeem LP for assets ────────────
    let cpi_accounts = voltr_vault::cpi::accounts::InstantWithdrawVault {
        user_transfer_authority: ctx.accounts.tranche_authority.to_account_info(),
        protocol: ctx.accounts.protocol.to_account_info(),
        vault: ctx.accounts.vault.to_account_info(),
        vault_asset_mint: ctx.accounts.asset_mint.to_account_info(),
        vault_lp_mint: ctx.accounts.vault_lp_mint.to_account_info(),
        user_lp_ata: ctx.accounts.vault_lp_custody.to_account_info(),
        vault_asset_idle_ata: ctx.accounts.vault_asset_idle_ata.to_account_info(),
        vault_asset_idle_auth: ctx.accounts.vault_asset_idle_auth.to_account_info(),
        user_asset_ata: ctx.accounts.asset_escrow.to_account_info(),
        asset_token_program: ctx.accounts.token_program.to_account_info(),
        lp_token_program: ctx.accounts.token_program.to_account_info(),
        system_program: ctx.accounts.system_program.to_account_info(),
    };

    voltr_vault::cpi::instant_withdraw_vault(
        CpiContext::new_with_signer(
            ctx.accounts.voltr_vault_program.to_account_info(),
            cpi_accounts,
            &[authority_seeds],
        ),
        net_asset_u64,
        false, // is_amount_in_lp = false (amount is in asset terms)
        false, // is_withdraw_all = false
    )?;

    // ── 7b. Measure actual LP burned ─────────────────────────────────────
    ctx.accounts.vault_lp_custody.reload()?;
    let lp_after = ctx.accounts.vault_lp_custody.amount;
    let lp_burned: u64 = lp_balance
        .checked_sub(lp_after)
        .ok_or(error!(TrancheError::MathOverflow))?;

    let actual_value_lost =
        Decimal::from_num(lp_burned).full_mul_int_ratio(vault_total_value, vault_lp_supply);

    // ── 8. Transfer: asset_escrow → user ─────────────────────────────────
    token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.asset_escrow.to_account_info(),
                to: ctx.accounts.user_asset_ata.to_account_info(),
                authority: ctx.accounts.tranche_authority.to_account_info(),
            },
            &[authority_seeds],
        ),
        net_asset_u64,
    )?;

    // ── 9. Burn senior tranche tokens directly from user ─────────────────
    token::burn(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Burn {
                mint: ctx.accounts.senior_mint.to_account_info(),
                from: ctx.accounts.user_senior_ata.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        ),
        token_amount,
    )?;

    // ── 10. Update NAVs ──────────────────────────────────────────────────
    let mut config = ctx.accounts.tranche_config.load_mut()?;

    // Use actual_value_lost (not net_asset) to account for Voltr redemption fee
    let updated_senior_nav = read_decimal(&config.senior_nav).safe_sub(actual_value_lost)?;
    config.senior_nav = write_decimal(updated_senior_nav);

    let new_total_nav = read_decimal(&config.total_nav).safe_sub(actual_value_lost)?;
    config.total_nav = write_decimal(new_total_nav);

    drop(config);

    // ── 11. Emit event ───────────────────────────────────────────────────
    emit!(InstantWithdrawEvent {
        user: ctx.accounts.user.key(),
        is_senior: true,
        asset_amount: net_asset_u64,
        tokens_burned: token_amount,
    });

    Ok(())
}
