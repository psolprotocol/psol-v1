//! Poseidon Hash Function for pSol Privacy Pool
//!
//! # Phase 3 Implementation
//!
//! This module provides Poseidon hashing using the `light-poseidon` crate,
//! which is optimized for Solana and compatible with circomlib's Poseidon.
//!
//! ## Hash Functions Provided
//! - `hash_two_to_one`: Merkle tree internal node hashing
//! - `hash_commitment`: commitment = Poseidon(secret, nullifier_preimage, amount)
//! - `hash_nullifier`: nullifier_hash = Poseidon(nullifier_preimage, secret)
//!
//! ## Field Compatibility
//! All operations are over the BN254 scalar field (Fr).
//! Values are represented as 32-byte little-endian arrays.
//!
//! ## Circuit Compatibility
//! These hash functions MUST produce identical outputs to the circomlib
//! Poseidon implementation used in the ZK circuits. The light-poseidon
//! crate uses the same parameters (t=3, RF=8, RP=57).

use light_poseidon::{Poseidon, PoseidonBytesHasher, PoseidonHasher};

// ============================================================================
// CONFIGURATION
// ============================================================================

/// Poseidon configuration for BN254 scalar field.
///
/// Parameters match circomlib's Poseidon:
/// - Width (t): 3 for 2-to-1 hashing
/// - Full rounds (RF): 8
/// - Partial rounds (RP): 57
/// - Security level: 128-bit
pub struct PoseidonConfig {
    /// Width parameter (t) - number of field elements in state
    pub width: u8,
    /// Number of full rounds
    pub full_rounds: u8,
    /// Number of partial rounds  
    pub partial_rounds: u8,
}

impl Default for PoseidonConfig {
    fn default() -> Self {
        PoseidonConfig {
            width: 3,
            full_rounds: 8,
            partial_rounds: 57,
        }
    }
}

// ============================================================================
// HASH FUNCTIONS
// ============================================================================

/// Hash two 32-byte values into one (for Merkle tree nodes).
///
/// # Algorithm
/// ```text
/// parent = Poseidon(left, right)
/// ```
///
/// # Arguments
/// * `left` - Left child hash (32 bytes, BN254 Fr element)
/// * `right` - Right child hash (32 bytes, BN254 Fr element)
///
/// # Returns
/// Parent node hash (32 bytes, BN254 Fr element)
///
/// # Circuit Equivalence
/// This function produces the same output as:
/// ```circom
/// component hash = Poseidon(2);
/// hash.inputs[0] <== left;
/// hash.inputs[1] <== right;
/// parent <== hash.out;
/// ```
///
/// # Panics
/// Should not panic with valid 32-byte inputs. Invalid inputs that don't
/// represent valid field elements will be reduced modulo the field order.
pub fn hash_two_to_one(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    // Create Poseidon hasher for 2 inputs
    let mut hasher = Poseidon::<ark_bn254::Fr>::new_circom(2).expect("Failed to create Poseidon hasher");
    
    // Hash the two inputs
    let result = hasher.hash_bytes_be(&[left, right]).expect("Poseidon hash failed");
    
    // Convert result to bytes (big-endian to match circom)
    let mut output = [0u8; 32];
    output.copy_from_slice(&result);
    output
}

