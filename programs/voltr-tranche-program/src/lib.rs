use anchor_lang::prelude::*;

declare_id!("EvVxBE31HKE3QdRZXZB7gkkPTcMcAPKGTWJKAvoGuZoX");

#[program]
pub mod voltr_tranche_program {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
