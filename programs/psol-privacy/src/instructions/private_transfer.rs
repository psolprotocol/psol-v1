//! Private Transfer Instruction - Phase 3
//!
//! 2-in-2-out transfers within the privacy pool.
//! Currently returns NotImplemented - requires dedicated circuit.

use anchor_lang::prelude::*;

use crate::error::PrivacyError;
use crate::state::{MerkleTree, PoolConfig};

#[derive(Accounts)]
pub struct PrivateTransfer<'info> {
    #[account(
        mut,
        seeds = [b"pool", pool_config.token_mint.as_ref()],
        bump = pool_config.bump,
    )]
    pub pool_config: Account<'info, PoolConfig>,

    #[account(
        mut,
        seeds = [b"merkle_tree", pool_config.key().as_ref()],
        bump,
    )]
    pub merkle_tree: Account<'info, MerkleTree>,

    #[account(mut)]
    pub submitter: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<PrivateTransfer>) -> Result<()> {
    let pool_config = &ctx.accounts.pool_config;
    pool_config.require_not_paused()?;

    msg!("Private transfer requires dedicated ZK circuit");
    msg!("This will be fully implemented when circuit is ready");

    Err(error!(PrivacyError::NotImplemented))
}
