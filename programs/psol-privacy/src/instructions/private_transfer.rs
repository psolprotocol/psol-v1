//! Private Transfer Instruction - disabled in pSOL v1
//!
//! NOTE:
//! This instruction is intentionally NOT implemented in pSOL v1.
//! It is a placeholder for a future join-split private transfer design.
//!
//! Any call to this instruction will always fail with PrivacyError::NotImplemented.

use anchor_lang::prelude::*;

use crate::error::PrivacyError;
use crate::state::{MerkleTree, PoolConfig};

#[deprecated(note = "Private transfers are not implemented in pSOL v1. This is a placeholder for a future version.")]
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

#[deprecated(note = "Private transfers are not implemented in pSOL v1. Use deposit/withdraw only.")]
pub fn handler(ctx: Context<PrivateTransfer>) -> Result<()> {
    let pool_config = &ctx.accounts.pool_config;
    pool_config.require_not_paused()?;

    msg!("ERROR: private_transfer is NOT available in pSOL v1.");
    msg!("This is a non-functional placeholder for a future join-split private transfer.");
    msg!("Please use deposit() and withdraw() only in this version.");

    Err(error!(PrivacyError::NotImplemented))
}