/// Compute commitment from secret, nullifier preimage, and amount.
///
/// # Algorithm
/// ```text
/// commitment = Poseidon(secret, nullifier_preimage, amount)
/// ```
///
/// # Arguments
/// * `secret` - Random secret (32 bytes) - USER MUST KEEP PRIVATE
/// * `nullifier_preimage` - Nullifier preimage (32 bytes) - USER MUST KEEP PRIVATE
/// * `amount` - Token amount (u64, converted to 32-byte field element)
///
/// # Returns
/// Commitment hash (32 bytes)
///
/// # Circuit Equivalence
/// ```circom
/// component commitment = Poseidon(3);
/// commitment.inputs[0] <== secret;
/// commitment.inputs[1] <== nullifier_preimage;
/// commitment.inputs[2] <== amount;
/// commitment_out <== commitment.out;
/// ```
///
/// # Security
/// - `secret` MUST be cryptographically random (32 bytes of entropy)
/// - `nullifier_preimage` MUST be cryptographically random
/// - Both values MUST be kept private by the user
/// - Loss of these values = loss of funds (no recovery possible)
pub fn hash_commitment(
    secret: &[u8; 32],
    nullifier_preimage: &[u8; 32],
    amount: u64,
) -> [u8; 32] {
    // Convert amount to 32-byte big-endian representation
    let amount_bytes = u64_to_bytes32_be(amount);
    
    // Create Poseidon hasher for 3 inputs
    let mut hasher = Poseidon::<ark_bn254::Fr>::new_circom(3).expect("Failed to create Poseidon hasher");
    
    // Hash all three inputs
    let result = hasher.hash_bytes_be(&[secret, nullifier_preimage, &amount_bytes])
        .expect("Poseidon hash failed");
    
    let mut output = [0u8; 32];
    output.copy_from_slice(&result);
    output
}

/// Compute nullifier hash from nullifier preimage and secret.
///
/// # Algorithm
/// ```text
/// nullifier_hash = Poseidon(nullifier_preimage, secret)
/// ```
///
/// The nullifier_hash is revealed on-chain during withdrawal to prevent
/// double-spending. The preimage and secret remain private.
///
/// # Arguments
/// * `nullifier_preimage` - Nullifier preimage (32 bytes)
/// * `secret` - The secret used in commitment (32 bytes)
///
/// # Returns
/// Nullifier hash (32 bytes)
///
/// # Circuit Equivalence
/// ```circom
/// component nullifier = Poseidon(2);
/// nullifier.inputs[0] <== nullifier_preimage;
/// nullifier.inputs[1] <== secret;
/// nullifier_hash <== nullifier.out;
/// ```
///
/// # Privacy Model
/// - On withdrawal, user reveals `nullifier_hash` (derived from private inputs)
/// - Observers cannot link withdrawal to deposit (commitments are hiding)
/// - Double-spend prevented by nullifier uniqueness
pub fn hash_nullifier(nullifier_preimage: &[u8; 32], secret: &[u8; 32]) -> [u8; 32] {
    // Create Poseidon hasher for 2 inputs
    let mut hasher = Poseidon::<ark_bn254::Fr>::new_circom(2).expect("Failed to create Poseidon hasher");
    
    // Hash nullifier_preimage and secret
    let result = hasher.hash_bytes_be(&[nullifier_preimage, secret])
        .expect("Poseidon hash failed");
    
    let mut output = [0u8; 32];
    output.copy_from_slice(&result);
    output
}

// ============================================================================
// UTILITY FUNCTIONS
// ============================================================================

/// Check if a 32-byte value is all zeros.
///
/// Used to detect invalid/uninitialized hashes.
pub fn is_zero_hash(hash: &[u8; 32]) -> bool {
    hash.iter().all(|&b| b == 0)
}

/// Convert u64 to 32-byte big-endian representation for field element.
///
/// The value is placed in the last 8 bytes (big-endian convention).
pub fn u64_to_bytes32_be(value: u64) -> [u8; 32] {
    let mut bytes = [0u8; 32];
    bytes[24..32].copy_from_slice(&value.to_be_bytes());
    bytes
}

/// Convert u64 to 32-byte little-endian representation.
///
/// Used for some serialization contexts.
pub fn u64_to_bytes32(value: u64) -> [u8; 32] {
    let mut bytes = [0u8; 32];
    bytes[..8].copy_from_slice(&value.to_le_bytes());
    bytes
}

