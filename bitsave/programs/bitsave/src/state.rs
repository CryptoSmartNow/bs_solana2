use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct GlobalState {
    pub admin: Pubkey,
    pub stable_coin_mint: Pubkey,
    pub cs_token_mint: Pubkey,
    pub join_fee: u64,
    pub saving_fee: u64,
    pub total_value_locked: u64,
    pub user_count: u64,
}

#[account]
#[derive(InitSpace)]
pub struct UserVault {
    pub owner: Pubkey,
}

#[account]
#[derive(InitSpace)]
pub struct Saving {
    pub owner: Pubkey,
    #[max_len(32)]
    pub name: String,
    pub amount: u64,
    pub token_mint: Pubkey, // default for native SOL
    pub start_time: i64,
    pub maturity_time: i64,
    pub penalty_percentage: u8,
    pub is_safe_mode: bool,
    pub is_valid: bool,
}