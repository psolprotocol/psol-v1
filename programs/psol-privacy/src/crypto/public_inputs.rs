//! Public Inputs for Zero-Knowledge Proofs
//!
//! # Phase 3 Implementation
//!
//! Defines the canonical structure for public inputs passed to
//! the withdrawal circuit verifier.
//!
//! ## Circuit Public Inputs
//! The withdrawal circuit takes these public inputs:
//! 1. merkle_root - Root of commitment tree (32 bytes)
//! 2. nullifier_hash - Hash of nullifier (prevents double-spend)
//! 3. recipient - Address receiving withdrawn tokens
//! 4. amount - Amount being withdrawn
//! 5. relayer - Relayer address (receives fee)
//! 6. relayer_fee - Fee paid to relayer
//!
//! ## Encoding
//! Each input is encoded as a 32-byte field element (BN254 scalar field).
//! All encodings are big-endian to match circomlib conventions.

use anchor_lang::prelude::*;

use solana_program::keccak;

/// Public inputs for the withdrawal ZK circuit.
///
/// These values are revealed on-chain and must match what was
/// committed to inside the ZK proof.
///
/// ## Invariants
/// - `merkle_root` must be in the pool's recent root history
/// - `nullifier_hash` must not have been used before
/// - `relayer_fee <= amount`
/// - `recipient` and `relayer` must have valid token accounts
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq)]
pub struct ZkPublicInputs {
    /// Merkle root the proof was generated against.
    /// Must be in the pool's recent root history.
    pub merkle_root: [u8; 32],

    /// Hash of the nullifier.
    /// Computed as: Poseidon(nullifier_preimage, secret)
    /// Prevents double-spending the same commitment.
    pub nullifier_hash: [u8; 32],

    /// Recipient address for withdrawn tokens.
    /// The recipient's token account must be owned by this address.
    pub recipient: Pubkey,

    /// Amount to withdraw (before fee deduction).
    pub amount: u64,

    /// Relayer address (receives relayer_fee).
    /// Can be same as recipient if user is self-relaying.
    pub relayer: Pubkey,

    /// Fee paid to relayer from the withdrawal amount.
    /// recipient receives (amount - relayer_fee).
    pub relayer_fee: u64,
}

impl ZkPublicInputs {
    /// Number of public inputs for the circuit.
    /// Used to validate VK IC length (should be COUNT + 1).
    pub const COUNT: usize = 6;

    /// Create new public inputs structure.
    pub fn new(
        merkle_root: [u8; 32],
        nullifier_hash: [u8; 32],
        recipient: Pubkey,
        amount: u64,
        relayer: Pubkey,
        relayer_fee: u64,
    ) -> Self {
        ZkPublicInputs {
            merkle_root,
            nullifier_hash,
            recipient,
            amount,
            relayer,
            relayer_fee,
        }
    }

    /// Convert public inputs to field elements for circuit verification.
    ///
    /// Each input is encoded as a 32-byte value representing a
    /// BN254 scalar field element in big-endian format.
    ///
    /// # Encoding Rules
    /// - 32-byte values: Used directly (assumed already in correct format)
    /// - Pubkeys: Converted to 32-byte representation
    /// - u64: Padded to 32 bytes (big-endian)
    ///
    /// # Returns
    /// Vector of 6 field element encodings in circuit order
    pub fn to_field_elements(&self) -> Vec<[u8; 32]> {
        vec![
            self.merkle_root,
            self.nullifier_hash,
            self.pubkey_to_field(&self.recipient),
            self.u64_to_field(self.amount),
            self.pubkey_to_field(&self.relayer),
            self.u64_to_field(self.relayer_fee),
        ]
    }

