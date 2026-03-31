use anchor_lang::prelude::*;

use crate::constants::TRANCHE_CONFIG_SEED;
use crate::error::TrancheError;
use crate::state::TrancheConfig;

#[derive(Accounts)]
pub struct AcceptAuthority<'info> {
    #[account(
        mut,
        seeds = [TRANCHE_CONFIG_SEED, tranche_config.load()?.vault.as_ref()],
        bump = tranche_config.load()?.tranche_config_bump,
    )]
    pub tranche_config: AccountLoader<'info, TrancheConfig>,

    pub pending_authority: Signer<'info>,
}

pub fn handler(ctx: Context<AcceptAuthority>) -> Result<()> {
    let mut config = ctx.accounts.tranche_config.load_mut()?;
    require!(
        config.pending_authority == ctx.accounts.pending_authority.key(),
        TrancheError::Unauthorized
    );
    config.authority = ctx.accounts.pending_authority.key();
    config.pending_authority = Pubkey::default();
    Ok(())
}
