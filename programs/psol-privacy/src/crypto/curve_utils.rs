//! BN254 Elliptic Curve Operations for Groth16 Verification
//!
//! # Phase 3 Implementation
//!
//! This module provides BN254 curve operations required for Groth16 proof
//! verification, using Solana's alt_bn128 precompiles where possible.
//!
//! ## Curve Parameters
//! BN254 (also known as alt_bn128) is a pairing-friendly elliptic curve:
//! - Base field: Fp with p = 21888242871839275222246405745257275088696311157297823662689037894645226208583
//! - Scalar field: Fr with r = 21888242871839275222246405745257275088548364400416034343698204186575808495617
//! - Curve equation: y² = x³ + 3
//!
//! ## Point Representations
//! - G1 points: 64 bytes (32 bytes x, 32 bytes y) - big-endian
//! - G2 points: 128 bytes (64 bytes x, 64 bytes y) - big-endian
//!
//! ## Solana Precompiles Used
//! - `sol_alt_bn128_g1_add` - G1 point addition
//! - `sol_alt_bn128_g1_multiply` - G1 scalar multiplication
//! - `sol_alt_bn128_pairing` - Pairing check

use anchor_lang::prelude::*;
use num_bigint::BigUint;
use num_traits::Zero;
use solana_program::alt_bn128::{
    prelude::{
        alt_bn128_addition, alt_bn128_multiplication, alt_bn128_pairing,
    },
    AltBn128Error,
};

use crate::error::PrivacyError;

// ============================================================================
// BN254 CURVE PARAMETERS
// ============================================================================

/// BN254 base field modulus (p) - big-endian bytes
/// p = 21888242871839275222246405745257275088696311157297823662689037894645226208583
pub const BN254_FIELD_MODULUS: [u8; 32] = [
    0x30, 0x64, 0x4e, 0x72, 0xe1, 0x31, 0xa0, 0x29,
    0xb8, 0x50, 0x45, 0xb6, 0x81, 0x81, 0x58, 0x5d,
    0x97, 0x81, 0x6a, 0x91, 0x68, 0x71, 0xca, 0x8d,
    0x3c, 0x20, 0x8c, 0x16, 0xd8, 0x7c, 0xfd, 0x47,
];

/// BN254 scalar field modulus (r) - order of G1 - big-endian bytes
/// r = 21888242871839275222246405745257275088548364400416034343698204186575808495617
pub const BN254_SCALAR_MODULUS: [u8; 32] = [
    0x30, 0x64, 0x4e, 0x72, 0xe1, 0x31, 0xa0, 0x29,
    0xb8, 0x50, 0x45, 0xb6, 0x81, 0x81, 0x58, 0x5d,
    0x28, 0x33, 0xe8, 0x48, 0x79, 0xb9, 0x70, 0x91,
    0x43, 0xe1, 0xf5, 0x93, 0xf0, 0x00, 0x00, 0x01,
];

/// G1 generator point (uncompressed)
pub const G1_GENERATOR: [u8; 64] = [
    // x = 1
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
    // y = 2
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02,
];

// ============================================================================
// G1 POINT OPERATIONS
// ============================================================================

/// G1 point in uncompressed form (64 bytes: x || y, big-endian).
pub type G1Point = [u8; 64];

/// G1 identity (point at infinity) - all zeros.
pub const G1_IDENTITY: G1Point = [0u8; 64];

/// Check if a G1 point is the identity (point at infinity).
pub fn is_g1_identity(point: &G1Point) -> bool {
    point.iter().all(|&b| b == 0)
}

/// Validate that a G1 point is on the BN254 curve.
///
/// Checks:
/// 1. Point is not all zeros (unless identity)
/// 2. Coordinates are in valid field range
/// 3. Point satisfies curve equation y² = x³ + 3
///
/// # Arguments
/// * `point` - G1 point (64 bytes, big-endian x || y)
///
/// # Returns
/// * `Ok(())` if point is valid
/// * `Err(InvalidProof)` if point is invalid
pub fn validate_g1_point(point: &G1Point) -> Result<()> {
    // Identity is valid
    if is_g1_identity(point) {
        return Ok(());
    }

    // Extract coordinates
    let x = BigUint::from_bytes_be(&point[0..32]);
    let y = BigUint::from_bytes_be(&point[32..64]);
    let p = BigUint::from_bytes_be(&BN254_FIELD_MODULUS);

    // Check coordinates are less than field modulus
    require!(x < p, PrivacyError::InvalidProof);
    require!(y < p, PrivacyError::InvalidProof);

    // Check curve equation: y² = x³ + 3 (mod p)
    let y_squared = (&y * &y) % &p;
    let x_cubed = (&x * &x * &x) % &p;
    let three = BigUint::from(3u32);
    let rhs = (x_cubed + three) % &p;

    require!(y_squared == rhs, PrivacyError::InvalidProof);

    Ok(())
}