    /// Compute a hash of all public inputs.
    /// Useful for logging and debugging.
    pub fn hash(&self) -> [u8; 32] {
        let mut data = Vec::with_capacity(32 + 32 + 32 + 8 + 32 + 8);
        data.extend_from_slice(&self.merkle_root);
        data.extend_from_slice(&self.nullifier_hash);
        data.extend_from_slice(&self.recipient.to_bytes());
        data.extend_from_slice(&self.amount.to_le_bytes());
        data.extend_from_slice(&self.relayer.to_bytes());
        data.extend_from_slice(&self.relayer_fee.to_le_bytes());
        keccak::hash(&data).to_bytes()
    }

    /// Convert Pubkey to 32-byte field element.
    fn pubkey_to_field(&self, pubkey: &Pubkey) -> [u8; 32] {
        pubkey.to_bytes()
    }

    /// Convert u64 to 32-byte field element (big-endian, zero-padded).
    fn u64_to_field(&self, value: u64) -> [u8; 32] {
        let mut bytes = [0u8; 32];
        bytes[24..32].copy_from_slice(&value.to_be_bytes());
        bytes
    }

    /// Validate public inputs for sanity and security.
    ///
    /// # Checks
    /// - Merkle root is not all zeros
    /// - Nullifier hash is not all zeros
    /// - Amount is positive
    /// - Relayer fee does not exceed amount
    pub fn validate(&self) -> Result<()> {
        use crate::error::PrivacyError;

        // Merkle root cannot be zero
        require!(
            !self.merkle_root.iter().all(|&b| b == 0),
            PrivacyError::InvalidMerkleRoot
        );

        // Nullifier cannot be zero
        require!(
            !self.nullifier_hash.iter().all(|&b| b == 0),
            PrivacyError::InvalidNullifier
        );

        // Amount must be positive
        require!(self.amount > 0, PrivacyError::InvalidAmount);

        // Fee cannot exceed amount
        require!(
            self.relayer_fee <= self.amount,
            PrivacyError::RelayerFeeExceedsAmount
        );

        Ok(())
    }

    /// Get net amount after relayer fee.
    pub fn net_amount(&self) -> u64 {
        self.amount.saturating_sub(self.relayer_fee)
    }

    /// Check if this is a self-relay (no external relayer).
    pub fn is_self_relay(&self) -> bool {
        self.recipient == self.relayer && self.relayer_fee == 0
    }
}

/// Builder for ZkPublicInputs (convenience for tests and clients).
#[derive(Default)]
pub struct ZkPublicInputsBuilder {
    merkle_root: Option<[u8; 32]>,
    nullifier_hash: Option<[u8; 32]>,
    recipient: Option<Pubkey>,
    amount: Option<u64>,
    relayer: Option<Pubkey>,
    relayer_fee: Option<u64>,
}

