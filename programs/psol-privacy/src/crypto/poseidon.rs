//! Hash Functions for pSol Privacy Pool - Phase 3
//!
//! # Hash Function Architecture
//!
//! ## Off-Chain (User/Circuit)
//! Users compute commitments and nullifiers OFF-CHAIN using Poseidon:
//! ```text
//! commitment = Poseidon(secret, nullifier_preimage, amount)
//! nullifier_hash = Poseidon(nullifier_preimage, secret)
//! ```
//!
//! These MUST use circomlib-compatible Poseidon parameters:
//! - Field: BN254 scalar field
//! - t = 3 for commitment (3 inputs)
//! - t = 2 for nullifier (2 inputs)
//! - Rounds: RF=8, RP=57
//!
//! ## On-Chain (Merkle Tree)
//! The Merkle tree uses Keccak256 for internal nodes.
//! This is acceptable because:
//! 1. Merkle tree structure is public (not privacy-sensitive)
//! 2. Keccak256 is available as a Solana syscall (efficient)
//! 3. ZK circuit can support any Merkle tree hash
//!
//! The commitment leaves ARE computed with Poseidon (off-chain),
//! but the tree aggregation uses Keccak256.
//!
//! # Circuit Compatibility
//!
//! Your ZK circuit must be configured to:
//! 1. Use Poseidon for commitment/nullifier verification
//! 2. Use Keccak256 for Merkle path verification
//! 3. Match the exact field element encoding (big-endian)

use solana_program::keccak;

// ============================================================================
// MERKLE TREE HASH (On-Chain, Keccak256)
// ============================================================================

/// Hash two 32-byte values for Merkle tree internal nodes.
///
/// Uses Keccak256 for efficiency on Solana.
/// 
/// # Arguments
/// * `left` - Left child hash
/// * `right` - Right child hash
///
/// # Returns
/// Parent node hash: Keccak256(left || right)
pub fn hash_two_to_one(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let mut combined = [0u8; 64];
    combined[..32].copy_from_slice(left);
    combined[32..].copy_from_slice(right);
    keccak::hash(&combined).to_bytes()
}

// ============================================================================
// COMMITMENT/NULLIFIER (Off-Chain Only - Documentation)
// ============================================================================

/// Compute commitment OFF-CHAIN using Poseidon.
///
/// # ⚠️ THIS IS DOCUMENTATION ONLY
/// This function is NOT called on-chain in Phase 3.
/// Users must compute commitments off-chain using a compatible library.
///
/// # Formula
/// ```text
/// commitment = Poseidon(secret, nullifier_preimage, amount)
/// ```
///
/// # Recommended Libraries
/// - JavaScript: `circomlib` / `snarkjs`
/// - Rust: `light-poseidon` (for off-chain use)
/// - Python: `py_ecc` with Poseidon implementation
///
/// # Parameters (circomlib compatible)
/// - Curve: BN254
/// - Field: Scalar field (Fr)
/// - t = 4 (3 inputs + 1 capacity)
/// - RF = 8 (full rounds)
/// - RP = 57 (partial rounds)
#[allow(dead_code)]
pub fn compute_commitment_offchain(
    _secret: &[u8; 32],
    _nullifier_preimage: &[u8; 32],
    _amount: u64,
) -> [u8; 32] {
    // This should be computed off-chain using:
    // const poseidon = require('circomlib').poseidon;
    // const commitment = poseidon([secret, nullifier_preimage, amount]);
    panic!("Commitments must be computed off-chain using Poseidon")
}

/// Compute nullifier hash OFF-CHAIN using Poseidon.
///
/// # ⚠️ THIS IS DOCUMENTATION ONLY
///
/// # Formula
/// ```text
/// nullifier_hash = Poseidon(nullifier_preimage, secret)
/// ```
///
/// The nullifier is revealed on-chain during withdrawal.
/// It prevents double-spending without revealing the commitment.
#[allow(dead_code)]
pub fn compute_nullifier_offchain(
    _nullifier_preimage: &[u8; 32],
    _secret: &[u8; 32],
) -> [u8; 32] {
    panic!("Nullifiers must be computed off-chain using Poseidon")
}

// ============================================================================
// LEGACY FUNCTIONS (Kept for backward compatibility, use with caution)
// ============================================================================

