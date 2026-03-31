use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

use crate::constants::*;
use crate::error::TrancheError;
use crate::events::RequestWithdrawEvent;
use crate::instructions::crank_accounting::{read_decimal, run_inline_crank, write_decimal};
use crate::math::{Decimal, DecimalExtra, SafeMath, BPS_PRECISION};
use crate::state::{TrancheConfig, WithdrawReceipt};

#[derive(Accounts)]
pub struct RequestWithdrawSenior<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        seeds = [TRANCHE_CONFIG_SEED, vault.key().as_ref()],
        bump,
    )]
    pub tranche_config: AccountLoader<'info, TrancheConfig>,

    /// CHECK: PDA used as CPI signer for token transfers; validated by seeds constraint.
    #[account(
        seeds = [TRANCHE_AUTHORITY_SEED, tranche_config.key().as_ref()],
        bump,
    )]
    pub tranche_authority: UncheckedAccount<'info>,

    /// Senior tranche token mint (read-only, needed for supply).
    pub senior_mint: Account<'info, Mint>,

    /// Junior tranche token mint (read-only, needed for inline crank auto-pause check).
    pub junior_mint: Account<'info, Mint>,

    /// User's senior tranche token account (source of tokens to escrow).
    #[account(
        mut,
        token::mint = senior_mint,
        token::authority = user,
    )]
    pub user_senior_ata: Account<'info, TokenAccount>,

    /// Senior tranche token escrow owned by tranche_authority (receives escrowed tokens).
    #[account(
        mut,
        token::mint = senior_mint,
        token::authority = tranche_authority,
    )]
    pub senior_token_escrow: Account<'info, TokenAccount>,

    /// LP token custody owned by tranche_authority (needed for inline crank NAV calc).
    pub vault_lp_custody: Account<'info, TokenAccount>,

    // ── Voltr Vault accounts (for inline crank) ────────────────────────

    /// The underlying Voltr Vault.
    pub vault: AccountLoader<'info, voltr_vault::state::Vault>,

    /// Vault LP token mint (supply needed for inline crank).
    pub vault_lp_mint: Account<'info, Mint>,

    // ── WithdrawReceipt (init) ─────────────────────────────────────────

    /// The withdraw receipt PDA to be created.
    #[account(
        init,
        payer = user,
        space = 8 + WithdrawReceipt::INIT_SPACE,
        seeds = [
            WITHDRAW_RECEIPT_SEED,
            tranche_config.key().as_ref(),
            tranche_config.load()?.next_receipt_id.to_le_bytes().as_ref(),
        ],
        bump,
    )]
    pub withdraw_receipt: Account<'info, WithdrawReceipt>,

    // ── Programs ─────────────────────────────────────────────────────────

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn handler(
    ctx: Context<RequestWithdrawSenior>,
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

    // ── 3. Load config and validate accounts ─────────────────────────────
    let mut config = ctx.accounts.tranche_config.load_mut()?;

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

    // Re-check pause after crank (auto-pause may have set withdrawal pause bit)
    require!(
        config.paused & 0x04 == 0,
        TrancheError::SeniorWithdrawalsPaused
    );

    // ── 5. Calculate token amount and asset value ────────────────────────
    let senior_supply = ctx.accounts.senior_mint.supply;
    let senior_nav = crank_result.senior_nav;

    // Guard against zero supply (no tokens to withdraw) or zero NAV
    // (post-loss wipeout). full_mul_int_ratio panics on zero denominator.
    require!(senior_supply > 0, TrancheError::ZeroAmount);
    require!(senior_nav > Decimal::ZERO, TrancheError::ZeroAmount);

    let (token_amount, asset_value) = if is_token_amount {
        // `amount` is in tranche tokens; compute asset value
        let asset_val = Decimal::from_num(amount).full_mul_int_ratio(
            senior_nav.to_decimal_bits(),
            u128::from(senior_supply),
        );
        (amount, asset_val)
    } else {
        // `amount` is in asset terms; compute token amount
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

    // ── 6. Apply exit fee ────────────────────────────────────────────────
    let fee = asset_value.mul_int_ratio(
        u128::from(config.senior_exit_fee_bps),
        u128::from(BPS_PRECISION),
    );
    let net_asset = asset_value.safe_sub(fee)?;

    // Add fee to reserve_nav, debit from senior_nav (internal reallocation)
    let new_reserve_nav = read_decimal(&config.reserve_nav).safe_add(fee)?;
    config.reserve_nav = write_decimal(new_reserve_nav);
    let new_senior_nav = read_decimal(&config.senior_nav).safe_sub(fee)?;
    config.senior_nav = write_decimal(new_senior_nav);

    // ── 7. Create WithdrawReceipt ────────────────────────────────────────
    let receipt_id = config.next_receipt_id;
    let cooldown = config.senior_cooldown_seconds;
    config.next_receipt_id = receipt_id
        .checked_add(1)
        .ok_or(error!(TrancheError::MathOverflow))?;

    // Save bump before dropping config
    drop(config);

    let withdraw_receipt = &mut ctx.accounts.withdraw_receipt;
    withdraw_receipt.tranche_config = ctx.accounts.tranche_config.key();
    withdraw_receipt.user = ctx.accounts.user.key();
    withdraw_receipt.receipt_id = receipt_id;
    withdraw_receipt.is_senior = true;
    withdraw_receipt.tranche_tokens_escrowed = token_amount;
    withdraw_receipt.asset_amount_at_request = net_asset.to_decimal_bits();
    withdraw_receipt.withdrawable_from_ts = now
        .checked_add(cooldown as i64)
        .ok_or(error!(TrancheError::MathOverflow))?;
    withdraw_receipt.bump = ctx.bumps.withdraw_receipt;

    // ── 8. Transfer senior tokens: user → escrow ─────────────────────────
    token::transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.user_senior_ata.to_account_info(),
                to: ctx.accounts.senior_token_escrow.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        ),
        token_amount,
    )?;

    // ── 9. Emit event ────────────────────────────────────────────────────
    emit!(RequestWithdrawEvent {
        user: ctx.accounts.user.key(),
        receipt_id,
        is_senior: true,
        tokens_escrowed: token_amount,
        asset_value: net_asset.to_decimal_bits(),
        timestamp: now,
    });

    Ok(())
}