/// Compute the empty leaf hash (zero leaf).
///
/// This is the canonical zero value used in the Merkle tree for empty leaves.
/// Must match the circuit's definition of an empty leaf.
pub fn empty_leaf_hash() -> [u8; 32] {
    // Empty leaf is all zeros - this is the standard convention
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
        
        assert_eq!(h1, h2, "Poseidon hash should be deterministic");
    }

    #[test]
    fn test_hash_two_to_one_not_commutative() {
        let left = [1u8; 32];
        let right = [2u8; 32];
        
        let h1 = hash_two_to_one(&left, &right);
        let h2 = hash_two_to_one(&right, &left);
        
        // Poseidon hash is NOT commutative (order matters)
        assert_ne!(h1, h2, "hash(left, right) != hash(right, left)");
    }

    #[test]
    fn test_hash_two_to_one_not_zero() {
        let left = [1u8; 32];
        let right = [2u8; 32];
        
        let h = hash_two_to_one(&left, &right);
        
        assert!(!is_zero_hash(&h), "Hash of non-zero inputs should not be zero");
    }

    #[test]
    fn test_hash_commitment_deterministic() {
        let secret = [1u8; 32];
        let nullifier = [2u8; 32];
        let amount = 1000u64;
        
        let c1 = hash_commitment(&secret, &nullifier, amount);
        let c2 = hash_commitment(&secret, &nullifier, amount);
        
        assert_eq!(c1, c2, "Commitment hash should be deterministic");
    }

    #[test]
    fn test_different_amounts_different_commitments() {
        let secret = [1u8; 32];
        let nullifier = [2u8; 32];
        
        let c1 = hash_commitment(&secret, &nullifier, 100);
        let c2 = hash_commitment(&secret, &nullifier, 200);
        
        assert_ne!(c1, c2, "Different amounts should produce different commitments");
    }

    #[test]
    fn test_different_secrets_different_commitments() {
        let secret1 = [1u8; 32];
        let secret2 = [2u8; 32];
        let nullifier = [3u8; 32];
        let amount = 1000u64;
        
        let c1 = hash_commitment(&secret1, &nullifier, amount);
        let c2 = hash_commitment(&secret2, &nullifier, amount);
        
        assert_ne!(c1, c2, "Different secrets should produce different commitments");
    }

    #[test]
    fn test_nullifier_hash_deterministic() {
        let preimage = [1u8; 32];
        let secret = [2u8; 32];
        
        let n1 = hash_nullifier(&preimage, &secret);
        let n2 = hash_nullifier(&preimage, &secret);
        
        assert_eq!(n1, n2, "Nullifier hash should be deterministic");
    }

    #[test]
    fn test_nullifier_hash_unique() {
        let preimage1 = [1u8; 32];
        let preimage2 = [2u8; 32];
        let secret = [3u8; 32];
        
        let n1 = hash_nullifier(&preimage1, &secret);
        let n2 = hash_nullifier(&preimage2, &secret);
        
        assert_ne!(n1, n2, "Different preimages should produce different nullifiers");
    }

    #[test]
    fn test_commitment_and_nullifier_different() {
        // Ensure commitment and nullifier hashes are different even with same inputs
        let secret = [1u8; 32];
        let nullifier_preimage = [2u8; 32];
        let amount = 1000u64;
        
        let commitment = hash_commitment(&secret, &nullifier_preimage, amount);
        let nullifier = hash_nullifier(&nullifier_preimage, &secret);
        
        // These should be different (different hash constructions)
        assert_ne!(commitment, nullifier, "Commitment and nullifier should differ");
    }

    #[test]
    fn test_u64_to_bytes32_be() {
        let value = 0x0102030405060708u64;
        let bytes = u64_to_bytes32_be(value);
        
        // First 24 bytes should be zero
        assert!(bytes[..24].iter().all(|&b| b == 0));
        // Last 8 bytes should be big-endian representation
        assert_eq!(&bytes[24..], &[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]);
    }

    #[test]
    fn test_is_zero_hash() {
        let zero = [0u8; 32];
        let non_zero = [1u8; 32];
        
        assert!(is_zero_hash(&zero));
        assert!(!is_zero_hash(&non_zero));
    }

    #[test]
    fn test_empty_leaf_is_zero() {
        let empty = empty_leaf_hash();
        assert!(is_zero_hash(&empty), "Empty leaf should be zero hash");
    }
}
