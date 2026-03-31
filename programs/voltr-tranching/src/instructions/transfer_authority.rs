use anchor_lang::prelude::*;

use crate::constants::TRANCHE_CONFIG_SEED;
use crate::state::TrancheConfig;

#[derive(Accounts)]
pub struct TransferAuthority<'info> {
    #[account(
        mut,
        seeds = [TRANCHE_CONFIG_SEED, tranche_config.load()?.vault.as_ref()],
        bump = tranche_config.load()?.tranche_config_bump,
        has_one = authority,
    )]
    pub tranche_config: AccountLoader<'info, TrancheConfig>,

    pub authority: Signer<'info>,

    /// The new authority that must call `accept_authority` to complete the transfer.
    /// CHECK: Any valid pubkey can be proposed as the new authority.
    pub new_authority: UncheckedAccount<'info>,
}

pub fn handler(ctx: Context<TransferAuthority>) -> Result<()> {
    let mut config = ctx.accounts.tranche_config.load_mut()?;
    config.pending_authority = ctx.accounts.new_authority.key();
    Ok(())
}
