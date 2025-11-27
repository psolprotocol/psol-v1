//! Comprehensive Test Suite for pSol Privacy Pool - Phase 3
//!
//! This module contains unit tests, property tests, and integration test
//! helpers for the privacy pool implementation.
//!
//! # Test Categories
//!
//! 1. **Poseidon Hash Tests**: Verify hash function correctness
//! 2. **Merkle Tree Tests**: Verify insertion, root computation, history
//! 3. **Cryptographic Tests**: Verify curve operations and proof handling
//! 4. **State Invariant Tests**: Verify protocol invariants hold
//! 5. **Property Tests**: Randomized testing of invariants

#[cfg(test)]
mod poseidon_tests {
    use crate::crypto::poseidon::*;

    #[test]
    fn test_commitment_uniqueness() {
        // Same inputs should produce same commitment
        let secret = [0x42u8; 32];
        let nullifier = [0x43u8; 32];
        let amount = 1000u64;

        let c1 = hash_commitment(&secret, &nullifier, amount);
        let c2 = hash_commitment(&secret, &nullifier, amount);
        assert_eq!(c1, c2, "Same inputs should produce same commitment");
    }

    #[test]
    fn test_commitment_different_secrets() {
        let secret1 = [0x01u8; 32];
        let secret2 = [0x02u8; 32];
        let nullifier = [0x03u8; 32];
        let amount = 1000u64;

        let c1 = hash_commitment(&secret1, &nullifier, amount);
        let c2 = hash_commitment(&secret2, &nullifier, amount);
        assert_ne!(c1, c2, "Different secrets must produce different commitments");
    }

    #[test]
    fn test_commitment_different_nullifiers() {
        let secret = [0x01u8; 32];
        let nullifier1 = [0x02u8; 32];
        let nullifier2 = [0x03u8; 32];
        let amount = 1000u64;

        let c1 = hash_commitment(&secret, &nullifier1, amount);
        let c2 = hash_commitment(&secret, &nullifier2, amount);
        assert_ne!(c1, c2, "Different nullifiers must produce different commitments");
    }

    #[test]
    fn test_commitment_different_amounts() {
        let secret = [0x01u8; 32];
        let nullifier = [0x02u8; 32];

        let c1 = hash_commitment(&secret, &nullifier, 100);
        let c2 = hash_commitment(&secret, &nullifier, 200);
        assert_ne!(c1, c2, "Different amounts must produce different commitments");
    }

    #[test]
    fn test_nullifier_hash_uniqueness() {
        let preimage = [0x01u8; 32];
        let secret = [0x02u8; 32];

        let n1 = hash_nullifier(&preimage, &secret);
        let n2 = hash_nullifier(&preimage, &secret);
        assert_eq!(n1, n2, "Same inputs should produce same nullifier hash");
    }

    #[test]
    fn test_nullifier_commitment_independence() {
        // Nullifier and commitment should be different even with same underlying values
        let secret = [0x01u8; 32];
        let nullifier_preimage = [0x02u8; 32];
        let amount = 1000u64;

        let commitment = hash_commitment(&secret, &nullifier_preimage, amount);
        let nullifier = hash_nullifier(&nullifier_preimage, &secret);

        assert_ne!(commitment, nullifier, "Commitment and nullifier must differ");
    }

    #[test]
    fn test_merkle_hash_non_commutative() {
        let left = [0x01u8; 32];
        let right = [0x02u8; 32];

        let h1 = hash_two_to_one(&left, &right);
        let h2 = hash_two_to_one(&right, &left);
        assert_ne!(h1, h2, "Merkle hash must not be commutative");
    }

    #[test]
    fn test_merkle_hash_deterministic() {
        let left = [0xAAu8; 32];
        let right = [0xBBu8; 32];

        let h1 = hash_two_to_one(&left, &right);
        let h2 = hash_two_to_one(&left, &right);
        assert_eq!(h1, h2, "Merkle hash must be deterministic");
    }

    #[test]
    fn test_empty_leaf_is_zero() {
        let empty = empty_leaf_hash();
        assert!(is_zero_hash(&empty), "Empty leaf should be zero hash");
    }
}

#[cfg(test)]
mod merkle_tree_tests {
    use crate::crypto::poseidon::hash_two_to_one;

    /// Compute zero values for a merkle tree of given depth
    fn compute_zeros(depth: u8) -> Vec<[u8; 32]> {
        let mut zeros = Vec::with_capacity((depth + 1) as usize);
        zeros.push([0u8; 32]); // Level 0: zero leaf
        
        for i in 1..=depth {
            let prev = &zeros[(i - 1) as usize];
            zeros.push(hash_two_to_one(prev, prev));
        }
        zeros
    }

