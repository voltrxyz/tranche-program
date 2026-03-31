use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

use crate::constants::*;
use crate::events::CancelWithdrawEvent;
use crate::state::{TrancheConfig, WithdrawReceipt};

#[derive(Accounts)]
pub struct CancelWithdraw<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        seeds = [TRANCHE_CONFIG_SEED, tranche_config.load()?.vault.as_ref()],
        bump,
    )]
    pub tranche_config: AccountLoader<'info, TrancheConfig>,

    /// CHECK: PDA used as CPI signer for escrow transfer; validated by seeds constraint.
    #[account(
        seeds = [TRANCHE_AUTHORITY_SEED, tranche_config.key().as_ref()],
        bump,
    )]
    pub tranche_authority: UncheckedAccount<'info>,

    /// The withdraw receipt to cancel. Validated: belongs to this config and this user.
    #[account(
        mut,
        close = user,
        has_one = user,
        has_one = tranche_config,
        seeds = [
            WITHDRAW_RECEIPT_SEED,
            tranche_config.key().as_ref(),
            withdraw_receipt.receipt_id.to_le_bytes().as_ref(),
        ],
        bump = withdraw_receipt.bump,
    )]
    pub withdraw_receipt: Account<'info, WithdrawReceipt>,

    /// Tranche token mint (senior or junior, must match the receipt's tranche).
    pub tranche_mint: Account<'info, Mint>,

    /// Tranche token escrow owned by tranche_authority (source of returned tokens).
    #[account(
        mut,
        token::mint = tranche_mint,
        token::authority = tranche_authority,
    )]
    pub tranche_token_escrow: Account<'info, TokenAccount>,

    /// User's tranche token account (destination for returned tokens).
    #[account(
        mut,
        token::mint = tranche_mint,
        token::authority = user,
    )]
    pub user_tranche_ata: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

pub fn handler(ctx: Context<CancelWithdraw>) -> Result<()> {
    let receipt = &ctx.accounts.withdraw_receipt;
    let tokens_to_return = receipt.tranche_tokens_escrowed;
    let receipt_id = receipt.receipt_id;

    // ── 1. Validate tranche mint matches receipt ────────────────────────
    {
        let config = ctx.accounts.tranche_config.load()?;
        let expected_mint = if receipt.is_senior {
            config.senior_mint
        } else {
            config.junior_mint
        };
        require_keys_eq!(
            ctx.accounts.tranche_mint.key(),
            expected_mint,
            anchor_lang::error::ErrorCode::ConstraintAddress
        );
    }

    // ── 2. Transfer escrowed tranche tokens: escrow → user ─────────────
    let tranche_config_key = ctx.accounts.tranche_config.key();
    let authority_seeds: &[&[u8]] = &[
        TRANCHE_AUTHORITY_SEED,
        tranche_config_key.as_ref(),
        &[ctx.bumps.tranche_authority],
    ];

    token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.tranche_token_escrow.to_account_info(),
                to: ctx.accounts.user_tranche_ata.to_account_info(),
                authority: ctx.accounts.tranche_authority.to_account_info(),
            },
            &[authority_seeds],
        ),
        tokens_to_return,
    )?;

    // ── 3. Emit event (receipt is closed automatically via `close = user`) ──
    emit!(CancelWithdrawEvent {
        user: ctx.accounts.user.key(),
        receipt_id,
        tokens_returned: tokens_to_return,
    });

    Ok(())
}