/// Negate a G1 point (used in pairing verification).
///
/// For BN254: -P = (x, -y mod p) = (x, p - y)
///
/// # Arguments
/// * `point` - G1 point to negate
///
/// # Returns
/// Negated G1 point
pub fn negate_g1(point: &G1Point) -> Result<G1Point> {
    // Identity negates to itself
    if is_g1_identity(point) {
        return Ok(*point);
    }

    // Extract y coordinate
    let y = BigUint::from_bytes_be(&point[32..64]);
    let p = BigUint::from_bytes_be(&BN254_FIELD_MODULUS);

    // Compute -y = p - y
    let neg_y = if y.is_zero() {
        BigUint::zero()
    } else {
        &p - &y
    };

    // Construct result
    let mut result = [0u8; 64];
    result[0..32].copy_from_slice(&point[0..32]); // x unchanged

    // Convert neg_y back to 32 bytes (big-endian, zero-padded)
    let neg_y_bytes = neg_y.to_bytes_be();
    let start = 32 - neg_y_bytes.len().min(32);
    result[32 + start..64].copy_from_slice(&neg_y_bytes[neg_y_bytes.len().saturating_sub(32)..]);

    Ok(result)
}

/// Add two G1 points using Solana's alt_bn128_addition precompile.
///
/// # Arguments
/// * `a` - First G1 point
/// * `b` - Second G1 point
///
/// # Returns
/// Sum of the two points (a + b)
pub fn g1_add(a: &G1Point, b: &G1Point) -> Result<G1Point> {
    // Prepare input: concatenate both points
    let mut input = [0u8; 128];
    input[0..64].copy_from_slice(a);
    input[64..128].copy_from_slice(b);

    // Call precompile
    let result = alt_bn128_addition(&input)
        .map_err(|e| map_bn128_error(e))?;

    let mut output = [0u8; 64];
    output.copy_from_slice(&result);
    Ok(output)
}

/// Multiply a G1 point by a scalar using Solana's alt_bn128_multiplication precompile.
///
/// # Arguments
/// * `point` - G1 point
/// * `scalar` - 32-byte scalar (big-endian)
///
/// # Returns
/// Scalar multiple (scalar * point)
pub fn g1_scalar_mul(point: &G1Point, scalar: &[u8; 32]) -> Result<G1Point> {
    // Prepare input: point || scalar
    let mut input = [0u8; 96];
    input[0..64].copy_from_slice(point);
    input[64..96].copy_from_slice(scalar);

    // Call precompile
    let result = alt_bn128_multiplication(&input)
        .map_err(|e| map_bn128_error(e))?;

    let mut output = [0u8; 64];
    output.copy_from_slice(&result);
    Ok(output)
}

// ============================================================================
// G2 POINT OPERATIONS
// ============================================================================

/// G2 point in uncompressed form (128 bytes).
/// G2 points are over the extension field Fp2.
pub type G2Point = [u8; 128];

/// G2 identity (point at infinity).
pub const G2_IDENTITY: G2Point = [0u8; 128];

/// Check if a G2 point is the identity.
pub fn is_g2_identity(point: &G2Point) -> bool {
    point.iter().all(|&b| b == 0)
}

/// Basic validation for G2 point (checks non-zero and field range).
///
/// Note: Full on-curve validation for G2 is more complex due to Fp2 arithmetic.
/// This function performs basic sanity checks.
pub fn validate_g2_point(point: &G2Point) -> Result<()> {
    // Identity is valid
    if is_g2_identity(point) {
        return Ok(());
    }

    // Check all coordinate components are in field range
    let p = BigUint::from_bytes_be(&BN254_FIELD_MODULUS);
    
    // G2 point has coordinates (x, y) where x, y ∈ Fp2
    // Each Fp2 element is represented as two Fp elements
    // Layout: x_c0 (32) || x_c1 (32) || y_c0 (32) || y_c1 (32)
    for i in 0..4 {
        let start = i * 32;
        let component = BigUint::from_bytes_be(&point[start..start + 32]);
        require!(component < p, PrivacyError::InvalidProof);
    }

    Ok(())
}

// ============================================================================
// SCALAR FIELD OPERATIONS
// ============================================================================

/// Scalar field element (32 bytes, big-endian).
pub type ScalarField = [u8; 32];

/// Check if scalar is less than the field modulus.
pub fn is_valid_scalar(scalar: &ScalarField) -> bool {
    let s = BigUint::from_bytes_be(scalar);
    let r = BigUint::from_bytes_be(&BN254_SCALAR_MODULUS);
    s < r
}

/// Convert u64 to scalar field element (big-endian).
pub fn u64_to_scalar(value: u64) -> ScalarField {
    let mut scalar = [0u8; 32];
    scalar[24..32].copy_from_slice(&value.to_be_bytes());
    scalar
}

