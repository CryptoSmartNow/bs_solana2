use anchor_lang::prelude::*;

#[account]
pub struct GlobalState {
    pub admin: Pubkey,
    pub stable_coin_mint: Pubkey,
    pub cs_token_mint: Pubkey,
    pub join_fee: u64,
    pub saving_fee: u64,
    pub total_value_locked: u64,
    pub vault_state: u64,
    pub fountain: u64, // Initial balance or pool for rewards
    pub user_count: u64,
}

impl GlobalState {
    pub const SIZE: usize = 8 + 32 + 32 + 32 + 8 + 8 + 8 + 8 + 8 + 8;
}

#[account]
pub struct UserVault {
    pub owner: Pubkey,
    pub total_points: u64,
}

impl UserVault {
    pub const SIZE: usize = 8 + 32 + 8;
}

#[account]
pub struct Saving {
    pub owner: Pubkey,
    pub name: String, // Up to 32 chars for seed derivation?
    pub amount: u64,
    pub token_mint: Pubkey, // default for native SOL
    pub interest_accumulated: u64,
    pub start_time: i64,
    pub maturity_time: i64,
    pub penalty_percentage: u8,
    pub is_safe_mode: bool,
    pub is_valid: bool,
}

impl Saving {
    pub const MAX_NAME_LEN: usize = 32;
    pub const SIZE: usize = 8 + 32 + (4 + Self::MAX_NAME_LEN) + 8 + 32 + 8 + 8 + 8 + 1 + 1 + 1;
}
