//! Cryptographic Primitives for pSol Privacy Pool - Phase 3
//!
//! # Module Overview
//!
//! ## curve_utils
//! BN254 elliptic curve operations using Solana's alt_bn128 precompiles:
//! - G1/G2 point validation
//! - Scalar multiplication
//! - Pairing operations
//!
//! ## groth16_verifier
//! Production-ready Groth16 proof verification:
//! - Full pairing-based verification
//! - No unsafe bypasses in production
//!
//! ## poseidon
//! Hash functions:
//! - Keccak256 for Merkle tree (on-chain)
//! - Poseidon documentation for commitments (off-chain)
//!
//! ## public_inputs
//! Public input encoding for ZK circuits:
//! - Field element conversion
//! - Input validation
//!
//! # Security Model
//!
//! - All verification functions are fail-closed
//! - Invalid proofs are always rejected
//! - Curve points are validated before use
//! - No dev-mode bypass in production builds

pub mod curve_utils;
pub mod groth16_verifier;
pub mod poseidon;
pub mod public_inputs;

// ============================================================================
// CURVE UTILITIES
// ============================================================================

pub use curve_utils::{
    // Point types
    G1Point, G2Point, PairingElement, ScalarField,
    
    // Constants
    G1_IDENTITY, G2_IDENTITY, G1_GENERATOR,
    BN254_FIELD_MODULUS, BN254_SCALAR_MODULUS,
    
    // G1 operations
    validate_g1_point, negate_g1, g1_add, g1_scalar_mul,
    is_g1_identity,
    
    // G2 operations
    validate_g2_point, is_g2_identity,
    
    // Scalar operations
    is_valid_scalar, u64_to_scalar, pubkey_to_scalar,
    
    // Pairing operations
    verify_pairing, make_pairing_element, compute_vk_x,
};

// ============================================================================
// GROTH16 VERIFIER
// ============================================================================

pub use groth16_verifier::{
    verify_groth16_proof,
    Groth16Proof,
    PROOF_DATA_LEN,
};

// ============================================================================
// HASH FUNCTIONS
// ============================================================================

pub use poseidon::{
    // Merkle tree hash (on-chain, Keccak256)
    hash_two_to_one,
    
    // Utilities
    is_zero_hash,
    empty_leaf_hash,
    u64_to_bytes32,
    u64_to_bytes32_be,
};

// ============================================================================
// PUBLIC INPUTS
// ============================================================================

pub use public_inputs::{
    ZkPublicInputs,
    ZkPublicInputsBuilder,
};

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exports_available() {
        // Verify key exports are available
        let _ = G1_IDENTITY;
        let _ = G2_IDENTITY;
        let _ = PROOF_DATA_LEN;
        
        // Verify functions are callable
        let zero = [0u8; 32];
        assert!(is_zero_hash(&zero));
        
        let g1_zero = [0u8; 64];
        assert!(is_g1_identity(&g1_zero));
    }
}
