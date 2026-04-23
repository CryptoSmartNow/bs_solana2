use anchor_lang::prelude::*;

#[error_code]
pub enum BitsaveError {
    #[msg("Admin only call required")]
    AdminCallRequired,
    #[msg("User not registered")]
    UserNotRegistered,
    #[msg("Insufficient funds to pay fee")]
    NotEnoughToPayFee,
    #[msg("Invalid maturity time")]
    InvalidTime,
    #[msg("Invalid saving name or already exists")]
    InvalidSaving,
    #[msg("Safe mode not yet supported")]
    NotSupported,
    #[msg("Arithmetic overflow")]
    MathOverflow,
    #[msg("Invalid token mint")]
    InvalidMint,
    #[msg("Invalid account owner")]
    InvalidOwner,
}
