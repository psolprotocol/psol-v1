//! Tests for pSol Privacy Pool - Phase 3
//!
//! Unit tests for cryptographic and protocol components.

#[cfg(test)]
mod crypto_tests {
    use crate::crypto::*;

    #[test]
    fn test_merkle_hash_deterministic() {
        let left = [1u8; 32];
        let right = [2u8; 32];
        let h1 = hash_two_to_one(&left, &right);
        let h2 = hash_two_to_one(&left, &right);
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_merkle_hash_non_commutative() {
        let a = [1u8; 32];
        let b = [2u8; 32];
        assert_ne!(hash_two_to_one(&a, &b), hash_two_to_one(&b, &a));
    }

    #[test]
    fn test_g1_identity_detection() {
        let zero = [0u8; 64];
        assert!(is_g1_identity(&zero));

        let non_zero = [1u8; 64];
        assert!(!is_g1_identity(&non_zero));
    }

    #[test]
    fn test_g2_identity_detection() {
        let zero = [0u8; 128];
        assert!(is_g2_identity(&zero));
    }

    #[test]
    fn test_proof_data_length() {
        assert_eq!(PROOF_DATA_LEN, 256);
    }
}

#[cfg(test)]
mod public_inputs_tests {
    use anchor_lang::prelude::*;
    use crate::crypto::ZkPublicInputs;

    fn test_pubkey() -> Pubkey {
        Pubkey::new_unique()
    }

    #[test]
    fn test_valid_inputs() {
        let inputs = ZkPublicInputs::new(
            [1u8; 32],
            [2u8; 32],
            test_pubkey(),
            1000,
            test_pubkey(),
            100,
        );
        assert!(inputs.validate().is_ok());
    }

    #[test]
    fn test_zero_amount_invalid() {
        let inputs = ZkPublicInputs::new(
            [1u8; 32],
            [2u8; 32],
            test_pubkey(),
            0,
            test_pubkey(),
            0,
        );
        assert!(inputs.validate().is_err());
    }

    #[test]
    fn test_fee_exceeds_amount() {
        let inputs = ZkPublicInputs::new(
            [1u8; 32],
            [2u8; 32],
            test_pubkey(),
            100,
            test_pubkey(),
            200,
        );
        assert!(inputs.validate().is_err());
    }

    #[test]
    fn test_field_elements_count() {
        let inputs = ZkPublicInputs::new(
            [1u8; 32],
            [2u8; 32],
            test_pubkey(),
            1000,
            test_pubkey(),
            100,
        );
        assert_eq!(inputs.to_field_elements().len(), ZkPublicInputs::COUNT);
    }

    #[test]
    fn test_self_relay_detection() {
        let addr = test_pubkey();
        let inputs = ZkPublicInputs::new(
            [1u8; 32],
            [2u8; 32],
            addr,
            1000,
            addr,
            0,
        );
        assert!(inputs.is_self_relay());
    }
}

#[cfg(test)]
mod state_tests {
    use crate::state::merkle_tree::MerkleTree;

    #[test]
    fn test_merkle_tree_space() {
        let space = MerkleTree::space(20, 100);
        assert!(space < 10_000_000); // < 10MB
    }
}
