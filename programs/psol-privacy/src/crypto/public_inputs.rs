//! Public Inputs for ZK Circuits - Phase 3
//!
//! This module defines the public inputs structure for Groth16 proofs.
//! Public inputs are the values that are visible to the verifier.
//!
//! # Withdrawal Circuit Public Inputs (6 total)
//! 1. merkle_root - Tree root for membership proof
//! 2. nullifier_hash - Prevents double-spending
//! 3. recipient - Address receiving funds
//! 4. amount - Withdrawal amount
//! 5. relayer - Relayer address
//! 6. relayer_fee - Fee paid to relayer
//!
//! # Field Element Encoding
//! All values are encoded as 32-byte big-endian field elements
//! in the BN254 scalar field.

use anchor_lang::prelude::*;

use crate::error::PrivacyError;

// ============================================================================
// PUBLIC INPUTS STRUCTURE
// ============================================================================

/// Public inputs for withdrawal circuit verification.
///
/// These are the values visible to the on-chain verifier.
/// They must match exactly what was used to generate the proof.
#[derive(Clone, Debug)]
pub struct ZkPublicInputs {
    /// Merkle root of the commitment tree
    pub merkle_root: [u8; 32],
    
    /// Nullifier hash (prevents double-spend)
    pub nullifier_hash: [u8; 32],
    
    /// Recipient address (who receives the tokens)
    pub recipient: Pubkey,
    
    /// Withdrawal amount (before fee)
    pub amount: u64,
    
    /// Relayer address (submits tx on behalf of user)
    pub relayer: Pubkey,
    
    /// Fee paid to relayer (deducted from amount)
    pub relayer_fee: u64,
}

impl ZkPublicInputs {
    /// Number of public inputs for verification
    pub const COUNT: usize = 6;

    /// Create new public inputs
    pub fn new(
        merkle_root: [u8; 32],
        nullifier_hash: [u8; 32],
        recipient: Pubkey,
        amount: u64,
        relayer: Pubkey,
        relayer_fee: u64,
    ) -> Self {
        Self {
            merkle_root,
            nullifier_hash,
            recipient,
            amount,
            relayer,
            relayer_fee,
        }
    }

    /// Validate public inputs
    pub fn validate(&self) -> Result<()> {
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

    /// Convert to field elements for Groth16 verification.
    ///
    /// Returns a vector of 32-byte field elements in the order
    /// expected by the circuit.
    pub fn to_field_elements(&self) -> Vec<[u8; 32]> {
        vec![
            self.merkle_root,
            self.nullifier_hash,
            self.recipient.to_bytes(),
            u64_to_field(self.amount),
            self.relayer.to_bytes(),
            u64_to_field(self.relayer_fee),
        ]
    }

    /// Calculate net amount after fee
    pub fn net_amount(&self) -> Result<u64> {
        self.amount
            .checked_sub(self.relayer_fee)
            .ok_or_else(|| error!(PrivacyError::ArithmeticOverflow))
    }

    /// Check if this is a self-relay (recipient == relayer, no fee)
    pub fn is_self_relay(&self) -> bool {
        self.recipient == self.relayer && self.relayer_fee == 0
    }
}

// ============================================================================
// BUILDER PATTERN
// ============================================================================

/// Builder for ZkPublicInputs
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
    /// Create a new builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Set merkle root
    pub fn merkle_root(mut self, root: [u8; 32]) -> Self {
        self.merkle_root = Some(root);
        self
    }

    /// Set nullifier hash
    pub fn nullifier_hash(mut self, hash: [u8; 32]) -> Self {
        self.nullifier_hash = Some(hash);
        self
    }

    /// Set recipient
    pub fn recipient(mut self, recipient: Pubkey) -> Self {
        self.recipient = Some(recipient);
        self
    }

    /// Set amount
    pub fn amount(mut self, amount: u64) -> Self {
        self.amount = Some(amount);
        self
    }

    /// Set relayer
    pub fn relayer(mut self, relayer: Pubkey) -> Self {
        self.relayer = Some(relayer);
        self
    }

    /// Set relayer fee
    pub fn relayer_fee(mut self, fee: u64) -> Self {
        self.relayer_fee = Some(fee);
        self
    }

