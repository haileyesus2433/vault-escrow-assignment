use anchor_lang::prelude::*;
use crate::state::Escrow;

#[derive(Accounts)]
pub struct Update<'info> {
    #[account(mut)]
    pub maker: Signer<'info>,

    #[account(
        mut,
        has_one = maker,
        seeds = [b"escrow", maker.key().as_ref(), escrow.seed.to_le_bytes().as_ref()],
        bump = escrow.bump,
    )]
    pub escrow: Account<'info, Escrow>,
}

impl<'info> Update<'info> {
    pub fn update(&mut self, receive: u64) -> Result<()> {
        self.escrow.receive = receive;
        Ok(())
    }
}
