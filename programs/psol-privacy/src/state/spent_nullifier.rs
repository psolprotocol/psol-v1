//! Spent Nullifier tracking using per-nullifier PDA pattern
//!
//! Each spent nullifier gets its own account, enabling O(1) lookup
//! via account existence check. This scales to unlimited nullifiers.
//!
//! # Anti-Double-Spend Mechanism
//! 1. User generates nullifier_hash = hash(nullifier_preimage, secret, ...)
//! 2. On withdrawal, program derives PDA from nullifier_hash
//! 3. If PDA exists → nullifier already spent → reject
//! 4. If PDA doesn't exist → create it → accept withdrawal
//!
//! # Storage Pattern
//! - O(1) lookup: check if account exists
//! - O(1) insert: create new account
//! - Unlimited capacity: no pre-allocated array
//! - Each nullifier uses ~100 bytes (rent-exempt minimum)

use anchor_lang::prelude::*;

/// Spent nullifier marker account.
///
/// PDA Seeds: `[b"nullifier", pool.key().as_ref(), nullifier_hash.as_ref()]`
///
/// # Design Rationale
/// Instead of storing nullifiers in a vector (O(n) lookup), we create
/// a separate account for each spent nullifier. Checking if nullifier
/// is spent = checking if account exists, which is O(1).
///
/// # Storage Cost
/// Each nullifier costs ~0.002 SOL in rent (minimum account size).
/// For privacy pools, this cost is amortized into withdrawal fees.
#[account]
pub struct SpentNullifier {
    /// Reference to parent pool (for validation)
    pub pool: Pubkey,

    /// The nullifier hash that was spent
    /// This is hash(nullifier_preimage, ...) NOT the raw preimage
    pub nullifier_hash: [u8; 32],

    /// Unix timestamp when nullifier was spent
    pub spent_at: i64,

    /// Slot number when nullifier was spent (for indexing)
    pub spent_slot: u64,

    /// PDA bump seed
    pub bump: u8,
}

impl SpentNullifier {
    /// Account space (minimal to reduce rent costs)
    pub const LEN: usize = 8  // discriminator
        + 32                  // pool
        + 32                  // nullifier_hash  
        + 8                   // spent_at
        + 8                   // spent_slot
        + 1;                  // bump

    /// Initialize spent nullifier record
    pub fn initialize(
        &mut self,
        pool: Pubkey,
        nullifier_hash: [u8; 32],
        spent_at: i64,
        spent_slot: u64,
        bump: u8,
    ) {
        self.pool = pool;
        self.nullifier_hash = nullifier_hash;
        self.spent_at = spent_at;
        self.spent_slot = spent_slot;
        self.bump = bump;
    }
}

/// Helper to derive SpentNullifier PDA address.
///
/// # Usage
/// ```ignore
/// let (pda, bump) = SpentNullifier::find_pda(
///     program_id,
///     &pool_config.key(),
///     &nullifier_hash,
/// );
/// ```
impl SpentNullifier {
    /// Derive the PDA address for a nullifier
    pub fn find_pda(
        program_id: &Pubkey,
        pool: &Pubkey,
        nullifier_hash: &[u8; 32],
    ) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[b"nullifier", pool.as_ref(), nullifier_hash.as_ref()],
            program_id,
        )
    }

    /// Get PDA seeds for signing (when bump is known)
    pub fn seeds<'a>(
        pool: &'a Pubkey,
        nullifier_hash: &'a [u8; 32],
        bump: &'a [u8; 1],
    ) -> [&'a [u8]; 4] {
        [b"nullifier", pool.as_ref(), nullifier_hash.as_ref(), bump]
    }

    /// Seed prefix for PDA derivation
    pub const SEED_PREFIX: &'static [u8] = b"nullifier";
}

// ============================================================================
// DEPRECATED: Old NullifierSet pattern (kept for reference, DO NOT USE)
// ============================================================================

/// DEPRECATED: Legacy nullifier set using vector storage.
/// DO NOT USE - kept only for migration reference.
///
/// Problems with this approach:
/// 1. O(n) lookup - doesn't scale
/// 2. Limited capacity - must be sized at init
/// 3. Account size grows unbounded
#[account]
#[deprecated(note = "Use SpentNullifier PDA pattern instead")]
pub struct LegacyNullifierSet {
    pub pool: Pubkey,
    pub max_capacity: u32,
    pub count: u64,
    pub nullifiers: Vec<[u8; 32]>,
}
