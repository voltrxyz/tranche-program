use anchor_lang::prelude::*;

pub mod constants;
pub mod error;
pub mod events;
pub mod instructions;
pub mod math;
pub mod state;

use instructions::*;

declare_id!("EvVxBE31HKE3QdRZXZB7gkkPTcMcAPKGTWJKAvoGuZoX");

#[program]
pub mod voltr_tranching {
    use super::*;

    pub fn initialize_tranche_config(
        ctx: Context<InitializeTrancheConfig>,
        params: InitializeTrancheConfigParams,
    ) -> Result<()> {
        instructions::initialize_tranche_config::handler(ctx, params)
    }

    pub fn transfer_authority(ctx: Context<TransferAuthority>) -> Result<()> {
        instructions::transfer_authority::handler(ctx)
    }

    pub fn accept_authority(ctx: Context<AcceptAuthority>) -> Result<()> {
        instructions::accept_authority::handler(ctx)
    }
}
