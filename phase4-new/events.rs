//! Events for pSol Privacy Pool - Phase 4

use anchor_lang::prelude::*;

#[event]
pub struct PoolInitialized {
    pub pool: Pubkey,
    pub authority: Pubkey,
    pub token_mint: Pubkey,
    pub tree_depth: u8,
    pub root_history_size: u16,
    pub timestamp: i64,
}

#[event]
pub struct PoolPaused {
    pub pool: Pubkey,
    pub authority: Pubkey,
    pub timestamp: i64,
}

#[event]
pub struct PoolUnpaused {
    pub pool: Pubkey,
    pub authority: Pubkey,
    pub timestamp: i64,
}

#[event]
pub struct AuthorityTransferInitiated {
    pub pool: Pubkey,
    pub current_authority: Pubkey,
    pub pending_authority: Pubkey,
    pub timestamp: i64,
}

#[event]
pub struct AuthorityTransferCompleted {
    pub pool: Pubkey,
    pub old_authority: Pubkey,
    pub new_authority: Pubkey,
    pub timestamp: i64,
}

#[event]
pub struct AuthorityTransferCancelled {
    pub pool: Pubkey,
    pub authority: Pubkey,
    pub cancelled_pending: Pubkey,
    pub timestamp: i64,
}

#[event]
pub struct VerificationKeySet {
    pub pool: Pubkey,
    pub authority: Pubkey,
    pub ic_length: u8,
    pub timestamp: i64,
}

#[event]
pub struct VerificationKeyLocked {
    pub pool: Pubkey,
    pub authority: Pubkey,
    pub timestamp: i64,
}

#[event]
pub struct DepositEvent {
    pub pool: Pubkey,
    pub commitment: [u8; 32],
    pub leaf_index: u32,
    pub amount: u64,
    pub timestamp: i64,
}

#[event]
pub struct WithdrawEvent {
    pub pool: Pubkey,
    pub nullifier_hash: [u8; 32],
    pub recipient: Pubkey,
    pub amount: u64,
    pub relayer: Pubkey,
    pub relayer_fee: u64,
    pub timestamp: i64,
}

#[event]
pub struct TransferEvent {
    pub pool: Pubkey,
    pub nullifier_hash_0: [u8; 32],
    pub nullifier_hash_1: [u8; 32],
    pub output_commitment_0: [u8; 32],
    pub output_commitment_1: [u8; 32],
    pub fee: u64,
    pub fee_recipient: Pubkey,
    pub timestamp: i64,
}