/// Legacy: Compute commitment using Keccak256.
///
/// # ⚠️ WARNING
/// This uses Keccak256, NOT Poseidon.
/// It will NOT match ZK circuit expectations.
/// Only use for testing or migration.
#[deprecated(note = "Use off-chain Poseidon for production")]
#[allow(dead_code)]
pub fn hash_commitment_legacy(
    secret: &[u8; 32],
    nullifier_preimage: &[u8; 32],
    amount: u64,
) -> [u8; 32] {
    let mut data = Vec::with_capacity(72);
    data.extend_from_slice(secret);
    data.extend_from_slice(nullifier_preimage);
    data.extend_from_slice(&amount.to_le_bytes());
    keccak::hash(&data).to_bytes()
}

/// Legacy: Compute nullifier using Keccak256.
///
/// # ⚠️ WARNING
/// This uses Keccak256, NOT Poseidon.
#[deprecated(note = "Use off-chain Poseidon for production")]
#[allow(dead_code)]
pub fn hash_nullifier_legacy(nullifier_preimage: &[u8; 32], secret: &[u8; 32]) -> [u8; 32] {
    let mut data = [0u8; 64];
    data[..32].copy_from_slice(nullifier_preimage);
    data[32..].copy_from_slice(secret);
    keccak::hash(&data).to_bytes()
}

// ============================================================================
// UTILITY FUNCTIONS
// ============================================================================

/// Check if a 32-byte value is all zeros.
#[inline]
pub fn is_zero_hash(hash: &[u8; 32]) -> bool {
    hash.iter().all(|&b| b == 0)
}

/// Convert u64 to 32-byte big-endian representation.
///
/// Places the 8-byte big-endian value in the last 8 bytes.
/// Used for encoding amounts as field elements.
#[inline]
pub fn u64_to_bytes32_be(value: u64) -> [u8; 32] {
    let mut bytes = [0u8; 32];
    bytes[24..32].copy_from_slice(&value.to_be_bytes());
    bytes
}

/// Convert u64 to 32-byte little-endian representation.
///
/// Places the 8-byte little-endian value in the first 8 bytes.
#[inline]
pub fn u64_to_bytes32(value: u64) -> [u8; 32] {
    let mut bytes = [0u8; 32];
    bytes[..8].copy_from_slice(&value.to_le_bytes());
    bytes
}

/// Empty leaf hash (all zeros).
///
/// Used as the canonical empty leaf value in the Merkle tree.
#[inline]
pub fn empty_leaf_hash() -> [u8; 32] {
    [0u8; 32]
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_two_to_one_deterministic() {
        let left = [1u8; 32];
        let right = [2u8; 32];
        let h1 = hash_two_to_one(&left, &right);
        let h2 = hash_two_to_one(&left, &right);
        assert_eq!(h1, h2, "Hash should be deterministic");
    }

    #[test]
    fn test_hash_two_to_one_non_commutative() {
        let a = [1u8; 32];
        let b = [2u8; 32];
        let h1 = hash_two_to_one(&a, &b);
        let h2 = hash_two_to_one(&b, &a);
        assert_ne!(h1, h2, "Hash should not be commutative");
    }

    #[test]
    fn test_hash_two_to_one_different_inputs() {
        let a = [1u8; 32];
        let b = [2u8; 32];
        let c = [3u8; 32];
        let h1 = hash_two_to_one(&a, &b);
        let h2 = hash_two_to_one(&a, &c);
        assert_ne!(h1, h2, "Different inputs should produce different hashes");
    }

    #[test]
    fn test_is_zero_hash() {
        let zero = [0u8; 32];
        assert!(is_zero_hash(&zero));

        let non_zero = [1u8; 32];
        assert!(!is_zero_hash(&non_zero));

        let partial = {
            let mut arr = [0u8; 32];
            arr[31] = 1;
            arr
        };
        assert!(!is_zero_hash(&partial));
    }

    #[test]
    fn test_u64_to_bytes32_be() {
        let value = 0x0102030405060708u64;
        let bytes = u64_to_bytes32_be(value);
        
        // First 24 bytes should be zero
        assert!(bytes[..24].iter().all(|&b| b == 0));
        
        // Last 8 bytes should be big-endian
        assert_eq!(bytes[24], 0x01);
        assert_eq!(bytes[25], 0x02);
        assert_eq!(bytes[31], 0x08);
    }

    #[test]
    fn test_u64_to_bytes32_le() {
        let value = 0x0102030405060708u64;
        let bytes = u64_to_bytes32(value);
        
        // First 8 bytes should be little-endian
        assert_eq!(bytes[0], 0x08);
        assert_eq!(bytes[7], 0x01);
        
        // Last 24 bytes should be zero
        assert!(bytes[8..].iter().all(|&b| b == 0));
    }

    #[test]
    fn test_empty_leaf_hash() {
        let empty = empty_leaf_hash();
        assert!(is_zero_hash(&empty));
    }
}
