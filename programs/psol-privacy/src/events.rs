//! Event definitions for pSol Privacy Pool
//!
//! Events are emitted for off-chain indexing and client synchronization.
//! Clients MUST index these events to construct Merkle proofs.

use anchor_lang::prelude::*;

/// Emitted when a new privacy pool is initialized.
#[event]
pub struct PoolInitialized {
    /// Pool configuration account address
    pub pool: Pubkey,
    /// Pool authority (admin)
    pub authority: Pubkey,
    /// SPL token mint for this pool
    pub token_mint: Pubkey,
    /// Merkle tree depth
    pub tree_depth: u8,
    /// Root history buffer size
    pub root_history_size: u16,
    /// Unix timestamp
    pub timestamp: i64,
}

/// Emitted when a verification key is set or updated.
#[event]
pub struct VerificationKeySet {
    /// Pool this VK belongs to
    pub pool: Pubkey,
    /// Authority who set the VK
    pub authority: Pubkey,
    /// Number of IC points in VK
    pub ic_length: u8,
    /// Unix timestamp
    pub timestamp: i64,
}

/// Emitted when tokens are deposited into the pool.
///
/// CRITICAL: Clients must index this event to:
/// 1. Track their leaf_index for proof generation
/// 2. Reconstruct the Merkle tree for path computation
#[event]
pub struct DepositEvent {
    /// Pool receiving the deposit
    pub pool: Pubkey,
    /// Commitment inserted into Merkle tree
    pub commitment: [u8; 32],
    /// Index of this leaf in the tree (0-indexed)
    pub leaf_index: u32,
    /// New Merkle root after insertion
    pub merkle_root: [u8; 32],
    /// Amount deposited
    pub amount: u64,
    /// Unix timestamp
    pub timestamp: i64,
}

/// Emitted when tokens are withdrawn from the pool.
#[event]
pub struct WithdrawEvent {
    /// Pool being withdrawn from
    pub pool: Pubkey,
    /// Nullifier hash (marks commitment as spent)
    pub nullifier_hash: [u8; 32],
    /// Recipient of withdrawn tokens
    pub recipient: Pubkey,
    /// Amount withdrawn (after fee)
    pub amount: u64,
    /// Relayer address (if any)
    pub relayer: Pubkey,
    /// Fee paid to relayer
    pub relayer_fee: u64,
    /// Unix timestamp
    pub timestamp: i64,
}

/// Emitted for private transfers (Phase 3).
#[event]
pub struct PrivateTransferEvent {
    /// Pool where transfer occurred
    pub pool: Pubkey,
    /// Number of input nullifiers consumed
    pub input_count: u8,
    /// Number of output commitments created
    pub output_count: u8,
    /// Unix timestamp
    pub timestamp: i64,
}

/// Emitted when pool is paused.
#[event]
pub struct PoolPaused {
    pub pool: Pubkey,
    pub authority: Pubkey,
    pub timestamp: i64,
}

/// Emitted when pool is unpaused.
#[event]
pub struct PoolUnpaused {
    pub pool: Pubkey,
    pub authority: Pubkey,
    pub timestamp: i64,
}

/// Emitted when pool authority is transferred.
#[event]
pub struct AuthorityTransferred {
    pub pool: Pubkey,
    pub old_authority: Pubkey,
    pub new_authority: Pubkey,
    pub timestamp: i64,
}
