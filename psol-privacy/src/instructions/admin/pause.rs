//! Pause Pool Instruction
//!
//! Emergency stop mechanism - disables deposits and withdrawals.
//! Only callable by pool authority.

use anchor_lang::prelude::*;

use crate::error::PrivacyError;
use crate::events::PoolPaused;
use crate::state::PoolConfig;

/// Accounts for pause_pool instruction.
#[derive(Accounts)]
pub struct PausePool<'info> {
    /// Pool configuration to pause.
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

/// Handler for pause_pool instruction.
pub fn handler(ctx: Context<PausePool>) -> Result<()> {
    let pool_config = &mut ctx.accounts.pool_config;

    pool_config.set_paused(true);

    emit!(PoolPaused {
        pool: pool_config.key(),
        authority: ctx.accounts.authority.key(),
        timestamp: Clock::get()?.unix_timestamp,
    });

    msg!("Pool paused by authority");

    Ok(())
}
