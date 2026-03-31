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

    pub fn crank_accounting(ctx: Context<CrankAccounting>) -> Result<()> {
        instructions::crank_accounting::handler(ctx)
    }

    pub fn update_tranche_config(
        ctx: Context<UpdateTrancheConfig>,
        field: TrancheConfigField,
    ) -> Result<()> {
        instructions::update_tranche_config::handler(ctx, field)
    }

    pub fn set_paused(ctx: Context<SetPaused>, paused: u8) -> Result<()> {
        instructions::set_paused::handler(ctx, paused)
    }

    pub fn transfer_authority(ctx: Context<TransferAuthority>) -> Result<()> {
        instructions::transfer_authority::handler(ctx)
    }

    pub fn accept_authority(ctx: Context<AcceptAuthority>) -> Result<()> {
        instructions::accept_authority::handler(ctx)
    }

    pub fn deposit_senior(ctx: Context<DepositSenior>, amount: u64) -> Result<()> {
        instructions::deposit_senior::handler(ctx, amount)
    }

    pub fn request_withdraw_senior(
        ctx: Context<RequestWithdrawSenior>,
        amount: u64,
        is_token_amount: bool,
    ) -> Result<()> {
        instructions::request_withdraw_senior::handler(ctx, amount, is_token_amount)
    }

    pub fn cancel_withdraw(ctx: Context<CancelWithdraw>) -> Result<()> {
        instructions::cancel_withdraw::handler(ctx)
    }
}
