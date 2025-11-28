//! Unified error types for pSol Privacy Pool
//!
//! Error codes are stable across versions for client compatibility.

use anchor_lang::prelude::*;

#[error_code]
pub enum PrivacyError {
    // ========== Proof Errors (6000-6009) ==========
    
    /// ZK proof verification failed or not yet implemented
    #[msg("Invalid proof: verification failed or not implemented")]
    InvalidProof, // 6000

    /// Proof data has incorrect format or length
    #[msg("Invalid proof format: expected 256 bytes (A: 64, B: 128, C: 64)")]
    InvalidProofFormat, // 6001

    /// Public inputs do not match expected format
    #[msg("Invalid public inputs for proof verification")]
    InvalidPublicInputs, // 6002

    /// Verification key not set or invalid
    #[msg("Verification key not configured for this pool")]
    VerificationKeyNotSet, // 6003

    // ========== Merkle Tree Errors (6010-6019) ==========

    /// Merkle root not found in recent history
    #[msg("Merkle root not in recent history")]
    InvalidMerkleRoot, // 6004

    /// Merkle tree has reached maximum capacity
    #[msg("Merkle tree is full")]
    MerkleTreeFull, // 6005

    /// Invalid tree depth parameter
    #[msg("Tree depth must be between 4 and 24")]
    InvalidTreeDepth, // 6006

    /// Root history size too small
    #[msg("Root history size must be at least 30")]
    InvalidRootHistorySize, // 6007

    // ========== Nullifier Errors (6020-6029) ==========

    /// Nullifier has already been spent (double-spend attempt)
    #[msg("Nullifier already spent")]
    NullifierAlreadySpent, // 6008

    /// Invalid nullifier format
    #[msg("Invalid nullifier: cannot be all zeros")]
    InvalidNullifier, // 6009

    // ========== Amount / Token Errors (6030-6039) ==========

    /// Amount must be greater than zero
    #[msg("Invalid amount: must be greater than zero")]
    InvalidAmount, // 6010

    /// Vault has insufficient balance for withdrawal
    #[msg("Insufficient vault balance")]
    InsufficientBalance, // 6011

    /// Token mint mismatch
    #[msg("Token mint does not match pool configuration")]
    InvalidMint, // 6012

    /// Relayer fee exceeds withdrawal amount
    #[msg("Relayer fee exceeds withdrawal amount")]
    RelayerFeeExceedsAmount, // 6013

    // ========== Commitment Errors (6040-6049) ==========

    /// Invalid commitment (cannot be zero)
    #[msg("Invalid commitment: cannot be all zeros")]
    InvalidCommitment, // 6014

    /// Duplicate commitment detected
    #[msg("Commitment already exists in tree")]
    DuplicateCommitment, // 6015

    /// Secret cannot be all zeros
    #[msg("Invalid secret: cannot be all zeros")]
    InvalidSecret, // 6016

    // ========== Authorization Errors (6050-6059) ==========

    /// Operation not authorized for caller
    #[msg("Unauthorized: caller is not pool authority")]
    Unauthorized, // 6017

    /// Pool is paused
    #[msg("Pool is paused")]
    PoolPaused, // 6018

    /// Recipient mismatch with proof public inputs
    #[msg("Recipient does not match proof public inputs")]
    RecipientMismatch, // 6019

    // ========== Overflow / Computation Errors (6060-6069) ==========

    /// Arithmetic overflow occurred
    #[msg("Arithmetic overflow")]
    ArithmeticOverflow, // 6020

    // ========== Implementation Status (6070-6079) ==========

    /// Feature not yet implemented
    #[msg("Feature not implemented in this version")]
    NotImplemented, // 6021

    /// Cryptographic verification not yet available
    #[msg("ZK verification not yet implemented - withdrawals disabled")]
    CryptoNotImplemented, // 6022
}
