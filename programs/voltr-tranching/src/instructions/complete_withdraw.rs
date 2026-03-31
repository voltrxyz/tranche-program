use anchor_lang::prelude::*;
use anchor_spl::token::{self, Burn, Mint, Token, TokenAccount, Transfer};

use crate::constants::*;
use crate::error::TrancheError;
use crate::events::CompleteWithdrawEvent;
use crate::instructions::crank_accounting::{read_decimal, run_inline_crank, write_decimal};
use crate::math::{Decimal, DecimalExtra, SafeMath, BPS_PRECISION};
use crate::state::{TrancheConfig, WithdrawReceipt};

#[derive(Accounts)]
pub struct CompleteWithdraw<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        seeds = [TRANCHE_CONFIG_SEED, vault.key().as_ref()],
        bump,
    )]
    pub tranche_config: AccountLoader<'info, TrancheConfig>,

    /// CHECK: PDA used as CPI signer for Voltr Vault withdrawal, token burn, and transfer;
    /// validated by seeds constraint.
    #[account(
        seeds = [TRANCHE_AUTHORITY_SEED, tranche_config.key().as_ref()],
        bump,
    )]
    pub tranche_authority: UncheckedAccount<'info>,

    /// Senior tranche token mint (mutable for burn if senior withdrawal).
    #[account(mut)]
    pub senior_mint: Account<'info, Mint>,

    /// Junior tranche token mint (mutable for burn if junior withdrawal;
    /// also needed for inline crank auto-pause check).
    #[account(mut)]
    pub junior_mint: Account<'info, Mint>,

    /// Tranche token escrow holding the escrowed tokens (senior or junior,
    /// depending on the receipt). Owned by tranche_authority.
    #[account(mut)]
    pub tranche_token_escrow: Account<'info, TokenAccount>,

    /// Asset escrow owned by tranche_authority (receives assets from vault CPI,
    /// then transfers to user).
    #[account(
        mut,
        token::authority = tranche_authority,
    )]
    pub asset_escrow: Account<'info, TokenAccount>,

    /// User's asset token account (receives final withdrawn assets).
    #[account(
        mut,
        token::authority = user,
    )]
    pub user_asset_ata: Account<'info, TokenAccount>,

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

    // -- WithdrawReceipt (close on completion) ------------------------------

    /// The withdraw receipt to complete. Closed atomically (rent -> user).
    #[account(
        mut,
        has_one = user,
        has_one = tranche_config,
        close = user,
    )]
    pub withdraw_receipt: Account<'info, WithdrawReceipt>,

    // -- Programs -----------------------------------------------------------

    pub voltr_vault_program: Program<'info, voltr_vault::program::VoltrVault>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<CompleteWithdraw>) -> Result<()> {
    // -- 1. Validate cooldown elapsed ----------------------------------------
    let now = Clock::get()?.unix_timestamp;
    let receipt = &ctx.accounts.withdraw_receipt;
    require!(
        now >= receipt.withdrawable_from_ts,
        TrancheError::CooldownNotElapsed
    );

    let is_senior = receipt.is_senior;
    let tokens_escrowed = receipt.tranche_tokens_escrowed;
    let asset_amount_at_request = receipt.asset_amount_at_request;
    let receipt_id = receipt.receipt_id;

    // -- 2. Read vault state for inline crank --------------------------------
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

    // -- 3. Load config and validate accounts --------------------------------
    let mut config = ctx.accounts.tranche_config.load_mut()?;
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

    // Validate tranche_token_escrow matches the correct mint
    let expected_escrow_mint = if is_senior {
        config.senior_mint
    } else {
        config.junior_mint
    };
    require!(
        ctx.accounts.tranche_token_escrow.mint == expected_escrow_mint,
        TrancheError::InvalidVault
    );
    require!(
        ctx.accounts.tranche_token_escrow.owner == ctx.accounts.tranche_authority.key(),
        TrancheError::InvalidVault
    );

    let junior_supply = ctx.accounts.junior_mint.supply;

    // -- 4. Inline crank_accounting -- sync NAV before pricing ---------------
    let crank_result = run_inline_crank(
        &mut config,
        vault_total_value,
        vault_lp_supply,
        lp_balance,
        junior_supply,
        now,
    )?;

    // -- 5. Recalculate current value and apply value cap --------------------
    let (tranche_nav, tranche_supply) = if is_senior {
        (crank_result.senior_nav, ctx.accounts.senior_mint.supply)
    } else {
        (crank_result.junior_nav, ctx.accounts.junior_mint.supply)
    };

    // Guard against zero supply/NAV (should not happen if receipt exists, but be safe)
    require!(tranche_supply > 0, TrancheError::ZeroAmount);
    require!(tranche_nav > Decimal::ZERO, TrancheError::ZeroAmount);

    let current_value = Decimal::from_num(tokens_escrowed).full_mul_int_ratio(
        tranche_nav.to_decimal_bits(),
        u128::from(tranche_supply),
    );

    // Value cap: min(current_value, asset_amount_at_request)
    // Prevents gaming the cooldown period for yield
    let request_value = Decimal::from_decimal_bits(asset_amount_at_request);
    let final_value = if current_value <= request_value {
        current_value
    } else {
        request_value
    };

    let final_value_u64: u64 = final_value.to_floor(); // round down (protocol-favoring)
    require!(final_value_u64 > 0, TrancheError::ZeroAmount);

    // Save values needed after dropping config
    let tranche_authority_bump = config.tranche_authority_bump;
    let min_jr_sr_ratio_bps = config.min_jr_sr_ratio_bps;
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
        final_value_u64,
        false, // is_amount_in_lp = false (amount is in asset terms)
        false, // is_withdraw_all = false
    )?;

    // -- 6b. Measure actual LP burned (may exceed proportional due to Voltr fee)
    ctx.accounts.vault_lp_custody.reload()?;
    let lp_after = ctx.accounts.vault_lp_custody.amount;
    let lp_burned: u64 = lp_balance
        .checked_sub(lp_after)
        .ok_or(error!(TrancheError::MathOverflow))?;

    // Actual value lost from our LP position (uses pre-CPI vault state)
    let actual_value_lost =
        Decimal::from_num(lp_burned).full_mul_int_ratio(vault_total_value, vault_lp_supply);

    // -- 7. Transfer: asset_escrow -> user -----------------------------------
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
        final_value_u64,
    )?;

    // -- 8. Burn escrowed tranche tokens -------------------------------------
    let burn_mint = if is_senior {
        ctx.accounts.senior_mint.to_account_info()
    } else {
        ctx.accounts.junior_mint.to_account_info()
    };

    token::burn(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Burn {
                mint: burn_mint,
                from: ctx.accounts.tranche_token_escrow.to_account_info(),
                authority: ctx.accounts.tranche_authority.to_account_info(),
            },
            &[authority_seeds],
        ),
        tokens_escrowed,
    )?;

    // -- 9. Update NAVs ------------------------------------------------------
    let mut config = ctx.accounts.tranche_config.load_mut()?;

    // Use actual_value_lost (not final_value) to account for Voltr redemption fee.
    // The user receives final_value assets, but the LP cost may be higher due to
    // the vault's redemption fee. Remaining tranche holders must not absorb that cost.
    if is_senior {
        let new_senior_nav = read_decimal(&config.senior_nav).safe_sub(actual_value_lost)?;
        config.senior_nav = write_decimal(new_senior_nav);
    } else {
        let new_junior_nav = read_decimal(&config.junior_nav).safe_sub(actual_value_lost)?;
        config.junior_nav = write_decimal(new_junior_nav);
    }

    let new_total_nav = read_decimal(&config.total_nav).safe_sub(actual_value_lost)?;
    config.total_nav = write_decimal(new_total_nav);

    // -- 10. Junior ratio check (for junior withdrawals) ---------------------
    if !is_senior {
        let post_junior_nav = read_decimal(&config.junior_nav);
        let post_senior_nav = read_decimal(&config.senior_nav);
        let min_junior_required = post_senior_nav.mul_int_ratio(
            u128::from(min_jr_sr_ratio_bps),
            u128::from(BPS_PRECISION),
        );
        require!(
            post_junior_nav >= min_junior_required,
            TrancheError::JuniorRatioBreach
        );
    }

    drop(config);

    // -- 11. Emit event (receipt closed by Anchor `close` constraint) --------
    emit!(CompleteWithdrawEvent {
        user: ctx.accounts.user.key(),
        receipt_id,
        is_senior,
        final_value: final_value_u64,
        tokens_burned: tokens_escrowed,
    });

    Ok(())
}
