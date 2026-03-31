use anchor_lang::prelude::*;

#[event]
#[derive(Default)]
pub struct DepositSeniorEvent {
    pub user: Pubkey,
    pub asset_amount: u64,
    pub tokens_minted: u64,
    pub senior_nav: u128,
    pub coverage_ratio: u128,
}

#[event]
#[derive(Default)]
pub struct DepositJuniorEvent {
    pub user: Pubkey,
    pub asset_amount: u64,
    pub tokens_minted: u64,
    pub junior_nav: u128,
    pub coverage_ratio: u128,
}

#[event]
#[derive(Default)]
pub struct RequestWithdrawEvent {
    pub user: Pubkey,
    pub receipt_id: u64,
    pub is_senior: bool,
    pub tokens_escrowed: u64,
    pub asset_value: u128,
    pub timestamp: i64,
}

#[event]
#[derive(Default)]
pub struct CompleteWithdrawEvent {
    pub user: Pubkey,
    pub receipt_id: u64,
    pub is_senior: bool,
    pub final_value: u64,
    pub tokens_burned: u64,
}

#[event]
#[derive(Default)]
pub struct CancelWithdrawEvent {
    pub user: Pubkey,
    pub receipt_id: u64,
    pub tokens_returned: u64,
}

#[event]
#[derive(Default)]
pub struct UpdateTrancheConfigEvent {
    pub authority: Pubkey,
    pub field_index: u8,
    pub slot: u64,
}

#[event]
#[derive(Default)]
pub struct CrankAccountingEvent {
    pub delta: i128,
    pub senior_nav: u128,
    pub junior_nav: u128,
    pub reserve_nav: u128,
    pub total_nav: u128,
    pub base_apr: u16,
    pub coverage_ratio: u128,
    pub slot: u64,
}

#[event]
#[derive(Default)]
pub struct WithdrawReserveEvent {
    pub authority: Pubkey,
    pub amount_withdrawn: u64,
    pub reserve_nav_after: u128,
    pub total_nav_after: u128,
}
