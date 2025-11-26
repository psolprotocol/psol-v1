// programs/psol-privacy/src/crypto/nullifier.rs

use anchor_lang::prelude::*;

/// Nullifier set to track spent commitments
/// NOTE: For v1, using simple Vec storage. For production at scale,
/// consider: bitmap, bloom filter, or separate nullifier accounts per nullifier
#[account]
pub struct NullifierSet {
    /// Reference to parent pool
    pub pool: Pubkey,

    /// Maximum capacity (set at initialization)
    /// This MUST match what was used in space() calculation
    pub max_capacity: u32,

    /// Count of spent nullifiers
    pub count: u64,

    /// Set of spent nullifiers
    /// OPTIMIZATION TODO: Replace with bitmap or account-per-nullifier pattern
    pub nullifiers: Vec<[u8; 32]>,
}

impl NullifierSet {
    /// Calculate space for nullifier set
    ///
    /// IMPORTANT: max_nullifiers should be reasonable for testnet (64-256)
    /// For mainnet, consider larger values or alternative storage patterns
    pub fn space(max_nullifiers: u32) -> usize {
        8                                   // discriminator
        + 32                                // pool (Pubkey)
        + 4                                 // max_capacity (u32)
        + 8                                 // count (u64)
        + 4                                 // nullifiers vec length prefix
        + (32 * max_nullifiers as usize) // nullifiers entries
    }

    /// Initialize nullifier set
    pub fn initialize(&mut self, pool: Pubkey, max_capacity: u32) {
        self.pool = pool;
        self.max_capacity = max_capacity;
        self.count = 0;
        self.nullifiers = Vec::new();
    }

    /// Check if nullifier has been spent
    pub fn is_spent(&self, nullifier: &[u8; 32]) -> bool {
        self.nullifiers.contains(nullifier)
    }

    /// Mark nullifier as spent
    pub fn mark_spent(&mut self, nullifier: [u8; 32]) -> Result<()> {
        // Check not already spent
        require!(
            !self.is_spent(&nullifier),
            crate::error::PrivacyError::NullifierAlreadySpent
        );

        // Check capacity against actual allocated space
        require!(
            (self.nullifiers.len() as u32) < self.max_capacity,
            crate::error::PrivacyError::NullifierSetFull
        );

        // Add to set
        self.nullifiers.push(nullifier);

        // Increment counter
        self.count = self
            .count
            .checked_add(1)
            .ok_or(error!(crate::error::PrivacyError::ArithmeticOverflow))?;

        Ok(())
    }

    /// Get count of spent nullifiers
    pub fn get_count(&self) -> u64 {
        self.count
    }

    /// Get remaining capacity
    pub fn remaining_capacity(&self) -> u32 {
        self.max_capacity
            .saturating_sub(self.nullifiers.len() as u32)
    }
}
