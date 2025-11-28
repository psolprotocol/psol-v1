//! State Account Definitions for pSol Privacy Pool
//!
//! # Account Overview
//!
//! ## Pool Configuration (`PoolConfig`)
//! - PDA Seeds: `["pool", token_mint]`
//! - Stores pool settings, authority, and statistics
//! - Controls pause state and VK configuration status
//!
//! ## Merkle Tree (`MerkleTree`)
//! - PDA Seeds: `["merkle_tree", pool_config]`
//! - Incremental Merkle tree for commitment storage
//! - Uses filled_subtrees pattern for O(log n) insertions
//! - Maintains root history for withdrawal proofs
//!
//! ## Verification Key (`VerificationKeyAccount`)
//! - PDA Seeds: `["verification_key", pool_config]`
//! - Stores Groth16 verification key from trusted setup
//! - Contains α, β, γ, δ points and IC array
//!
//! ## Spent Nullifier (`SpentNullifier`)
//! - PDA Seeds: `["nullifier", pool_config, nullifier_hash]`
//! - Per-nullifier account for O(1) double-spend detection
//! - Created during withdrawal, existence = spent

pub mod merkle_tree;
pub mod pool_config;
pub mod spent_nullifier;
pub mod verification_key;

pub use merkle_tree::MerkleTree;
pub use pool_config::PoolConfig;
pub use spent_nullifier::SpentNullifier;
pub use verification_key::{VerificationKey, VerificationKeyAccount};
