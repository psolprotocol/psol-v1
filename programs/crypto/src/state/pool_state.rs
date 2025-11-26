use anchor_lang::prelude::*;

/// Main pool configuration and state
#[account]
pub struct PoolConfig {
    /// Pool authority (admin)
    pub authority: Pubkey,

    /// SPL token mint for deposits/withdrawals
    pub token_mint: Pubkey,

    /// Token vault PDA holding all deposited funds
    pub vault: Pubkey,

    /// Merkle tree depth
    pub tree_depth: u8,

    /// Current merkle root
    pub current_root: [u8; 32],

    /// Total deposits processed
    pub total_deposits: u64,

    /// Total withdrawals processed
    pub total_withdrawals: u64,

    /// Pool paused flag
    pub is_paused: bool,

    /// Bump seed for PDA derivation
    pub bump: u8,
}

impl PoolConfig {
    pub const LEN: usize = 8 + // discriminator
        32 + // authority
        32 + // token_mint
        32 + // vault
        1 +  // tree_depth
        32 + // current_root
        8 +  // total_deposits
        8 +  // total_withdrawals
        1 +  // is_paused
        1; // bump

    /// Initialize a new pool configuration
    pub fn initialize(
        &mut self,
        authority: Pubkey,
        token_mint: Pubkey,
        vault: Pubkey,
        tree_depth: u8,
        bump: u8,
    ) -> Result<()> {
        self.authority = authority;
        self.token_mint = token_mint;
        self.vault = vault;
        self.tree_depth = tree_depth;
        self.current_root = [0u8; 32]; // Will be updated with tree initialization
        self.total_deposits = 0;
        self.total_withdrawals = 0;
        self.is_paused = false;
        self.bump = bump;
        Ok(())
    }

    /// Update the current merkle root
    pub fn update_root(&mut self, new_root: [u8; 32]) {
        self.current_root = new_root;
    }

    /// Increment deposit counter
    pub fn increment_deposits(&mut self) -> Result<()> {
        self.total_deposits = self
            .total_deposits
            .checked_add(1)
            .ok_or(error!(crate::error::PrivacyError::ArithmeticOverflow))?;
        Ok(())
    }

    /// Increment withdrawal counter
    pub fn increment_withdrawals(&mut self) -> Result<()> {
        self.total_withdrawals = self
            .total_withdrawals
            .checked_add(1)
            .ok_or(error!(crate::error::PrivacyError::ArithmeticOverflow))?;
        Ok(())
    }

    /// Check if pool is paused
    pub fn require_not_paused(&self) -> Result<()> {
        require!(!self.is_paused, crate::error::PrivacyError::PoolPaused);
        Ok(())
    }
}