impl ZkPublicInputsBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn merkle_root(mut self, root: [u8; 32]) -> Self {
        self.merkle_root = Some(root);
        self
    }

    pub fn nullifier_hash(mut self, hash: [u8; 32]) -> Self {
        self.nullifier_hash = Some(hash);
        self
    }

    pub fn recipient(mut self, recipient: Pubkey) -> Self {
        self.recipient = Some(recipient);
        self
    }

    pub fn amount(mut self, amount: u64) -> Self {
        self.amount = Some(amount);
        self
    }

    pub fn relayer(mut self, relayer: Pubkey) -> Self {
        self.relayer = Some(relayer);
        self
    }

    pub fn relayer_fee(mut self, fee: u64) -> Self {
        self.relayer_fee = Some(fee);
        self
    }

    /// Build the public inputs. Returns None if any required field is missing.
    pub fn build(self) -> Option<ZkPublicInputs> {
        Some(ZkPublicInputs {
            merkle_root: self.merkle_root?,
            nullifier_hash: self.nullifier_hash?,
            recipient: self.recipient?,
            amount: self.amount?,
            relayer: self.relayer?,
            relayer_fee: self.relayer_fee.unwrap_or(0),
        })
    }

    /// Build for self-relay (sets relayer = recipient with zero fee).
    pub fn build_self_relay(self) -> Option<ZkPublicInputs> {
        let recipient = self.recipient?;
        Some(ZkPublicInputs {
            merkle_root: self.merkle_root?,
            nullifier_hash: self.nullifier_hash?,
            recipient,
            amount: self.amount?,
            relayer: recipient,
            relayer_fee: 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_elements_count() {
        let inputs = ZkPublicInputs::new(
            [1u8; 32],
            [2u8; 32],
            Pubkey::default(),
            1000,
            Pubkey::default(),
            10,
        );
        
        let fields = inputs.to_field_elements();
        assert_eq!(fields.len(), ZkPublicInputs::COUNT);
    }

    #[test]
    fn test_net_amount() {
        let inputs = ZkPublicInputs::new(
            [1u8; 32],
            [2u8; 32],
            Pubkey::default(),
            1000,
            Pubkey::default(),
            50,
        );
        
        assert_eq!(inputs.net_amount(), 950);
    }

    #[test]
    fn test_net_amount_overflow_protection() {
        let inputs = ZkPublicInputs::new(
            [1u8; 32],
            [2u8; 32],
            Pubkey::default(),
            100,
            Pubkey::default(),
            200, // Fee > amount (invalid, but tests saturation)
        );
        
        assert_eq!(inputs.net_amount(), 0);
    }

    #[test]
    fn test_builder() {
        let inputs = ZkPublicInputsBuilder::new()
            .merkle_root([1u8; 32])
            .nullifier_hash([2u8; 32])
            .recipient(Pubkey::default())
            .amount(1000)
            .relayer(Pubkey::default())
            .relayer_fee(10)
            .build()
            .unwrap();
        
        assert_eq!(inputs.amount, 1000);
        assert_eq!(inputs.relayer_fee, 10);
    }

    #[test]
    fn test_builder_self_relay() {
        let recipient = Pubkey::new_unique();
        let inputs = ZkPublicInputsBuilder::new()
            .merkle_root([1u8; 32])
            .nullifier_hash([2u8; 32])
            .recipient(recipient)
            .amount(1000)
            .build_self_relay()
            .unwrap();
        
        assert_eq!(inputs.recipient, recipient);
        assert_eq!(inputs.relayer, recipient);
        assert_eq!(inputs.relayer_fee, 0);
        assert!(inputs.is_self_relay());
    }

    #[test]
    fn test_u64_to_field_big_endian() {
        let inputs = ZkPublicInputs::new(
            [0u8; 32],
            [0u8; 32],
            Pubkey::default(),
            0x0102030405060708,
            Pubkey::default(),
            0,
        );
        
        let fields = inputs.to_field_elements();
        let amount_field = &fields[3];
        
        // First 24 bytes should be zero
        assert!(amount_field[..24].iter().all(|&b| b == 0));
        // Last 8 bytes should be big-endian representation
        assert_eq!(
            &amount_field[24..],
            &[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]
        );
    }

    #[test]
    fn test_validate_success() {
        let inputs = ZkPublicInputs::new(
            [1u8; 32],
            [2u8; 32],
            Pubkey::default(),
            1000,
            Pubkey::default(),
            10,
        );
        
        assert!(inputs.validate().is_ok());
    }

    #[test]
    fn test_validate_zero_merkle_root() {
        let inputs = ZkPublicInputs::new(
            [0u8; 32], // Invalid
            [2u8; 32],
            Pubkey::default(),
            1000,
            Pubkey::default(),
            10,
        );
        
        assert!(inputs.validate().is_err());
    }

    #[test]
    fn test_validate_zero_amount() {
        let inputs = ZkPublicInputs::new(
            [1u8; 32],
            [2u8; 32],
            Pubkey::default(),
            0, // Invalid
            Pubkey::default(),
            0,
        );
        
        assert!(inputs.validate().is_err());
    }

    #[test]
    fn test_validate_fee_exceeds_amount() {
        let inputs = ZkPublicInputs::new(
            [1u8; 32],
            [2u8; 32],
            Pubkey::default(),
            100,
            Pubkey::default(),
            200, // Invalid: fee > amount
        );
        
        assert!(inputs.validate().is_err());
    }
}
