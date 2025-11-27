//! Groth16 Zero-Knowledge Proof Verifier
//!
//! # Phase 3 Implementation
//!
//! This module implements Groth16 proof verification for the pSol privacy pool.
//! It uses Solana's alt_bn128 precompiles for efficient pairing operations.
//!
//! ## Verification Equation
//! The Groth16 verification equation is:
//! ```text
//! e(A, B) = e(α, β) · e(vk_x, γ) · e(C, δ)
//! ```
//!
//! Which can be rewritten as a single pairing check:
//! ```text
//! e(-A, B) · e(α, β) · e(vk_x, γ) · e(C, δ) = 1
//! ```
//!
//! Where:
//! - (A, B, C) are the proof elements
//! - (α, β, γ, δ) are from the verification key
//! - vk_x = IC[0] + Σ(public_input[i] · IC[i+1])
//!
//! ## Dev Mode
//! When compiled with the `dev-mode` feature, proof verification can be
//! bypassed for testing. This feature MUST NEVER be enabled in production.
//!
//! ## References
//! - Groth16 paper: https://eprint.iacr.org/2016/260
//! - Solana alt_bn128: solana_program::alt_bn128

use anchor_lang::prelude::*;

use crate::error::PrivacyError;
use crate::state::verification_key::VerificationKey;

use super::curve_utils::{
    compute_vk_x, is_g1_identity, is_g2_identity, make_pairing_element,
    negate_g1, validate_g1_point, validate_g2_point, verify_pairing,
    G1Point, G2Point, PairingElement,
};
use super::public_inputs::ZkPublicInputs;

// ============================================================================
// PROOF STRUCTURE
// ============================================================================

/// Expected proof data length in bytes.
/// A = 64 bytes (G1 uncompressed)
/// B = 128 bytes (G2 uncompressed)
/// C = 64 bytes (G1 uncompressed)
pub const PROOF_DATA_LEN: usize = 256;

/// Groth16 proof structure.
///
/// A Groth16 proof consists of three curve points: (A, B, C)
/// where A, C ∈ G1 and B ∈ G2.
///
/// ## Encoding
/// All points are in uncompressed big-endian format:
/// - G1 points: 64 bytes (32 bytes x, 32 bytes y)
/// - G2 points: 128 bytes (64 bytes x, 64 bytes y)
#[derive(Clone, Debug)]
pub struct Groth16Proof {
    /// Point A ∈ G1 (uncompressed, 64 bytes)
    pub a: G1Point,
    
    /// Point B ∈ G2 (uncompressed, 128 bytes)
    pub b: G2Point,
    
    /// Point C ∈ G1 (uncompressed, 64 bytes)
    pub c: G1Point,
}

impl Groth16Proof {
    /// Parse proof from raw bytes.
    ///
    /// # Arguments
    /// * `data` - Raw proof bytes (256 bytes expected)
    ///
    /// # Returns
    /// Parsed proof structure or error
    ///
    /// # Layout
    /// ```text
    /// [0..64]    - A (G1 point)
    /// [64..192]  - B (G2 point)
    /// [192..256] - C (G1 point)
    /// ```
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        require!(
            data.len() == PROOF_DATA_LEN,
            PrivacyError::InvalidProofFormat
        );

        let mut proof = Groth16Proof {
            a: [0u8; 64],
            b: [0u8; 128],
            c: [0u8; 64],
        };

        proof.a.copy_from_slice(&data[0..64]);
        proof.b.copy_from_slice(&data[64..192]);
        proof.c.copy_from_slice(&data[192..256]);

        Ok(proof)
    }

    /// Serialize proof to bytes.
    pub fn to_bytes(&self) -> [u8; PROOF_DATA_LEN] {
        let mut bytes = [0u8; PROOF_DATA_LEN];
        bytes[0..64].copy_from_slice(&self.a);
        bytes[64..192].copy_from_slice(&self.b);
        bytes[192..256].copy_from_slice(&self.c);
        bytes
    }
}