    /// Compute merkle root from leaves using filled_subtrees pattern
    fn compute_root(leaves: &[[u8; 32]], depth: u8) -> [u8; 32] {
        let zeros = compute_zeros(depth);
        let mut filled_subtrees: Vec<[u8; 32]> = zeros[..depth as usize].to_vec();
        let mut current_root = zeros[depth as usize];

        for (leaf_index, &leaf) in leaves.iter().enumerate() {
            let mut current_hash = leaf;
            let mut current_index = leaf_index as u32;

            for level in 0..depth {
                let is_right_child = (current_index & 1) == 1;
                current_index >>= 1;

                if is_right_child {
                    current_hash = hash_two_to_one(&filled_subtrees[level as usize], &current_hash);
                } else {
                    filled_subtrees[level as usize] = current_hash;
                    current_hash = hash_two_to_one(&current_hash, &zeros[level as usize]);
                }
            }
            current_root = current_hash;
        }

        current_root
    }

    #[test]
    fn test_empty_tree_root() {
        let zeros = compute_zeros(4);
        let empty_root = zeros[4]; // Root of depth-4 empty tree
        
        // Empty tree should have deterministic root
        let zeros2 = compute_zeros(4);
        assert_eq!(empty_root, zeros2[4], "Empty tree root should be deterministic");
    }

    #[test]
    fn test_single_leaf_insertion() {
        let leaf = [0x42u8; 32];
        let root1 = compute_root(&[leaf], 4);
        let root2 = compute_root(&[leaf], 4);
        assert_eq!(root1, root2, "Single leaf insertion should be deterministic");
    }

    #[test]
    fn test_different_leaves_different_roots() {
        let leaf1 = [0x01u8; 32];
        let leaf2 = [0x02u8; 32];
        
        let root1 = compute_root(&[leaf1], 4);
        let root2 = compute_root(&[leaf2], 4);
        assert_ne!(root1, root2, "Different leaves should produce different roots");
    }

    #[test]
    fn test_leaf_order_matters() {
        let leaf1 = [0x01u8; 32];
        let leaf2 = [0x02u8; 32];
        
        let root_12 = compute_root(&[leaf1, leaf2], 4);
        let root_21 = compute_root(&[leaf2, leaf1], 4);
        assert_ne!(root_12, root_21, "Leaf order should matter");
    }

    #[test]
    fn test_incremental_insertions() {
        let leaves: Vec<[u8; 32]> = (0..4).map(|i| [i as u8; 32]).collect();
        
        // Insert one at a time and verify root changes
        let mut prev_root = compute_root(&[], 4);
        
        for i in 1..=leaves.len() {
            let new_root = compute_root(&leaves[..i], 4);
            assert_ne!(new_root, prev_root, "Root should change with each insertion");
            prev_root = new_root;
        }
    }
}

#[cfg(test)]
mod curve_utils_tests {
    use crate::crypto::curve_utils::*;

    #[test]
    fn test_g1_identity_check() {
        let identity = G1_IDENTITY;
        assert!(is_g1_identity(&identity), "Zero point should be identity");
        
        let non_identity = [1u8; 64];
        assert!(!is_g1_identity(&non_identity), "Non-zero point is not identity");
    }

    #[test]
    fn test_g2_identity_check() {
        let identity = G2_IDENTITY;
        assert!(is_g2_identity(&identity), "Zero point should be identity");
        
        let non_identity = [1u8; 128];
        assert!(!is_g2_identity(&non_identity), "Non-zero point is not identity");
    }

    #[test]
    fn test_negate_identity_is_identity() {
        let identity = G1_IDENTITY;
        let negated = negate_g1(&identity).unwrap();
        assert_eq!(identity, negated, "-O should equal O");
    }

    #[test]
    fn test_u64_to_scalar_big_endian() {
        let value = 0x0102030405060708u64;
        let scalar = u64_to_scalar(value);
        
        // First 24 bytes should be zero
        assert!(scalar[..24].iter().all(|&b| b == 0));
        // Last 8 bytes should be big-endian
        assert_eq!(&scalar[24..], &[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]);
    }

    #[test]
    fn test_scalar_validity() {
        // Zero is valid
        assert!(is_valid_scalar(&[0u8; 32]));
        
        // Small value is valid
        let small = u64_to_scalar(12345);
        assert!(is_valid_scalar(&small));
    }

