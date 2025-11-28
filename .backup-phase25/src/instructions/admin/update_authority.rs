//! Update Authority Instruction
//!
//! Transfer pool admin rights to a new address.
//! Only callable by current authority.
//!
//! # Security
//! This is a critical operation. Consider:
//! - Using a multisig as authority
//! - Implementing a time-lock for authority transfers
//! - Requiring two-step transfer (propose + accept)

use anchor_lang::prelude::*;

use crate::error::PrivacyError;
use crate::events::AuthorityTransferred;
use crate::state::PoolConfig;

/// Accounts for update_authority instruction.
#[derive(Accounts)]
pub struct UpdateAuthority<'info> {
    /// Pool configuration to update.
    #[account(
        mut,
        seeds = [b"pool", pool_config.token_mint.as_ref()],
        bump = pool_config.bump,
        has_one = authority @ PrivacyError::Unauthorized,
    )]
    pub pool_config: Account<'info, PoolConfig>,

    /// Current pool authority (must sign).
    pub authority: Signer<'info>,
}

/// Handler for update_authority instruction.
///
/// # Arguments
/// * `new_authority` - Address of new pool authority
pub fn handler(ctx: Context<UpdateAuthority>, new_authority: Pubkey) -> Result<()> {
    let pool_config = &mut ctx.accounts.pool_config;
    let old_authority = pool_config.authority;

    // Prevent setting to same address (no-op protection)
    require!(
        new_authority != old_authority,
        PrivacyError::Unauthorized
    );

    // Prevent setting to system program (common mistake)
    require!(
        new_authority != Pubkey::default(),
        PrivacyError::Unauthorized
    );

    pool_config.transfer_authority(new_authority);

    emit!(AuthorityTransferred {
        pool: pool_config.key(),
        old_authority,
        new_authority,
        timestamp: Clock::get()?.unix_timestamp,
    });

    msg!("Authority transferred");
    msg!("Old authority: {}", old_authority);
    msg!("New authority: {}", new_authority);

    Ok(())
}
