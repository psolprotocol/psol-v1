//! Cryptographic primitives for pSol Privacy Pool
//!
//! # Phase 3 Implementation
//!
//! This module provides production-ready cryptographic operations:
//! - **Poseidon hashing** - For commitments, nullifiers, and Merkle tree
//! - **BN254 curve operations** - For Groth16 verification
//! - **Groth16 verifier** - ZK proof verification
//!
//! ## Security Model
//! - All verification functions are fail-closed
//! - Invalid proofs are always rejected
//! - Dev mode is feature-gated and clearly marked
//!
//! ## Circuit Compatibility
//! All hash functions and field operations MUST match the circomlib
//! implementations used in the withdrawal circuit.

pub mod curve_utils;
pub mod groth16_verifier;
pub mod poseidon;
pub mod public_inputs;

// Re-export commonly used items
pub use curve_utils::{
    compute_vk_x, g1_add, g1_scalar_mul, is_g1_identity, is_g2_identity,
    make_pairing_element, negate_g1, pubkey_to_scalar, u64_to_scalar,
    validate_g1_point, validate_g2_point, verify_pairing,
    G1Point, G2Point, PairingElement, ScalarField,
    G1_IDENTITY, G2_IDENTITY, BN254_FIELD_MODULUS, BN254_SCALAR_MODULUS,
};
pub use groth16_verifier::{verify_groth16_proof, Groth16Proof, PROOF_DATA_LEN};
pub use poseidon::{
    empty_leaf_hash, hash_commitment, hash_nullifier, hash_two_to_one,
    is_zero_hash, u64_to_bytes32, u64_to_bytes32_be,
};
pub use public_inputs::{ZkPublicInputs, ZkPublicInputsBuilder};