    #[test]
    fn test_pairing_element_construction() {
        let g1 = [0x01u8; 64];
        let g2 = [0x02u8; 128];
        
        let elem = make_pairing_element(&g1, &g2);
        
        assert_eq!(&elem[0..64], &g1);
        assert_eq!(&elem[64..192], &g2);
    }
}

#[cfg(test)]
mod proof_tests {
    use crate::crypto::groth16_verifier::{Groth16Proof, PROOF_DATA_LEN};

    #[test]
    fn test_proof_parsing() {
        let mut data = [0u8; PROOF_DATA_LEN];
        // Set distinct patterns for A, B, C
        data[0..64].fill(0x01);
        data[64..192].fill(0x02);
        data[192..256].fill(0x03);

        let proof = Groth16Proof::from_bytes(&data).unwrap();
        
        assert!(proof.a.iter().all(|&b| b == 0x01));
        assert!(proof.b.iter().all(|&b| b == 0x02));
        assert!(proof.c.iter().all(|&b| b == 0x03));
    }

    #[test]
    fn test_proof_roundtrip() {
        let original = [0x42u8; PROOF_DATA_LEN];
        let proof = Groth16Proof::from_bytes(&original).unwrap();
        let serialized = proof.to_bytes();
        assert_eq!(original, serialized);
    }

    #[test]
    fn test_proof_invalid_length_short() {
        let data = [0u8; 100];
        assert!(Groth16Proof::from_bytes(&data).is_err());
    }

    #[test]
    fn test_proof_invalid_length_long() {
        let data = [0u8; 300];
        assert!(Groth16Proof::from_bytes(&data).is_err());
    }
}

#[cfg(test)]
mod public_inputs_tests {
    use crate::crypto::public_inputs::{ZkPublicInputs, ZkPublicInputsBuilder};
    use anchor_lang::prelude::Pubkey;

    fn make_valid_inputs() -> ZkPublicInputs {
        ZkPublicInputs::new(
            [0x01u8; 32],  // merkle_root
            [0x02u8; 32],  // nullifier_hash
            Pubkey::new_unique(),  // recipient
            1000,          // amount
            Pubkey::new_unique(),  // relayer
            50,            // relayer_fee
        )
    }

    #[test]
    fn test_valid_inputs() {
        let inputs = make_valid_inputs();
        assert!(inputs.validate().is_ok());
    }

    #[test]
    fn test_zero_merkle_root_invalid() {
        let mut inputs = make_valid_inputs();
        inputs.merkle_root = [0u8; 32];
        assert!(inputs.validate().is_err());
    }

    #[test]
    fn test_zero_nullifier_invalid() {
        let mut inputs = make_valid_inputs();
        inputs.nullifier_hash = [0u8; 32];
        assert!(inputs.validate().is_err());
    }

    #[test]
    fn test_zero_amount_invalid() {
        let mut inputs = make_valid_inputs();
        inputs.amount = 0;
        assert!(inputs.validate().is_err());
    }

    #[test]
    fn test_fee_exceeds_amount_invalid() {
        let mut inputs = make_valid_inputs();
        inputs.relayer_fee = inputs.amount + 1;
        assert!(inputs.validate().is_err());
    }

    #[test]
    fn test_fee_equals_amount_valid() {
        let mut inputs = make_valid_inputs();
        inputs.relayer_fee = inputs.amount;
        assert!(inputs.validate().is_ok());
        assert_eq!(inputs.net_amount(), 0);
    }

    #[test]
    fn test_net_amount_calculation() {
        let inputs = ZkPublicInputs::new(
            [0x01u8; 32],
            [0x02u8; 32],
            Pubkey::new_unique(),
            1000,
            Pubkey::new_unique(),
            100,
        );
        assert_eq!(inputs.net_amount(), 900);
    }

    #[test]
    fn test_field_elements_count() {
        let inputs = make_valid_inputs();
        let fields = inputs.to_field_elements();
        assert_eq!(fields.len(), ZkPublicInputs::COUNT);
    }

    #[test]
    fn test_builder_complete() {
        let inputs = ZkPublicInputsBuilder::new()
            .merkle_root([0x01u8; 32])
            .nullifier_hash([0x02u8; 32])
            .recipient(Pubkey::new_unique())
            .amount(1000)
            .relayer(Pubkey::new_unique())
            .relayer_fee(50)
            .build();
        assert!(inputs.is_some());
    }

    #[test]
    fn test_builder_missing_fields() {
        let inputs = ZkPublicInputsBuilder::new()
            .merkle_root([0x01u8; 32])
            .amount(1000)
            .build();
        assert!(inputs.is_none());
    }

