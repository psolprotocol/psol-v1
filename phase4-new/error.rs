//! Error Types for pSol Privacy Pool - Phase 4

use anchor_lang::prelude::*;

#[error_code]
pub enum PrivacyError {
    #[msg("Invalid proof: verification failed")]
    InvalidProof, // 6000

    #[msg("Invalid proof format: expected 256 bytes")]
    InvalidProofFormat, // 6001

    #[msg("Invalid public inputs for proof verification")]
    InvalidPublicInputs, // 6002

    #[msg("Verification key not configured for this pool")]
    VerificationKeyNotSet, // 6003

    #[msg("Merkle root not in recent history")]
    InvalidMerkleRoot, // 6004

    #[msg("Merkle tree is full")]
    MerkleTreeFull, // 6005

    #[msg("Tree depth must be between 4 and 24")]
    InvalidTreeDepth, // 6006

    #[msg("Root history size must be at least 30")]
    InvalidRootHistorySize, // 6007

    #[msg("Nullifier already spent")]
    NullifierAlreadySpent, // 6008

    #[msg("Invalid nullifier: cannot be all zeros")]
    InvalidNullifier, // 6009

    #[msg("Invalid amount: must be greater than zero")]
    InvalidAmount, // 6010

    #[msg("Insufficient vault balance")]
    InsufficientBalance, // 6011

    #[msg("Token mint does not match pool configuration")]
    InvalidMint, // 6012

    #[msg("Relayer fee exceeds withdrawal amount")]
    RelayerFeeExceedsAmount, // 6013

    #[msg("Invalid commitment: cannot be all zeros")]
    InvalidCommitment, // 6014

    #[msg("Commitment already exists in tree")]
    DuplicateCommitment, // 6015

    #[msg("Invalid secret: cannot be all zeros")]
    InvalidSecret, // 6016

    #[msg("Unauthorized: caller is not pool authority")]
    Unauthorized, // 6017

    #[msg("Pool is paused")]
    PoolPaused, // 6018

    #[msg("Recipient does not match proof public inputs")]
    RecipientMismatch, // 6019

    #[msg("Arithmetic overflow")]
    ArithmeticOverflow, // 6020

    #[msg("Feature not implemented in this version")]
    NotImplemented, // 6021

    #[msg("ZK verification not yet implemented")]
    CryptoNotImplemented, // 6022

    // Phase 4 New Errors
    #[msg("Verification key is locked and cannot be modified")]
    VerificationKeyLocked, // 6023

    #[msg("Invalid authority address")]
    InvalidAuthority, // 6024

    #[msg("No pending authority transfer")]
    NoPendingAuthority, // 6025

    #[msg("Account already initialized")]
    AlreadyInitialized, // 6026

    #[msg("Input exceeds maximum allowed length")]
    InputTooLarge, // 6027

    #[msg("Pool has active deposits")]
    PoolHasDeposits, // 6028

    #[msg("Invalid account owner")]
    InvalidOwner, // 6029

    #[msg("Account data corrupted")]
    CorruptedData, // 6030

    #[msg("Operation exceeds safe limits")]
    LimitExceeded, // 6031

    #[msg("Invalid timestamp")]
    InvalidTimestamp, // 6032
}
