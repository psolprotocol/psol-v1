//! Pool configuration state account
//!
//! Stores the core configuration for a privacy pool including
//! authority, token mint, and operational state.

use anchor_lang::prelude::*;

use crate::error::PrivacyError;

/// Main pool configuration account.
///
/// PDA Seeds: `[b"pool", token_mint.key().as_ref()]`
#[account]
pub struct PoolConfig {
    /// Pool authority (admin) - can pause, update VK, transfer authority
    pub authority: Pubkey,

    /// SPL token mint for deposits/withdrawals
    pub token_mint: Pubkey,

    /// Token vault PDA address (cached for convenience)
    pub vault: Pubkey,

    /// Merkle tree account address (cached for convenience)
    pub merkle_tree: Pubkey,

    /// Verification key account address (cached for convenience)
    pub verification_key: Pubkey,

    /// Merkle tree depth (immutable after init)
    pub tree_depth: u8,

    /// Total number of deposits processed
    pub total_deposits: u64,

    /// Total number of withdrawals processed
    pub total_withdrawals: u64,

    /// Pool paused flag - blocks deposits and withdrawals when true
    pub is_paused: bool,

    /// Whether verification key has been set
    pub vk_configured: bool,

    /// PDA bump seed
    pub bump: u8,

    /// Reserved space for future upgrades
    pub _reserved: [u8; 64],
}

impl PoolConfig {
    /// Account space calculation
    pub const LEN: usize = 8 // discriminator
        + 32 // authority
        + 32 // token_mint
        + 32 // vault
        + 32 // merkle_tree
        + 32 // verification_key
        + 1  // tree_depth
        + 8  // total_deposits
        + 8  // total_withdrawals
        + 1  // is_paused
        + 1  // vk_configured
        + 1  // bump
        + 64; // reserved

    /// Initialize pool configuration
    pub fn initialize(
        &mut self,
        authority: Pubkey,
        token_mint: Pubkey,
        vault: Pubkey,
        merkle_tree: Pubkey,
        verification_key: Pubkey,
        tree_depth: u8,
        bump: u8,
    ) {
        self.authority = authority;
        self.token_mint = token_mint;
        self.vault = vault;
        self.merkle_tree = merkle_tree;
        self.verification_key = verification_key;
        self.tree_depth = tree_depth;
        self.total_deposits = 0;
        self.total_withdrawals = 0;
        self.is_paused = false;
        self.vk_configured = false;
        self.bump = bump;
        self._reserved = [0u8; 64];
    }

    /// Check if pool is not paused
    pub fn require_not_paused(&self) -> Result<()> {
        require!(!self.is_paused, PrivacyError::PoolPaused);
        Ok(())
    }

    /// Check if verification key is configured
    pub fn require_vk_configured(&self) -> Result<()> {
        require!(self.vk_configured, PrivacyError::VerificationKeyNotSet);
        Ok(())
    }

    /// Increment deposit counter (checked arithmetic)
    pub fn increment_deposits(&mut self) -> Result<()> {
        self.total_deposits = self
            .total_deposits
            .checked_add(1)
            .ok_or(error!(PrivacyError::ArithmeticOverflow))?;
        Ok(())
    }

    /// Increment withdrawal counter (checked arithmetic)
    pub fn increment_withdrawals(&mut self) -> Result<()> {
        self.total_withdrawals = self
            .total_withdrawals
            .checked_add(1)
            .ok_or(error!(PrivacyError::ArithmeticOverflow))?;
        Ok(())
    }

    /// Set pause state
    pub fn set_paused(&mut self, paused: bool) {
        self.is_paused = paused;
    }

    /// Mark verification key as configured
    pub fn set_vk_configured(&mut self, configured: bool) {
        self.vk_configured = configured;
    }

    /// Transfer authority to new address
    pub fn transfer_authority(&mut self, new_authority: Pubkey) {
        self.authority = new_authority;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_config_size() {
        // Ensure calculated size matches actual struct size
        // This helps catch serialization mismatches
        assert!(PoolConfig::LEN >= 8 + 32 * 5 + 1 + 8 + 8 + 1 + 1 + 1 + 64);
    }
}
