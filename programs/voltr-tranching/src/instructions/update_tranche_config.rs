use anchor_lang::prelude::*;

use crate::constants::*;
use crate::error::TrancheError;
use crate::events::UpdateTrancheConfigEvent;
use crate::state::TrancheConfig;

#[derive(AnchorDeserialize, AnchorSerialize)]
pub enum TrancheConfigField {
    AprFloorBps(u16),
    RiskXBps(u16),
    RiskYBps(u16),
    RiskK([u8; 16]),
    ReserveFeeBps(u16),
    SeniorExitFeeBps(u16),
    JuniorExitFeeBps(u16),
    MinJrSrRatioBps(u16),
    MinCoverageForMintBps(u16),
    MinCoverageBufferBps(u16),
    SeniorCooldownSeconds(u64),
    JuniorCooldownSeconds(u64),
}

impl TrancheConfigField {
    pub fn discriminant(&self) -> u8 {
        match self {
            Self::AprFloorBps(_) => 0,
            Self::RiskXBps(_) => 1,
            Self::RiskYBps(_) => 2,
            Self::RiskK(_) => 3,
            Self::ReserveFeeBps(_) => 4,
            Self::SeniorExitFeeBps(_) => 5,
            Self::JuniorExitFeeBps(_) => 6,
            Self::MinJrSrRatioBps(_) => 7,
            Self::MinCoverageForMintBps(_) => 8,
            Self::MinCoverageBufferBps(_) => 9,
            Self::SeniorCooldownSeconds(_) => 10,
            Self::JuniorCooldownSeconds(_) => 11,
        }
    }
}

#[derive(Accounts)]
pub struct UpdateTrancheConfig<'info> {
    pub authority: Signer<'info>,

    #[account(
        mut,
        seeds = [TRANCHE_CONFIG_SEED, tranche_config.load()?.vault.as_ref()],
        bump = tranche_config.load()?.tranche_config_bump,
        has_one = authority @ TrancheError::Unauthorized,
    )]
    pub tranche_config: AccountLoader<'info, TrancheConfig>,
}

pub fn handler(ctx: Context<UpdateTrancheConfig>, field: TrancheConfigField) -> Result<()> {
    let field_index = field.discriminant();

    {
        let mut config = ctx.accounts.tranche_config.load_mut()?;

        match field {
            TrancheConfigField::AprFloorBps(val) => {
                config.apr_floor_bps = val;
            }
            TrancheConfigField::RiskXBps(val) => {
                config.risk_x_bps = val;
            }
            TrancheConfigField::RiskYBps(val) => {
                config.risk_y_bps = val;
            }
            TrancheConfigField::RiskK(val) => {
                config.risk_k = val;
            }
            TrancheConfigField::ReserveFeeBps(val) => {
                require!(val <= MAX_RESERVE_FEE_BPS, TrancheError::InvalidFee);
                config.reserve_fee_bps = val;
            }
            TrancheConfigField::SeniorExitFeeBps(val) => {
                require!(val <= MAX_EXIT_FEE_BPS, TrancheError::InvalidFee);
                config.senior_exit_fee_bps = val;
            }
            TrancheConfigField::JuniorExitFeeBps(val) => {
                require!(val <= MAX_EXIT_FEE_BPS, TrancheError::InvalidFee);
                config.junior_exit_fee_bps = val;
            }
            TrancheConfigField::MinJrSrRatioBps(val) => {
                config.min_jr_sr_ratio_bps = val;
            }
            TrancheConfigField::MinCoverageForMintBps(val) => {
                config.min_coverage_for_mint_bps = val;
            }
            TrancheConfigField::MinCoverageBufferBps(val) => {
                config.min_coverage_buffer_bps = val;
            }
            TrancheConfigField::SeniorCooldownSeconds(val) => {
                config.senior_cooldown_seconds = val;
            }
            TrancheConfigField::JuniorCooldownSeconds(val) => {
                config.junior_cooldown_seconds = val;
            }
        }
    }

    emit!(UpdateTrancheConfigEvent {
        authority: ctx.accounts.authority.key(),
        field_index,
        slot: Clock::get()?.slot,
    });

    Ok(())
}
