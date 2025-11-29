//! Update Authority Instructions - Phase 4 (2-Step Transfer)

use anchor_lang::prelude::*;

use crate::error::PrivacyError;
use crate::events::{AuthorityTransferInitiated, AuthorityTransferCompleted, AuthorityTransferCancelled};
use crate::state::PoolConfig;

#[derive(Accounts)]
pub struct InitiateAuthorityTransfer<'info> {
    #[account(
        mut,
        seeds = [b"pool", pool_config.token_mint.as_ref()],
        bump = pool_config.bump,
        has_one = authority @ PrivacyError::Unauthorized,
    )]
    pub pool_config: Account<'info, PoolConfig>,

    pub authority: Signer<'info>,
}

pub fn initiate_transfer_handler(
    ctx: Context<InitiateAuthorityTransfer>,
    new_authority: Pubkey,
) -> Result<()> {
    let pool_config = &mut ctx.accounts.pool_config;
    let current_authority = ctx.accounts.authority.key();

    pool_config.initiate_authority_transfer(new_authority)?;

    emit!(AuthorityTransferInitiated {
        pool: pool_config.key(),
        current_authority,
        pending_authority: new_authority,
        timestamp: Clock::get()?.unix_timestamp,
    });

    msg!("Authority transfer initiated to: {}", new_authority);
    Ok(())
}

#[derive(Accounts)]
pub struct AcceptAuthorityTransfer<'info> {
    #[account(
        mut,
        seeds = [b"pool", pool_config.token_mint.as_ref()],
        bump = pool_config.bump,
    )]
    pub pool_config: Account<'info, PoolConfig>,

    pub new_authority: Signer<'info>,
}

pub fn accept_transfer_handler(ctx: Context<AcceptAuthorityTransfer>) -> Result<()> {
    let pool_config = &mut ctx.accounts.pool_config;
    let new_authority = ctx.accounts.new_authority.key();
    let old_authority = pool_config.authority;

    pool_config.accept_authority_transfer(new_authority)?;

    emit!(AuthorityTransferCompleted {
        pool: pool_config.key(),
        old_authority,
        new_authority,
        timestamp: Clock::get()?.unix_timestamp,
    });

    msg!("Authority transfer completed");
    Ok(())
}

#[derive(Accounts)]
pub struct CancelAuthorityTransfer<'info> {
    #[account(
        mut,
        seeds = [b"pool", pool_config.token_mint.as_ref()],
        bump = pool_config.bump,
        has_one = authority @ PrivacyError::Unauthorized,
    )]
    pub pool_config: Account<'info, PoolConfig>,

    pub authority: Signer<'info>,
}

pub fn cancel_transfer_handler(ctx: Context<CancelAuthorityTransfer>) -> Result<()> {
    let pool_config = &mut ctx.accounts.pool_config;

    if !pool_config.has_pending_transfer() {
        msg!("No pending authority transfer");
        return Ok(());
    }

    let cancelled_pending = pool_config.pending_authority;
    pool_config.cancel_authority_transfer();

    emit!(AuthorityTransferCancelled {
        pool: pool_config.key(),
        authority: ctx.accounts.authority.key(),
        cancelled_pending,
        timestamp: Clock::get()?.unix_timestamp,
    });

    msg!("Authority transfer cancelled");
    Ok(())
}

// Legacy support
#[derive(Accounts)]
pub struct UpdateAuthority<'info> {
    #[account(
        mut,
        seeds = [b"pool", pool_config.token_mint.as_ref()],
        bump = pool_config.bump,
        has_one = authority @ PrivacyError::Unauthorized,
    )]
    pub pool_config: Account<'info, PoolConfig>,

    pub authority: Signer<'info>,
}

#[deprecated(note = "Use 2-step authority transfer")]
pub fn handler(ctx: Context<UpdateAuthority>, new_authority: Pubkey) -> Result<()> {
    let pool_config = &mut ctx.accounts.pool_config;
    pool_config.initiate_authority_transfer(new_authority)?;
    msg!("DEPRECATED: Use initiate/accept_authority_transfer");
    Ok(())
}
