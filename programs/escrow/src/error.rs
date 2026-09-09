use anchor_lang::prelude::*;

#[error_code]
pub enum EscrowError {
    #[msg("Escrow has expired; take is no longer allowed")]
    EscrowExpired,
    #[msg("Escrow has not expired yet; refund is not allowed")]
    EscrowNotExpired,
}
