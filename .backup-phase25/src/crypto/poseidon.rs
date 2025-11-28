//! Poseidon Hash Function for pSol Privacy Pool
//!
//! # IMPORTANT: Development Placeholder
//! 
//! This implementation uses Keccak256 as a stand-in for Poseidon due to
//! Solana BPF stack size limitations with the light-poseidon crate.
//!
//! For production with real ZK circuits:
//! - Poseidon hashing must be done off-chain
//! - On-chain verification uses the Groth16 proof which embeds the hash
//! - Or use a Poseidon implementation optimized for Solana's constraints

use solana_program::keccak;

/// Hash two 32-byte values into one (for Merkle tree nodes).
pub fn hash_two_to_one(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let mut combined = [0u8; 64];
    combined[..32].copy_from_slice(left);
    combined[32..].copy_from_slice(right);
    keccak::hash(&combined).to_bytes()
}

/// Compute commitment from secret, nullifier preimage, and amount.
/// commitment = Hash(secret, nullifier_preimage, amount)
pub fn hash_commitment(
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

/// Compute nullifier hash from nullifier preimage and secret.
/// nullifier_hash = Hash(nullifier_preimage, secret)
pub fn hash_nullifier(nullifier_preimage: &[u8; 32], secret: &[u8; 32]) -> [u8; 32] {
    let mut data = [0u8; 64];
    data[..32].copy_from_slice(nullifier_preimage);
    data[32..].copy_from_slice(secret);
    keccak::hash(&data).to_bytes()
}

/// Check if a 32-byte value is all zeros.
pub fn is_zero_hash(hash: &[u8; 32]) -> bool {
    hash.iter().all(|&b| b == 0)
}

/// Convert u64 to 32-byte big-endian representation.
pub fn u64_to_bytes32_be(value: u64) -> [u8; 32] {
    let mut bytes = [0u8; 32];
    bytes[24..32].copy_from_slice(&value.to_be_bytes());
    bytes
}

/// Convert u64 to 32-byte little-endian representation.
pub fn u64_to_bytes32(value: u64) -> [u8; 32] {
    let mut bytes = [0u8; 32];
    bytes[..8].copy_from_slice(&value.to_le_bytes());
    bytes
}

/// Empty leaf hash (zero).
pub fn empty_leaf_hash() -> [u8; 32] {
    [0u8; 32]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_two_to_one_deterministic() {
        let left = [1u8; 32];
        let right = [2u8; 32];
        let h1 = hash_two_to_one(&left, &right);
        let h2 = hash_two_to_one(&left, &right);
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_hash_commitment_deterministic() {
        let secret = [1u8; 32];
        let nullifier = [2u8; 32];
        let c1 = hash_commitment(&secret, &nullifier, 1000);
        let c2 = hash_commitment(&secret, &nullifier, 1000);
        assert_eq!(c1, c2);
    }
}
