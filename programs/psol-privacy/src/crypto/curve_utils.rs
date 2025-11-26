//! Elliptic Curve Utility Functions
//!
//! # PHASE 2 STATUS: PLACEHOLDER
//!
//! This module will contain utilities for BN254 curve operations:
//! - Point validation (on-curve checks)
//! - Point encoding/decoding
//! - Subgroup checks
//!
//! # Phase 3 Implementation
//! Implement using Solana's alt_bn128 precompiles or a Rust library
//! like `ark-bn254`.

use anchor_lang::prelude::*;

use crate::error::PrivacyError;

// ============================================================================
// BN254 CURVE PARAMETERS
// ============================================================================

/// BN254 base field modulus (p)
/// p = 21888242871839275222246405745257275088696311157297823662689037894645226208583
#[allow(dead_code)]
pub const BN254_FIELD_MODULUS: [u8; 32] = [
    0x47, 0xfd, 0x7c, 0xd8, 0x16, 0x8c, 0x20, 0x3c,
    0x8d, 0xca, 0x71, 0x68, 0x91, 0x6a, 0x81, 0x97,
    0x5d, 0x58, 0x81, 0x81, 0xb6, 0x45, 0x50, 0xb8,
    0x29, 0xa0, 0x31, 0xe1, 0x72, 0x4e, 0x64, 0x30,
];

/// BN254 scalar field modulus (r) - order of G1
/// r = 21888242871839275222246405745257275088548364400416034343698204186575808495617
#[allow(dead_code)]
pub const BN254_SCALAR_MODULUS: [u8; 32] = [
    0x01, 0x00, 0x00, 0xf0, 0x93, 0xf5, 0xe1, 0x43,
    0x91, 0x70, 0xb9, 0x79, 0x48, 0xe8, 0x33, 0x28,
    0x5d, 0x58, 0x81, 0x81, 0xb6, 0x45, 0x50, 0xb8,
    0x29, 0xa0, 0x31, 0xe1, 0x72, 0x4e, 0x64, 0x30,
];

// ============================================================================
// G1 POINT OPERATIONS
// ============================================================================

/// G1 point in uncompressed form (64 bytes: x || y).
pub type G1Point = [u8; 64];

/// Check if a G1 point is the identity (point at infinity).
pub fn is_g1_identity(point: &G1Point) -> bool {
    point.iter().all(|&b| b == 0)
}

/// Validate that a G1 point is on the BN254 curve.
///
/// # PHASE 3 TODO
/// Implement the curve equation check: y² = x³ + 3 (mod p)
#[allow(dead_code)]
pub fn validate_g1_point(_point: &G1Point) -> Result<()> {
    // TODO [PHASE 3]: Implement curve check
    //
    // 1. Extract x, y coordinates from point bytes
    // 2. Check y² ≡ x³ + 3 (mod p)
    // 3. Optionally check subgroup membership
    //
    // For now, just check non-zero
    // if is_g1_identity(point) {
    //     return Err(error!(PrivacyError::InvalidProof));
    // }
    
    Ok(())
}

/// Negate a G1 point (used in pairing verification).
///
/// For BN254: -P = (x, -y mod p)
///
/// # PHASE 3 TODO
/// Implement proper field negation
#[allow(dead_code)]
pub fn negate_g1(point: &G1Point) -> Result<G1Point> {
    if is_g1_identity(point) {
        return Ok(*point); // -O = O
    }
    
    // TODO [PHASE 3]: Implement proper negation
    // 1. Extract y coordinate (bytes 32-63)
    // 2. Compute -y mod p
    // 3. Return (x, -y)
    
    Err(error!(PrivacyError::CryptoNotImplemented))
}

// ============================================================================
// G2 POINT OPERATIONS
// ============================================================================

/// G2 point in uncompressed form (128 bytes).
/// G2 points are over the extension field Fp2.
pub type G2Point = [u8; 128];

/// Check if a G2 point is the identity.
pub fn is_g2_identity(point: &G2Point) -> bool {
    point.iter().all(|&b| b == 0)
}

/// Validate that a G2 point is on the curve.
///
/// # PHASE 3 TODO
/// Implement curve check over Fp2
#[allow(dead_code)]
pub fn validate_g2_point(_point: &G2Point) -> Result<()> {
    // TODO [PHASE 3]: Implement curve check for G2
    Ok(())
}

// ============================================================================
// SCALAR FIELD OPERATIONS
// ============================================================================

/// Scalar field element (32 bytes, little-endian).
pub type ScalarField = [u8; 32];

/// Check if scalar is less than the field modulus.
///
/// # PHASE 3 TODO
/// Implement proper modular comparison
#[allow(dead_code)]
pub fn is_valid_scalar(_scalar: &ScalarField) -> bool {
    // TODO [PHASE 3]: Check scalar < r
    true
}

/// Convert u64 to scalar field element.
pub fn u64_to_scalar(value: u64) -> ScalarField {
    let mut scalar = [0u8; 32];
    scalar[..8].copy_from_slice(&value.to_le_bytes());
    scalar
}

/// Convert Pubkey to scalar field element.
/// Simply uses the 32-byte representation.
pub fn pubkey_to_scalar(pubkey: &Pubkey) -> ScalarField {
    pubkey.to_bytes()
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_g1_identity() {
        let zero = [0u8; 64];
        assert!(is_g1_identity(&zero));
        
        let non_zero = [1u8; 64];
        assert!(!is_g1_identity(&non_zero));
    }

    #[test]
    fn test_g2_identity() {
        let zero = [0u8; 128];
        assert!(is_g2_identity(&zero));
    }

    #[test]
    fn test_u64_to_scalar() {
        let scalar = u64_to_scalar(12345);
        let expected = 12345u64.to_le_bytes();
        assert_eq!(&scalar[..8], &expected);
        assert!(scalar[8..].iter().all(|&b| b == 0));
    }
}