    #[test]
    fn test_self_relay_detection() {
        let recipient = Pubkey::new_unique();
        let inputs = ZkPublicInputs::new(
            [0x01u8; 32],
            [0x02u8; 32],
            recipient,
            1000,
            recipient,  // same as recipient
            0,          // zero fee
        );
        assert!(inputs.is_self_relay());
    }

    #[test]
    fn test_not_self_relay_different_address() {
        let inputs = ZkPublicInputs::new(
            [0x01u8; 32],
            [0x02u8; 32],
            Pubkey::new_unique(),
            1000,
            Pubkey::new_unique(),
            0,
        );
        assert!(!inputs.is_self_relay());
    }

    #[test]
    fn test_not_self_relay_with_fee() {
        let recipient = Pubkey::new_unique();
        let inputs = ZkPublicInputs::new(
            [0x01u8; 32],
            [0x02u8; 32],
            recipient,
            1000,
            recipient,
            50,  // non-zero fee
        );
        assert!(!inputs.is_self_relay());
    }
}

#[cfg(test)]
mod protocol_invariant_tests {
    use crate::crypto::poseidon::{hash_commitment, hash_nullifier};

    /// Test that the commitment/nullifier scheme provides key invariants
    #[test]
    fn test_commitment_binding() {
        // Two deposits with different amounts must have different commitments
        let secret = [0x42u8; 32];
        let nullifier_preimage = [0x43u8; 32];
        
        let c100 = hash_commitment(&secret, &nullifier_preimage, 100);
        let c200 = hash_commitment(&secret, &nullifier_preimage, 200);
        
        assert_ne!(c100, c200, "Amount binding: different amounts → different commitments");
    }

    #[test]
    fn test_nullifier_uniqueness() {
        // Each commitment has a unique nullifier
        let secret1 = [0x01u8; 32];
        let secret2 = [0x02u8; 32];
        let preimage = [0x03u8; 32];
        
        let n1 = hash_nullifier(&preimage, &secret1);
        let n2 = hash_nullifier(&preimage, &secret2);
        
        assert_ne!(n1, n2, "Different secrets → different nullifiers");
    }

    #[test]
    fn test_commitment_hiding() {
        // Cannot determine commitment from nullifier hash
        // (This is a structural property - nullifier doesn't reveal commitment inputs)
        let secret = [0x01u8; 32];
        let preimage = [0x02u8; 32];
        let amount = 1000u64;
        
        let commitment = hash_commitment(&secret, &preimage, amount);
        let nullifier = hash_nullifier(&preimage, &secret);
        
        // These are computed from same inputs but are different values
        assert_ne!(commitment, nullifier);
        
        // Importantly: knowing nullifier doesn't reveal commitment
        // (This is cryptographically enforced by Poseidon being a one-way function)
    }
}

// ============================================================================
// PROPERTY-BASED TESTS (using proptest)
// ============================================================================

#[cfg(test)]
mod property_tests {
    // These tests use proptest for randomized testing
    // Uncomment when proptest is available in dev-dependencies
    
    /*
    use proptest::prelude::*;
    use crate::crypto::poseidon::*;

    proptest! {
        #[test]
        fn prop_commitment_deterministic(
            secret in prop::array::uniform32(any::<u8>()),
            nullifier in prop::array::uniform32(any::<u8>()),
            amount in any::<u64>(),
        ) {
            let c1 = hash_commitment(&secret, &nullifier, amount);
            let c2 = hash_commitment(&secret, &nullifier, amount);
            prop_assert_eq!(c1, c2);
        }

        #[test]
        fn prop_nullifier_deterministic(
            preimage in prop::array::uniform32(any::<u8>()),
            secret in prop::array::uniform32(any::<u8>()),
        ) {
            let n1 = hash_nullifier(&preimage, &secret);
            let n2 = hash_nullifier(&preimage, &secret);
            prop_assert_eq!(n1, n2);
        }

        #[test]
        fn prop_merkle_hash_deterministic(
            left in prop::array::uniform32(any::<u8>()),
            right in prop::array::uniform32(any::<u8>()),
        ) {
            let h1 = hash_two_to_one(&left, &right);
            let h2 = hash_two_to_one(&left, &right);
            prop_assert_eq!(h1, h2);
        }

        #[test]
        fn prop_hash_output_not_zero(
            secret in prop::array::uniform32(1u8..=255u8),
            nullifier in prop::array::uniform32(1u8..=255u8),
            amount in 1u64..,
        ) {
            let c = hash_commitment(&secret, &nullifier, amount);
            prop_assert!(!is_zero_hash(&c), "Commitment should not be zero");
        }
    }
    */
}
