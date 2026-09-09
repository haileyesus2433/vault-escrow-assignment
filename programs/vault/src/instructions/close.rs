use anchor_lang::prelude::*;
use anchor_lang::system_program::{transfer, Transfer};
use crate::state::VaultState;

#[derive(Accounts)]
pub struct Close<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        seeds = [b"state", user.key().as_ref()],
        bump = vault_state.state_bump,
        close = user
    )]
    pub vault_state: Account<'info, VaultState>,

    #[account(
        mut,
        seeds = [b"vault", vault_state.key().as_ref()],
        bump = vault_state.vault_bump,
    )]
    pub vault: SystemAccount<'info>,

    pub system_program: Program<'info, System>,
}

impl<'info> Close<'info> {
    pub fn close(&mut self) -> Result<()> {
        let balance = self.vault.lamports();

        if balance > 0 {
            let seeds = &[
                b"vault",
                self.vault_state.to_account_info().key.as_ref(),
                &[self.vault_state.vault_bump],
            ];
            let signer_seeds = &[&seeds[..]];

            let cpi_accounts = Transfer {
                from: self.vault.to_account_info(),
                to: self.user.to_account_info(),
            };

            let cpi_ctx = CpiContext::new_with_signer(
                self.system_program.key(),
                cpi_accounts,
                signer_seeds,
            );

            transfer(cpi_ctx, balance)?;
        }

        Ok(())
    }
}
