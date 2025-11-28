//! Unpause Pool Instruction
//!
//! Resumes pool operations after emergency pause.

use anchor_lang::prelude::*;

use crate::error::PrivacyError;
use crate::events::PoolUnpaused;
use crate::state::PoolConfig;

/// Accounts for unpause_pool instruction.
#[derive(Accounts)]
pub struct UnpausePool<'info> {
    /// Pool configuration account.
    #[account(
        mut,
        seeds = [b"pool", pool_config.token_mint.as_ref()],
        bump = pool_config.bump,
        has_one = authority @ PrivacyError::Unauthorized,
    )]
    pub pool_config: Account<'info, PoolConfig>,

    /// Pool authority (must sign).
    pub authority: Signer<'info>,
}

/// Handler for unpause_pool instruction.
pub fn handler(ctx: Context<UnpausePool>) -> Result<()> {
    let pool_config = &mut ctx.accounts.pool_config;

    // Clear paused state
    pool_config.set_paused(false);

    // Emit event
    emit!(PoolUnpaused {
        pool: pool_config.key(),
        authority: ctx.accounts.authority.key(),
        timestamp: Clock::get()?.unix_timestamp,
    });

    msg!("Pool unpaused");

    Ok(())
}
