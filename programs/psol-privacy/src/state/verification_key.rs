//! Verification Key storage for Groth16 proofs
//!
//! Stores the verification key from the trusted setup ceremony.
//! The VK is used to verify withdrawal proofs.
//!
//! # Security
//! - VK MUST come from a properly executed trusted setup
//! - Compromised VK = compromised pool (fake proofs possible)
//! - VK should be immutable after initial setup in production

use anchor_lang::prelude::*;

/// Groth16 Verification Key account.
///
/// Stores the VK in a format compatible with BN254/alt_bn128 curves.
///
/// PDA Seeds: `[b"verification_key", pool_config.key().as_ref()]`
///
/// # Point Encodings
/// - G1 points: 64 bytes (32 bytes x, 32 bytes y) - uncompressed
/// - G2 points: 128 bytes (64 bytes x, 64 bytes y) - uncompressed
///
/// # Note
/// For BN254, G1 and G2 points use different field representations.
/// Ensure encoding matches what the verifier expects.
#[account]
pub struct VerificationKeyAccount {
    /// Reference to parent pool
    pub pool: Pubkey,

    /// α ∈ G1 - Part of the verification equation
    pub vk_alpha_g1: [u8; 64],

    /// β ∈ G2 - Part of the verification equation
    pub vk_beta_g2: [u8; 128],

    /// γ ∈ G2 - Used for public input accumulation
    pub vk_gamma_g2: [u8; 128],

    /// δ ∈ G2 - Used for proof verification
    pub vk_delta_g2: [u8; 128],

    /// Number of IC points (= number of public inputs + 1)
    pub vk_ic_len: u8,

    /// IC points ∈ G1 - Used for public input linear combination
    /// IC[0] + Σ(public_input[i] * IC[i+1])
    ///
    /// For withdrawal circuit with 6 public inputs:
    /// - merkle_root, nullifier, recipient, amount, relayer, relayer_fee
    /// - vk_ic_len should be 7 (6 inputs + 1 base point)
    pub vk_ic: Vec<[u8; 64]>,

    /// Whether this VK has been initialized
    pub is_initialized: bool,

    /// PDA bump seed
    pub bump: u8,
}

impl VerificationKeyAccount {
    /// Calculate space for VK account.
    ///
    /// # Arguments
    /// * `max_ic_points` - Maximum number of IC points to support
    ///
    /// # Note
    /// For a circuit with N public inputs, you need N+1 IC points.
    /// Typical withdrawal circuit has ~6 public inputs → 7 IC points.
    pub fn space(max_ic_points: u8) -> usize {
        8                                   // discriminator
            + 32                            // pool
            + 64                            // vk_alpha_g1
            + 128                           // vk_beta_g2
            + 128                           // vk_gamma_g2
            + 128                           // vk_delta_g2
            + 1                             // vk_ic_len
            + 4 + (64 * max_ic_points as usize) // vk_ic (vec)
            + 1                             // is_initialized
            + 1                             // bump
    }

    /// Default max IC points for withdrawal circuit
    /// 6 public inputs + 1 = 7
    pub const DEFAULT_MAX_IC_POINTS: u8 = 10;

    /// Initialize the VK account (empty, not yet configured)
    pub fn initialize(&mut self, pool: Pubkey, bump: u8) {
        self.pool = pool;
        self.vk_alpha_g1 = [0u8; 64];
        self.vk_beta_g2 = [0u8; 128];
        self.vk_gamma_g2 = [0u8; 128];
        self.vk_delta_g2 = [0u8; 128];
        self.vk_ic_len = 0;
        self.vk_ic = Vec::new();
        self.is_initialized = false;
        self.bump = bump;
    }

    /// Set the verification key data.
    ///
    /// # Arguments
    /// All point data in uncompressed form
    pub fn set_vk(
        &mut self,
        alpha_g1: [u8; 64],
        beta_g2: [u8; 128],
        gamma_g2: [u8; 128],
        delta_g2: [u8; 128],
        ic: Vec<[u8; 64]>,
    ) {
        self.vk_alpha_g1 = alpha_g1;
        self.vk_beta_g2 = beta_g2;
        self.vk_gamma_g2 = gamma_g2;
        self.vk_delta_g2 = delta_g2;
        self.vk_ic_len = ic.len() as u8;
        self.vk_ic = ic;
        self.is_initialized = true;
    }

    /// Check if VK is properly initialized
    pub fn is_valid(&self) -> bool {
        self.is_initialized && self.vk_ic_len > 0
    }

    /// Get expected number of public inputs based on IC length
    pub fn expected_public_inputs(&self) -> u8 {
        if self.vk_ic_len > 0 {
            self.vk_ic_len - 1
        } else {
            0
        }
    }
}

/// Represents Groth16 VK in a format suitable for verification.
/// This is a helper struct for verification logic.
#[derive(Clone, Debug)]
pub struct VerificationKey {
    pub alpha_g1: [u8; 64],
    pub beta_g2: [u8; 128],
    pub gamma_g2: [u8; 128],
    pub delta_g2: [u8; 128],
    pub ic: Vec<[u8; 64]>,
}

impl From<&VerificationKeyAccount> for VerificationKey {
    fn from(account: &VerificationKeyAccount) -> Self {
        VerificationKey {
            alpha_g1: account.vk_alpha_g1,
            beta_g2: account.vk_beta_g2,
            gamma_g2: account.vk_gamma_g2,
            delta_g2: account.vk_delta_g2,
            ic: account.vk_ic.clone(),
        }
    }
}
