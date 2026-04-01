use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

use crate::constants::*;
use crate::error::TrancheError;
use crate::events::WithdrawReserveEvent;
use crate::instructions::crank_accounting::{read_decimal, run_inline_crank, write_decimal};
use crate::math::{Decimal, DecimalExtra, SafeMath, BPS_PRECISION};
use crate::state::TrancheConfig;

#[derive(Accounts)]
pub struct WithdrawReserve<'info> {
    pub authority: Signer<'info>,

    #[account(
        mut,
        has_one = authority,
        seeds = [TRANCHE_CONFIG_SEED, vault.key().as_ref()],
        bump,
    )]
    pub tranche_config: AccountLoader<'info, TrancheConfig>,

    /// CHECK: PDA used as CPI signer for Voltr Vault withdrawal and token transfer;
    /// validated by seeds constraint.
    #[account(
        seeds = [TRANCHE_AUTHORITY_SEED, tranche_config.key().as_ref()],
        bump,
    )]
    pub tranche_authority: UncheckedAccount<'info>,

    /// Asset escrow owned by tranche_authority (receives assets from vault CPI,
    /// then transfers to authority destination).
    #[account(
        mut,
        token::authority = tranche_authority,
    )]
    pub asset_escrow: Account<'info, TokenAccount>,

    /// Authority's destination ATA for receiving withdrawn reserve assets.
    #[account(
        mut,
        token::authority = authority,
    )]
    pub destination_ata: Account<'info, TokenAccount>,

    /// LP token custody owned by tranche_authority (LP tokens burned via CPI).
    #[account(mut)]
    pub vault_lp_custody: Account<'info, TokenAccount>,

    /// Underlying asset mint.
    pub asset_mint: Account<'info, Mint>,

    /// Junior tranche token mint (needed for inline crank auto-pause check).
    pub junior_mint: Account<'info, Mint>,

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

