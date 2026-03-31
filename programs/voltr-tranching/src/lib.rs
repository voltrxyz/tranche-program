use anchor_lang::prelude::*;

pub mod constants;
pub mod error;
pub mod events;
pub mod math;
pub mod state;

declare_id!("EvVxBE31HKE3QdRZXZB7gkkPTcMcAPKGTWJKAvoGuZoX");

#[program]
pub mod voltr_tranching {}
