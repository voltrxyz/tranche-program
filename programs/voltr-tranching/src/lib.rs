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
        instructions::initialize_tranche_config::handle_initialize_tranche_config(ctx, params)
    }

    pub fn push_apr_observation(ctx: Context<PushAprObservation>) -> Result<()> {
        instructions::push_apr_observation::handle_push_apr_observation(ctx)
    }
}
