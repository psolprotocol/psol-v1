//! Deposit Instruction - Phase 3
//!
//! Deposits SPL tokens into the privacy pool and inserts a commitment
//! into the Merkle tree.
//!
//! # Phase 3 Commitment Model (Off-Chain)
//!
//! In Phase 3, the commitment is computed OFF-CHAIN by the user:
//! ```text
//! commitment = Poseidon(secret, nullifier_preimage, amount)
//! ```
//!
//! The user provides the pre-computed commitment to the deposit instruction.
//! This design:
//! 1. Avoids Solana BPF stack limits with Poseidon
//! 2. Ensures exact compatibility with ZK circuit
//! 3. Keeps secret/nullifier_preimage completely off-chain
//!
//! # User Responsibility
//! Users MUST:
//! 1. Generate random (secret, nullifier_preimage)
//! 2. Compute commitment = Poseidon(secret, nullifier_preimage, amount)
//! 3. Save (secret, nullifier_preimage, leaf_index) securely
//! 4. Lost secrets = lost funds (no recovery possible)
//!
//! # Security
//! The ZK circuit enforces that the commitment was computed correctly.
//! An invalid commitment will make withdrawal impossible (funds locked).

use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

use crate::error::PrivacyError;
use crate::events::DepositEvent;
use crate::state::{MerkleTree, PoolConfig};

/// Accounts for deposit instruction.
#[derive(Accounts)]
#[instruction(amount: u64, commitment: [u8; 32])]
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
/// * `commitment` - Pre-computed commitment hash (32 bytes)
///
/// The commitment MUST be computed off-chain as:
/// ```text
/// commitment = Poseidon(secret, nullifier_preimage, amount)
/// ```
///
/// Using the exact same Poseidon parameters as the withdrawal circuit.
///
/// # Returns
/// Emits DepositEvent with leaf_index needed for withdrawal proof.
///
/// # Security Notes
/// - Commitment must be computed correctly off-chain
/// - Invalid commitment = funds locked forever (can't generate valid proof)
/// - User must store (secret, nullifier_preimage, leaf_index) securely
pub fn handler(
    ctx: Context<Deposit>,
    amount: u64,
    commitment: [u8; 32],
) -> Result<()> {
    let pool_config = &mut ctx.accounts.pool_config;
    let merkle_tree = &mut ctx.accounts.merkle_tree;

    // ========== VALIDATION ==========

    // Check pool is not paused
    pool_config.require_not_paused()?;

    // Validate amount
    require!(amount > 0, PrivacyError::InvalidAmount);

    // Validate commitment is not zero (invalid commitment)
    require!(
        !is_zero_commitment(&commitment),
        PrivacyError::InvalidCommitment
    );

    // Check tree has capacity
    require!(!merkle_tree.is_full(), PrivacyError::MerkleTreeFull);

    // ========== TOKEN TRANSFER ==========

    // Transfer tokens from user to vault
    let cpi_accounts = Transfer {
        from: ctx.accounts.user_token_account.to_account_info(),
        to: ctx.accounts.vault.to_account_info(),
        authority: ctx.accounts.user.to_account_info(),
    };
    let cpi_ctx = CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);
    token::transfer(cpi_ctx, amount)?;

    // ========== MERKLE TREE UPDATE ==========

    // Insert commitment into Merkle tree
    let leaf_index = merkle_tree.insert_leaf(commitment)?;

    // Get new root after insertion
    let new_root = merkle_tree.get_current_root();

    // ========== STATE UPDATE ==========

    // Update pool statistics
    pool_config.increment_deposits()?;

    // ========== EVENT EMISSION ==========

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
    msg!("Commitment: {:?}", &commitment[..8]); // Only log first 8 bytes for privacy

    Ok(())
}

/// Check if commitment is all zeros (invalid).
#[inline]
fn is_zero_commitment(commitment: &[u8; 32]) -> bool {
    commitment.iter().all(|&b| b == 0)
}

// ============================================================================
// CLIENT HELPER FUNCTIONS (for off-chain use)
// ============================================================================

/// Compute commitment off-chain.
///
/// This function documents the expected commitment format.
/// Actual computation should use a Poseidon library matching the circuit.
///
/// ```text
/// commitment = Poseidon(secret, nullifier_preimage, amount_as_field_element)
/// ```
///
/// # Parameters
/// - `secret`: 32 random bytes
/// - `nullifier_preimage`: 32 random bytes  
/// - `amount`: u64 token amount
///
/// # Returns
/// 32-byte commitment hash
///
/// # Note
/// This is documentation only. Use snarkjs/circomlib for actual computation.
#[allow(dead_code)]
pub fn compute_commitment_offchain(
    _secret: &[u8; 32],
    _nullifier_preimage: &[u8; 32],
    _amount: u64,
) -> [u8; 32] {
    // This should be computed using Poseidon matching the circuit
    // Example with circomlib:
    // const commitment = poseidon([secret, nullifier_preimage, amount]);
    unimplemented!("Use off-chain Poseidon library (snarkjs/circomlib)")
}

/// Compute nullifier hash off-chain.
///
/// ```text
/// nullifier_hash = Poseidon(nullifier_preimage, secret)
/// ```
///
/// This is revealed on-chain during withdrawal.
#[allow(dead_code)]
pub fn compute_nullifier_offchain(
    _nullifier_preimage: &[u8; 32],
    _secret: &[u8; 32],
) -> [u8; 32] {
    // This should be computed using Poseidon matching the circuit
    unimplemented!("Use off-chain Poseidon library (snarkjs/circomlib)")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zero_commitment_detection() {
        let zero = [0u8; 32];
        assert!(is_zero_commitment(&zero));

        let non_zero = [1u8; 32];
        assert!(!is_zero_commitment(&non_zero));

        let partial = {
            let mut arr = [0u8; 32];
            arr[31] = 1;
            arr
        };
        assert!(!is_zero_commitment(&partial));
    }
}
