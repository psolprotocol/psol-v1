//! Private Transfer Instruction (Phase 3 Feature)
//!
//! Allows transferring value between commitments without leaving
//! the privacy pool. This is NOT IMPLEMENTED in Phase 2.
//!
//! # Concept
//! 1. User proves knowledge of N input commitments
//! 2. User provides M output commitments
//! 3. Circuit verifies: sum(input_amounts) = sum(output_amounts)
//! 4. Input nullifiers are marked spent, output commitments are inserted
//!
//! # Benefits
//! - Enables mixing without withdrawal
//! - Can split or merge commitments
//! - Never touches external accounts

use anchor_lang::prelude::*;

use crate::error::PrivacyError;
use crate::events::PrivateTransferEvent;
use crate::state::{MerkleTree, PoolConfig};

/// Accounts for private_transfer instruction (Phase 3).
#[derive(Accounts)]
pub struct PrivateTransfer<'info> {
    /// Pool configuration.
    #[account(
        mut,
        seeds = [b"pool", pool_config.token_mint.as_ref()],
        bump = pool_config.bump,
    )]
    pub pool_config: Account<'info, PoolConfig>,

    /// Merkle tree state.
    #[account(
        mut,
        seeds = [b"merkle_tree", pool_config.key().as_ref()],
        bump,
    )]
    pub merkle_tree: Account<'info, MerkleTree>,

    /// Transaction submitter.
    #[account(mut)]
    pub submitter: Signer<'info>,

    /// System program (for nullifier account creation).
    pub system_program: Program<'info, System>,
}

/// Handler for private_transfer instruction.
///
/// # PHASE 3: NOT IMPLEMENTED
///
/// This instruction is a placeholder for Phase 3 implementation.
/// Currently returns `NotImplemented` error.
pub fn handler(ctx: Context<PrivateTransfer>) -> Result<()> {
    let pool_config = &ctx.accounts.pool_config;

    // Check pool is not paused
    pool_config.require_not_paused()?;

    // Emit placeholder event for testing
    emit!(PrivateTransferEvent {
        pool: pool_config.key(),
        input_count: 0,
        output_count: 0,
        timestamp: Clock::get()?.unix_timestamp,
    });

    msg!("Private transfer is not implemented in Phase 2");
    msg!("This feature will be available in Phase 3");

    // Always fail - feature not implemented
    Err(error!(PrivacyError::NotImplemented))
}