pub fn handler(ctx: Context<WithdrawReserve>, amount: u64) -> Result<()> {
    require!(amount > 0, TrancheError::ZeroAmount);

    // -- 1. Read vault state for inline crank --------------------------------
    let vault = ctx.accounts.vault.load()?;
    let vault_total_value = vault.get_total_asset_value();
    let vault_lp_supply =
        vault.get_total_lp_supply_incl_fees(ctx.accounts.vault_lp_mint.supply)?;
    require!(
        vault.lp.mint == ctx.accounts.vault_lp_mint.key(),
        TrancheError::InvalidVault
    );
    let vault_redemption_fee_bps = vault.fee_configuration.redemption_fee;
    drop(vault);

    let lp_balance = ctx.accounts.vault_lp_custody.amount;
    let now = Clock::get()?.unix_timestamp;

    // -- 2. Load config and validate accounts --------------------------------
    let mut config = ctx.accounts.tranche_config.load_mut()?;
    require!(
        ctx.accounts.vault_lp_custody.key() == config.lp_custody,
        TrancheError::InvalidVault
    );
    require!(
        ctx.accounts.asset_mint.key() == config.asset_mint,
        TrancheError::InvalidVault
    );

    let junior_supply = ctx.accounts.junior_mint.supply;

    // -- 3. Inline crank_accounting -- sync NAV before pricing ---------------
    let _crank_result = run_inline_crank(
        &mut config,
        vault_total_value,
        vault_lp_supply,
        lp_balance,
        junior_supply,
        now,
    )?;

    // -- 4. Calculate withdrawal amount ----------------------------------------
    // In a shared vault with redemption fees, the LP position delta (actual_value_lost)
    // exceeds the cash withdrawn because the fee portion is retained for other LP
    // holders. Cap the withdrawal so the worst-case position delta stays within
    // reserve_nav, ensuring the full cost is charged to reserve — not leaked to
    // junior/senior via a later crank.
    let reserve_nav = read_decimal(&config.reserve_nav);
    let total_nav = read_decimal(&config.total_nav);

    require!(reserve_nav > Decimal::ZERO, TrancheError::ZeroAmount);
    require!(total_nav > Decimal::ZERO, TrancheError::ZeroAmount);

    // max_withdrawable = reserve_nav * (BPS - redemption_fee_bps) / BPS
    // This ensures: withdraw_amount * BPS / (BPS - fee) <= reserve_nav
    let max_withdrawable = if vault_redemption_fee_bps > 0 {
        let net_bps = BPS_PRECISION.saturating_sub(vault_redemption_fee_bps);
        require!(net_bps > 0, TrancheError::MathOverflow);
        reserve_nav.mul_int_ratio(u128::from(net_bps), u128::from(BPS_PRECISION))
    } else {
        reserve_nav
    };

    let withdraw_amount_decimal = Decimal::from_num(amount);
    let actual_withdraw = if withdraw_amount_decimal >= max_withdrawable {
        max_withdrawable
    } else {
        withdraw_amount_decimal
    };

    let withdraw_u64: u64 = actual_withdraw.to_floor(); // round down (protocol-favoring)
    require!(withdraw_u64 > 0, TrancheError::ZeroAmount);

    // -- 5. Save values needed after dropping config -------------------------
    let tranche_authority_bump = config.tranche_authority_bump;
    drop(config);

    let tranche_config_key = ctx.accounts.tranche_config.key();
    let authority_seeds: &[&[u8]] = &[
        TRANCHE_AUTHORITY_SEED,
        tranche_config_key.as_ref(),
        &[tranche_authority_bump],
    ];

    // -- 6. CPI: instant_withdraw_vault -- redeem LP for assets --------------
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
        withdraw_u64,
        false, // is_amount_in_lp = false (amount is in asset terms)
        false, // is_withdraw_all = false
    )?;

    // -- 6b. Measure actual LP burned using post-CPI vault state -------------
    // Reload after CPI so pricing reflects any management fee accrued inside
    // the vault's `handle_management_fee()` call.
    ctx.accounts.vault_lp_custody.reload()?;
    ctx.accounts.vault_lp_mint.reload()?;

    let lp_after = ctx.accounts.vault_lp_custody.amount;
    let _lp_burned: u64 = lp_balance
        .checked_sub(lp_after)
        .ok_or(error!(TrancheError::MathOverflow))?;

    let vault_post = ctx.accounts.vault.load()?;
    let post_vault_total_value = vault_post.get_total_asset_value();
    let post_vault_lp_supply =
        vault_post.get_total_lp_supply_incl_fees(ctx.accounts.vault_lp_mint.supply)?;
    drop(vault_post);

    // Measure the change in LP position value (pre minus post).
    // This correctly excludes the vault's redemption fee, which burns extra LP
    // but keeps the corresponding assets in the vault for remaining LP holders.
    let pre_position_value = Decimal::from_num(lp_balance)
        .full_mul_int_ratio(vault_total_value, vault_lp_supply);
    let post_position_value = if lp_after == 0 {
        Decimal::ZERO
    } else {
        Decimal::from_num(lp_after)
            .full_mul_int_ratio(post_vault_total_value, post_vault_lp_supply)
    };
    let actual_value_lost = pre_position_value.safe_sub(post_position_value)?;

    // -- 7. Transfer: asset_escrow -> authority destination -------------------
    token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.asset_escrow.to_account_info(),
                to: ctx.accounts.destination_ata.to_account_info(),
                authority: ctx.accounts.tranche_authority.to_account_info(),
            },
            &[authority_seeds],
        ),
        withdraw_u64,
    )?;

    // -- 8. Update NAVs ------------------------------------------------------
    // Deduct actual_value_lost (the full LP position delta) from reserve and total.
    // Because we capped the withdrawal amount pre-CPI so that the worst-case
    // position delta stays within reserve_nav, actual_value_lost <= reserve_nav
    // and this subtraction is safe.
    let mut config = ctx.accounts.tranche_config.load_mut()?;
    let new_reserve_nav = read_decimal(&config.reserve_nav).safe_sub(actual_value_lost)?;
    config.reserve_nav = write_decimal(new_reserve_nav);

    let new_total_nav = read_decimal(&config.total_nav).safe_sub(actual_value_lost)?;
    config.total_nav = write_decimal(new_total_nav);

    drop(config);

    // -- 9. Emit event -------------------------------------------------------
    emit!(WithdrawReserveEvent {
        authority: ctx.accounts.authority.key(),
        amount_withdrawn: withdraw_u64,
        reserve_nav_after: new_reserve_nav.to_decimal_bits(),
        total_nav_after: new_total_nav.to_decimal_bits(),
    });

    Ok(())
}
