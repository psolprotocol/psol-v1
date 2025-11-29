//! Pool Configuration State - Phase 4 Hardened
//!
//! # Security Properties
//! - Authority changes require 2-step process (pending → accept)
//! - VK can be locked to prevent post-deployment changes
//! - All counters use checked arithmetic

use anchor_lang::prelude::*;

use crate::error::PrivacyError;

/// Main pool configuration account.
#[account]
pub struct PoolConfig {
    /// Current pool authority (admin)
    pub authority: Pubkey,
    
    /// Pending authority for 2-step transfer (zero if none pending)
    pub pending_authority: Pubkey,

    /// SPL token mint for deposits/withdrawals
    pub token_mint: Pubkey,

    /// Token vault PDA address
    pub vault: Pubkey,

    /// Merkle tree account address
    pub merkle_tree: Pubkey,

    /// Verification key account address
    pub verification_key: Pubkey,

    /// Merkle tree depth (immutable after init)
    pub tree_depth: u8,

    /// PDA bump seed
    pub bump: u8,

    /// Pool paused flag
    pub is_paused: bool,

    /// Whether verification key has been set
    pub vk_configured: bool,
    
    /// Whether verification key is locked (immutable)
    pub vk_locked: bool,

    /// Total number of deposits processed
    pub total_deposits: u64,

    /// Total number of withdrawals processed
    pub total_withdrawals: u64,
    
    /// Total value deposited
    pub total_value_deposited: u64,
    
    /// Total value withdrawn
    pub total_value_withdrawn: u64,

    /// Schema version
    pub version: u8,

    /// Reserved space for future upgrades
    pub _reserved: [u8; 64],
}

impl PoolConfig {
    pub const LEN: usize = 8 + 32 + 32 + 32 + 32 + 32 + 32 + 1 + 1 + 1 + 1 + 1 + 3 + 8 + 8 + 8 + 8 + 1 + 64;
    pub const VERSION: u8 = 2;

    #[allow(clippy::too_many_arguments)]
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
        self.pending_authority = Pubkey::default();
        self.token_mint = token_mint;
        self.vault = vault;
        self.merkle_tree = merkle_tree;
        self.verification_key = verification_key;
        self.tree_depth = tree_depth;
        self.bump = bump;
        self.is_paused = false;
        self.vk_configured = false;
        self.vk_locked = false;
        self.total_deposits = 0;
        self.total_withdrawals = 0;
        self.total_value_deposited = 0;
        self.total_value_withdrawn = 0;
        self.version = Self::VERSION;
        self._reserved = [0u8; 64];
    }

    #[inline]
    pub fn require_not_paused(&self) -> Result<()> {
        require!(!self.is_paused, PrivacyError::PoolPaused);
        Ok(())
    }

    #[inline]
    pub fn require_vk_configured(&self) -> Result<()> {
        require!(self.vk_configured, PrivacyError::VerificationKeyNotSet);
        Ok(())
    }
    
    #[inline]
    pub fn require_vk_unlocked(&self) -> Result<()> {
        require!(!self.vk_locked, PrivacyError::VerificationKeyLocked);
        Ok(())
    }

    pub fn record_deposit(&mut self, amount: u64) -> Result<()> {
        self.total_deposits = self.total_deposits
            .checked_add(1)
            .ok_or(error!(PrivacyError::ArithmeticOverflow))?;
        self.total_value_deposited = self.total_value_deposited
            .checked_add(amount)
            .ok_or(error!(PrivacyError::ArithmeticOverflow))?;
        Ok(())
    }

    pub fn record_withdrawal(&mut self, amount: u64) -> Result<()> {
        self.total_withdrawals = self.total_withdrawals
            .checked_add(1)
            .ok_or(error!(PrivacyError::ArithmeticOverflow))?;
        self.total_value_withdrawn = self.total_value_withdrawn
            .checked_add(amount)
            .ok_or(error!(PrivacyError::ArithmeticOverflow))?;
        Ok(())
    }

    #[inline]
    pub fn set_paused(&mut self, paused: bool) {
        self.is_paused = paused;
    }

    #[inline]
    pub fn set_vk_configured(&mut self, configured: bool) {
        self.vk_configured = configured;
    }
    
    #[inline]
    pub fn lock_vk(&mut self) {
        self.vk_locked = true;
    }

    pub fn initiate_authority_transfer(&mut self, new_authority: Pubkey) -> Result<()> {
        require!(new_authority != Pubkey::default(), PrivacyError::InvalidAuthority);
        require!(new_authority != self.authority, PrivacyError::InvalidAuthority);
        self.pending_authority = new_authority;
        Ok(())
    }
    
    pub fn accept_authority_transfer(&mut self, acceptor: Pubkey) -> Result<()> {
        require!(self.pending_authority != Pubkey::default(), PrivacyError::NoPendingAuthority);
        require!(acceptor == self.pending_authority, PrivacyError::Unauthorized);
        self.authority = self.pending_authority;
        self.pending_authority = Pubkey::default();
        Ok(())
    }
    
    pub fn cancel_authority_transfer(&mut self) {
        self.pending_authority = Pubkey::default();
    }
    
    #[inline]
    pub fn has_pending_transfer(&self) -> bool {
        self.pending_authority != Pubkey::default()
    }

    #[deprecated(note = "Use record_deposit() for value tracking")]
    pub fn increment_deposits(&mut self) -> Result<()> {
        self.total_deposits = self.total_deposits
            .checked_add(1)
            .ok_or(error!(PrivacyError::ArithmeticOverflow))?;
        Ok(())
    }

    #[deprecated(note = "Use record_withdrawal() for value tracking")]
    pub fn increment_withdrawals(&mut self) -> Result<()> {
        self.total_withdrawals = self.total_withdrawals
            .checked_add(1)
            .ok_or(error!(PrivacyError::ArithmeticOverflow))?;
        Ok(())
    }
}
