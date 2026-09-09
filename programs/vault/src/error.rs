use anchor_lang::prelude::*;

#[error_code]
pub enum VaultError {
    #[msg("Invalid amount provided")]
    InvalidAmount,
}