// ============================================================================
// VERIFICATION FUNCTION
// ============================================================================

/// Verify a Groth16 zero-knowledge proof.
///
/// # Algorithm
/// 1. Parse and validate proof points
/// 2. Validate verification key
/// 3. Encode public inputs as field elements
/// 4. Compute vk_x = IC[0] + Σ(public_input[i] · IC[i+1])
/// 5. Compute pairing: e(-A, B) · e(α, β) · e(vk_x, γ) · e(C, δ) = 1
///
/// # Arguments
/// * `proof_bytes` - Raw proof data (256 bytes)
/// * `vk` - Verification key from trusted setup
/// * `public_inputs` - Public inputs to the circuit
///
/// # Returns
/// * `Ok(true)` - Proof is valid
/// * `Err(...)` - Proof is invalid or verification failed
///
/// # Security
/// - This function is cryptographically critical
/// - Invalid proofs MUST always be rejected
/// - The verification key must come from a trusted setup
///
/// # Dev Mode
/// When compiled with `dev-mode` feature, returns Ok(true) without
/// performing cryptographic verification. NEVER use in production.
pub fn verify_groth16_proof(
    proof_bytes: &[u8],
    vk: &VerificationKey,
    public_inputs: &ZkPublicInputs,
) -> Result<bool> {
    // Dev mode bypass (ONLY for testing)
    #[cfg(feature = "dev-mode")]
    {
        msg!("⚠️  DEV MODE: Proof verification bypassed!");
        msg!("⚠️  This build is NOT safe for production!");
        
        // Still validate inputs to catch obvious errors
        let _ = Groth16Proof::from_bytes(proof_bytes)?;
        validate_verification_key(vk)?;
        public_inputs.validate()?;
        
        return Ok(true);
    }

    // Production verification
    #[cfg(not(feature = "dev-mode"))]
    {
        verify_groth16_proof_impl(proof_bytes, vk, public_inputs)
    }
}

/// Internal implementation of Groth16 verification.
///
/// This function performs the full cryptographic verification.
#[cfg(not(feature = "dev-mode"))]
fn verify_groth16_proof_impl(
    proof_bytes: &[u8],
    vk: &VerificationKey,
    public_inputs: &ZkPublicInputs,
) -> Result<bool> {
    msg!("Groth16 verification starting...");

    // Step 1: Parse proof structure
    let proof = Groth16Proof::from_bytes(proof_bytes)?;
    msg!("Proof parsed successfully");

    // Step 2: Validate proof points are on curve and not identity
    validate_proof_points(&proof)?;
    msg!("Proof points validated");

    // Step 3: Validate VK is properly configured
    validate_verification_key(vk)?;
    msg!("Verification key validated");

    // Step 4: Validate and encode public inputs
    public_inputs.validate()?;
    let encoded_inputs = public_inputs.to_field_elements();
    msg!("Public inputs encoded: {} elements", encoded_inputs.len());

    // Step 5: Compute vk_x = IC[0] + Σ(input[i] * IC[i+1])
    let vk_x = compute_vk_x(&vk.ic, &encoded_inputs)?;
    msg!("vk_x computed");

    // Step 6: Negate A for pairing equation
    let neg_a = negate_g1(&proof.a)?;
    msg!("A negated");

    // Step 7: Construct pairing elements
    // Verification equation: e(-A, B) · e(α, β) · e(vk_x, γ) · e(C, δ) = 1
    let pairing_elements: [PairingElement; 4] = [
        make_pairing_element(&neg_a, &proof.b),           // e(-A, B)
        make_pairing_element(&vk.alpha_g1, &vk.beta_g2),  // e(α, β)
        make_pairing_element(&vk_x, &vk.gamma_g2),        // e(vk_x, γ)
        make_pairing_element(&proof.c, &vk.delta_g2),     // e(C, δ)
    ];

    // Step 8: Verify pairing
    msg!("Performing pairing check...");
    let result = verify_pairing(&pairing_elements)?;

    if result {
        msg!("✓ Proof verified successfully");
    } else {
        msg!("✗ Proof verification failed");
    }

    Ok(result)
}

