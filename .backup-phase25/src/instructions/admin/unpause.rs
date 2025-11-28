//! Unpause Pool Instruction
//!
//! Resume pool operations after emergency pause.
//! Only callable by pool authority.

use anchor_lang::prelude::*;

use crate::error::PrivacyError;
use crate::events::PoolUnpaused;
use crate::state::PoolConfig;

/// Accounts for unpause_pool instruction.
#[derive(Accounts)]
pub struct UnpausePool<'info> {
    /// Pool configuration to unpause.
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

    pool_config.set_paused(false);

    emit!(PoolUnpaused {
        pool: pool_config.key(),
        authority: ctx.accounts.authority.key(),
        timestamp: Clock::get()?.unix_timestamp,
    });

    msg!("Pool unpaused by authority");

    Ok(())
}
