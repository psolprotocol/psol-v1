//! Deposit Instruction
//!
//! Deposits SPL tokens into the privacy pool and inserts a commitment
//! into the Merkle tree.
//!
//! # Commitment Scheme
//! commitment = hash(secret, nullifier_preimage, amount)
//!
//! The commitment is computed ON-CHAIN to ensure:
//! 1. Amount is bound to the commitment (prevents amount manipulation)
//! 2. Circuit can replicate the same computation
//! 3. User cannot insert arbitrary commitments
//!
//! # User Responsibility
//! Users MUST save their secret and nullifier_preimage.
//! These are needed to generate withdrawal proofs later.
//! Lost secrets = lost funds (no recovery possible).

use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

use crate::crypto::poseidon;
use crate::error::PrivacyError;
use crate::events::DepositEvent;
use crate::state::{MerkleTree, PoolConfig};

/// Accounts for deposit instruction.
#[derive(Accounts)]
pub struct Deposit<'info> {
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
        constraint = merkle_tree.pool == pool_config.key() @ PrivacyError::Unauthorized,
    )]
    pub merkle_tree: Account<'info, MerkleTree>,

    /// Token vault (receives deposited tokens).
    #[account(
        mut,
        seeds = [b"vault", pool_config.key().as_ref()],
        bump,
        constraint = vault.mint == pool_config.token_mint @ PrivacyError::InvalidMint,
        constraint = vault.owner == pool_config.key() @ PrivacyError::Unauthorized,
    )]
    pub vault: Account<'info, TokenAccount>,

    /// User's source token account.
    #[account(
        mut,
        constraint = user_token_account.mint == pool_config.token_mint @ PrivacyError::InvalidMint,
    )]
    pub user_token_account: Account<'info, TokenAccount>,

    /// User making the deposit (signs transaction).
    #[account(mut)]
    pub user: Signer<'info>,

    /// Token program.
    pub token_program: Program<'info, Token>,
}

/// Handler for deposit instruction.
///
/// # Arguments
/// * `amount` - Token amount to deposit (must be > 0)
/// * `secret` - Random secret (32 bytes) - USER MUST SAVE THIS
/// * `nullifier_preimage` - Nullifier preimage (32 bytes) - USER MUST SAVE THIS
///
/// # Commitment Computation
/// The commitment is computed as:
/// ```text
/// commitment = Poseidon(secret, nullifier_preimage, amount)
/// ```
///
/// This binds the amount to the commitment, preventing amount manipulation
/// during withdrawal.
///
/// # Security Notes
/// - secret and nullifier_preimage should be cryptographically random
/// - User must store these values securely - they're needed for withdrawal
/// - Lost secret/nullifier = lost funds (no recovery mechanism)
pub fn handler(
    ctx: Context<Deposit>,
    amount: u64,
    secret: [u8; 32],
    nullifier_preimage: [u8; 32],
) -> Result<()> {
    let pool_config = &mut ctx.accounts.pool_config;
    let merkle_tree = &mut ctx.accounts.merkle_tree;

    // Check pool is not paused
    pool_config.require_not_paused()?;

    // Validate amount
    require!(amount > 0, PrivacyError::InvalidAmount);

    // Validate secret is not all zeros (weak secret)
    require!(
        !secret.iter().all(|&b| b == 0),
        PrivacyError::InvalidSecret
    );

    // Validate nullifier preimage is not all zeros
    require!(
        !nullifier_preimage.iter().all(|&b| b == 0),
        PrivacyError::InvalidNullifier
    );

    // Compute commitment on-chain
    // This ensures amount is bound to the commitment
    let commitment = poseidon::hash_commitment(&secret, &nullifier_preimage, amount);

    // Validate commitment is not zero (should never happen with valid inputs)
    require!(
        !commitment.iter().all(|&b| b == 0),
        PrivacyError::InvalidCommitment
    );

    // Check tree has capacity
    require!(!merkle_tree.is_full(), PrivacyError::MerkleTreeFull);

    // Transfer tokens from user to vault
    let cpi_accounts = Transfer {
        from: ctx.accounts.user_token_account.to_account_info(),
        to: ctx.accounts.vault.to_account_info(),
        authority: ctx.accounts.user.to_account_info(),
    };
    let cpi_ctx = CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);
    token::transfer(cpi_ctx, amount)?;

    // Insert commitment into Merkle tree
    let leaf_index = merkle_tree.insert_leaf(commitment)?;

    // Get new root after insertion
    let new_root = merkle_tree.get_current_root();

    // Update pool statistics
    pool_config.increment_deposits()?;

    // Emit deposit event
    // IMPORTANT: Clients must index this to track their leaf_index
    emit!(DepositEvent {
        pool: pool_config.key(),
        commitment,
        leaf_index,
        merkle_root: new_root,
        amount,
        timestamp: Clock::get()?.unix_timestamp,
    });

    msg!("Deposit successful");
    msg!("Amount: {}", amount);
    msg!("Leaf index: {}", leaf_index);
    msg!("Commitment: {:?}", &commitment[..8]); // Only log first 8 bytes
    msg!("New root: {:?}", &new_root[..8]);

    Ok(())
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Compute the nullifier hash from preimage and secret.
/// This is what gets revealed on-chain during withdrawal.
///
/// nullifier_hash = Poseidon(nullifier_preimage, secret)
///
/// # Note
/// This function is provided as a helper for clients but is not used
/// on-chain during deposit. It's used during withdrawal verification.
#[allow(dead_code)]
pub fn compute_nullifier_hash(nullifier_preimage: &[u8; 32], secret: &[u8; 32]) -> [u8; 32] {
    poseidon::hash_nullifier(nullifier_preimage, secret)
}