// ============================================================================
// VALIDATION HELPERS
// ============================================================================

/// Validate that proof points are well-formed.
///
/// Checks:
/// 1. A ∈ G1 is not identity and on curve
/// 2. B ∈ G2 is not identity and valid
/// 3. C ∈ G1 is not identity and on curve
fn validate_proof_points(proof: &Groth16Proof) -> Result<()> {
    // Check A is not identity
    require!(
        !is_g1_identity(&proof.a),
        PrivacyError::InvalidProof
    );
    
    // Validate A is on curve
    validate_g1_point(&proof.a)?;

    // Check B is not identity
    require!(
        !is_g2_identity(&proof.b),
        PrivacyError::InvalidProof
    );
    
    // Validate B (basic check)
    validate_g2_point(&proof.b)?;

    // Check C is not identity
    require!(
        !is_g1_identity(&proof.c),
        PrivacyError::InvalidProof
    );
    
    // Validate C is on curve
    validate_g1_point(&proof.c)?;

    Ok(())
}

/// Validate verification key structure and values.
///
/// Checks:
/// 1. Sufficient IC points for public inputs
/// 2. Alpha is not identity
/// 3. All VK points are valid (basic validation)
fn validate_verification_key(vk: &VerificationKey) -> Result<()> {
    // Must have at least 2 IC points (1 base + 1 for at least 1 public input)
    require!(
        vk.ic.len() >= 2,
        PrivacyError::VerificationKeyNotSet
    );

    // For withdrawal circuit with 6 public inputs, we need 7 IC points
    require!(
        vk.ic.len() == ZkPublicInputs::COUNT + 1,
        PrivacyError::InvalidPublicInputs
    );

    // Alpha must not be identity
    require!(
        !is_g1_identity(&vk.alpha_g1),
        PrivacyError::VerificationKeyNotSet
    );

    // Validate alpha is on curve
    validate_g1_point(&vk.alpha_g1)?;

    // Validate G2 points
    validate_g2_point(&vk.beta_g2)?;
    validate_g2_point(&vk.gamma_g2)?;
    validate_g2_point(&vk.delta_g2)?;

    // Validate each IC point
    for (i, ic_point) in vk.ic.iter().enumerate() {
        if is_g1_identity(ic_point) && i == 0 {
            // IC[0] can technically be identity, but it's unusual
            msg!("Warning: IC[0] is identity point");
        }
        validate_g1_point(ic_point)?;
    }

    Ok(())
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proof_parsing() {
        let data = [1u8; PROOF_DATA_LEN];
        let proof = Groth16Proof::from_bytes(&data).unwrap();
        
        assert_eq!(proof.a, [1u8; 64]);
        assert_eq!(proof.b, [1u8; 128]);
        assert_eq!(proof.c, [1u8; 64]);
    }

    #[test]
    fn test_proof_roundtrip() {
        let data = [42u8; PROOF_DATA_LEN];
        let proof = Groth16Proof::from_bytes(&data).unwrap();
        let back = proof.to_bytes();
        
        assert_eq!(data, back);
    }

    #[test]
    fn test_invalid_proof_length() {
        let data = [1u8; 100]; // Too short
        let result = Groth16Proof::from_bytes(&data);
        
        assert!(result.is_err());
    }

    #[test]
    fn test_proof_length_too_long() {
        let data = [1u8; 300]; // Too long
        let result = Groth16Proof::from_bytes(&data);
        
        assert!(result.is_err());
    }

    // Note: Full verification tests require valid VK and proofs from a circuit
    // Those are typically done in integration tests with snarkjs or similar
}