    /// Build for self-relay (recipient = relayer, no fee)
    pub fn build_self_relay(mut self) -> Result<ZkPublicInputs> {
        let recipient = self.recipient.ok_or(error!(PrivacyError::InvalidAmount))?;
        self.relayer = Some(recipient);
        self.relayer_fee = Some(0);
        self.build()
    }

    /// Build the public inputs
    pub fn build(self) -> Result<ZkPublicInputs> {
        let inputs = ZkPublicInputs {
            merkle_root: self.merkle_root.ok_or(error!(PrivacyError::InvalidMerkleRoot))?,
            nullifier_hash: self.nullifier_hash.ok_or(error!(PrivacyError::InvalidNullifier))?,
            recipient: self.recipient.ok_or(error!(PrivacyError::RecipientMismatch))?,
            amount: self.amount.ok_or(error!(PrivacyError::InvalidAmount))?,
            relayer: self.relayer.ok_or(error!(PrivacyError::RecipientMismatch))?,
            relayer_fee: self.relayer_fee.unwrap_or(0),
        };

        inputs.validate()?;
        Ok(inputs)
    }
}

// ============================================================================
// UTILITY FUNCTIONS
// ============================================================================

/// Convert u64 to 32-byte field element (big-endian).
///
/// The value is placed in the last 8 bytes of a 32-byte array.
fn u64_to_field(value: u64) -> [u8; 32] {
    let mut bytes = [0u8; 32];
    bytes[24..32].copy_from_slice(&value.to_be_bytes());
    bytes
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_zero_merkle_root_invalid() {
        let inputs = ZkPublicInputs::new(
            [0u8; 32], // Zero root
            [2u8; 32],
            test_pubkey(),
            1000,
            test_pubkey(),
            100,
        );
        assert!(inputs.validate().is_err());
    }

    #[test]
    fn test_zero_nullifier_invalid() {
        let inputs = ZkPublicInputs::new(
            [1u8; 32],
            [0u8; 32], // Zero nullifier
            test_pubkey(),
            1000,
            test_pubkey(),
            100,
        );
        assert!(inputs.validate().is_err());
    }

    #[test]
    fn test_zero_amount_invalid() {
        let inputs = ZkPublicInputs::new(
            [1u8; 32],
            [2u8; 32],
            test_pubkey(),
            0, // Zero amount
            test_pubkey(),
            0,
        );
        assert!(inputs.validate().is_err());
    }

    #[test]
    fn test_fee_exceeds_amount_invalid() {
        let inputs = ZkPublicInputs::new(
            [1u8; 32],
            [2u8; 32],
            test_pubkey(),
            100,
            test_pubkey(),
            200, // Fee > amount
        );
        assert!(inputs.validate().is_err());
    }

    #[test]
    fn test_fee_equals_amount_valid() {
        let inputs = ZkPublicInputs::new(
            [1u8; 32],
            [2u8; 32],
            test_pubkey(),
            100,
            test_pubkey(),
            100, // Fee = amount (all goes to relayer)
        );
        assert!(inputs.validate().is_ok());
        assert_eq!(inputs.net_amount().unwrap(), 0);
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
        let elements = inputs.to_field_elements();
        assert_eq!(elements.len(), ZkPublicInputs::COUNT);
    }

    #[test]
    fn test_self_relay() {
        let addr = test_pubkey();
        let inputs = ZkPublicInputs::new(
            [1u8; 32],
            [2u8; 32],
            addr,
            1000,
            addr, // Same as recipient
            0,    // No fee
        );
        assert!(inputs.is_self_relay());
    }

    #[test]
    fn test_builder() {
        let result = ZkPublicInputsBuilder::new()
            .merkle_root([1u8; 32])
            .nullifier_hash([2u8; 32])
            .recipient(test_pubkey())
            .amount(1000)
            .relayer(test_pubkey())
            .relayer_fee(100)
            .build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_builder_missing_field() {
        let result = ZkPublicInputsBuilder::new()
            .merkle_root([1u8; 32])
            // Missing nullifier_hash
            .recipient(test_pubkey())
            .amount(1000)
            .build();
        assert!(result.is_err());
    }

    #[test]
    fn test_u64_to_field_encoding() {
        let value = 0x0102030405060708u64;
        let field = u64_to_field(value);
        
        // First 24 bytes should be zero
        assert!(field[..24].iter().all(|&b| b == 0));
        
        // Last 8 bytes should be big-endian
        assert_eq!(field[24], 0x01);
        assert_eq!(field[31], 0x08);
    }
}