/// Convert Pubkey to scalar field element.
pub fn pubkey_to_scalar(pubkey: &Pubkey) -> ScalarField {
    pubkey.to_bytes()
}

// ============================================================================
// PAIRING OPERATIONS
// ============================================================================

/// Input element for pairing operation (G1 point || G2 point = 192 bytes).
pub type PairingElement = [u8; 192];

/// Verify a pairing equation using Solana's alt_bn128_pairing precompile.
///
/// The pairing check verifies:
/// ```text
/// ∏ e(G1[i], G2[i]) = 1
/// ```
///
/// This is equivalent to checking if the sum of pairings equals zero in GT.
///
/// # Arguments
/// * `elements` - Slice of (G1, G2) pairs to verify
///
/// # Returns
/// * `Ok(true)` if pairing check passes (product = 1)
/// * `Ok(false)` if pairing check fails
/// * `Err(...)` on computation error
pub fn verify_pairing(elements: &[PairingElement]) -> Result<bool> {
    if elements.is_empty() {
        return Ok(true); // Empty product is 1
    }

    // Concatenate all elements
    let mut input = Vec::with_capacity(elements.len() * 192);
    for elem in elements {
        input.extend_from_slice(elem);
    }

    // Call pairing precompile
    let result = alt_bn128_pairing(&input)
        .map_err(|e| map_bn128_error(e))?;

    // Result is 32 bytes: 1 if pairing check passes, 0 otherwise
    Ok(result[31] == 1 && result[..31].iter().all(|&b| b == 0))
}

/// Construct a pairing element from G1 and G2 points.
pub fn make_pairing_element(g1: &G1Point, g2: &G2Point) -> PairingElement {
    let mut element = [0u8; 192];
    element[0..64].copy_from_slice(g1);
    element[64..192].copy_from_slice(g2);
    element
}

// ============================================================================
// VK_X COMPUTATION
// ============================================================================

/// Compute vk_x = IC[0] + Σ(public_input[i] * IC[i+1]) for Groth16 verification.
///
/// This computes the linear combination of IC points weighted by public inputs.
///
/// # Arguments
/// * `ic` - IC points from verification key (IC[0], IC[1], ..., IC[n])
/// * `public_inputs` - Public inputs as field elements (n elements)
///
/// # Returns
/// The computed vk_x point in G1
///
/// # Errors
/// Returns error if:
/// * `ic.len() != public_inputs.len() + 1`
/// * Any curve operation fails
pub fn compute_vk_x(ic: &[[u8; 64]], public_inputs: &[[u8; 32]]) -> Result<G1Point> {
    // Validate lengths
    require!(
        ic.len() == public_inputs.len() + 1,
        PrivacyError::InvalidPublicInputs
    );

    // Start with IC[0]
    let mut acc = ic[0];

    // Add public_input[i] * IC[i+1] for each input
    for (i, input) in public_inputs.iter().enumerate() {
        // Compute input[i] * IC[i+1]
        let term = g1_scalar_mul(&ic[i + 1], input)?;
        
        // Add to accumulator
        acc = g1_add(&acc, &term)?;
    }

    Ok(acc)
}

// ============================================================================
// ERROR HANDLING
// ============================================================================

/// Map alt_bn128 errors to PrivacyError.
fn map_bn128_error(e: AltBn128Error) -> anchor_lang::error::Error {
    msg!("BN254 operation failed: {:?}", e);
    error!(PrivacyError::InvalidProof)
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
        // Last 8 bytes should be big-endian representation
        assert_eq!(&scalar[24..], &12345u64.to_be_bytes());
        // First 24 bytes should be zero
        assert!(scalar[..24].iter().all(|&b| b == 0));
    }

    #[test]
    fn test_negate_identity() {
        let identity = G1_IDENTITY;
        let negated = negate_g1(&identity).unwrap();
        assert_eq!(identity, negated, "-O should equal O");
    }

    #[test]
    fn test_valid_scalar_check() {
        // Zero is valid
        let zero = [0u8; 32];
        assert!(is_valid_scalar(&zero));

        // Small value is valid
        let small = u64_to_scalar(1000);
        assert!(is_valid_scalar(&small));
    }

    #[test]
    fn test_make_pairing_element() {
        let g1 = [1u8; 64];
        let g2 = [2u8; 128];
        
        let elem = make_pairing_element(&g1, &g2);
        
        assert_eq!(&elem[0..64], &g1);
        assert_eq!(&elem[64..192], &g2);
    }

    #[test]
    fn test_g1_generator_on_curve() {
        // The generator (1, 2) should satisfy y² = x³ + 3
        // 2² = 4
        // 1³ + 3 = 4
        // So 4 = 4 ✓
        let result = validate_g1_point(&G1_GENERATOR);
        assert!(result.is_ok(), "Generator should be on curve");
    }
}
