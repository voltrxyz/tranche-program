use anchor_lang::prelude::*;

use crate::constants::*;
use crate::error::TrancheError;
use crate::state::TrancheConfig;

/// Valid pause bitflags mask (bits 0–3 only).
const VALID_PAUSE_MASK: u8 = 0x0F;

#[derive(Accounts)]
pub struct SetPaused<'info> {
    pub authority: Signer<'info>,

    #[account(
        mut,
        has_one = authority,
        seeds = [TRANCHE_CONFIG_SEED, tranche_config.load()?.vault.as_ref()],
        bump = tranche_config.load()?.tranche_config_bump,
    )]
    pub tranche_config: AccountLoader<'info, TrancheConfig>,
}

pub fn handler(ctx: Context<SetPaused>, paused: u8) -> Result<()> {
    require_eq!(
        paused & !VALID_PAUSE_MASK,
        0,
        TrancheError::InvalidPauseFlags
    );

    let mut config = ctx.accounts.tranche_config.load_mut()?;
    config.paused = paused;

    Ok(())
}
