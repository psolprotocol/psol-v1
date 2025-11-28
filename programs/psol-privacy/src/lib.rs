//! pSol Privacy Pool - Phase 3 Implementation
//!
//! A ZK-based privacy pool for Solana using commitment/nullifier scheme
//! with Groth16 proof verification.
//!
//! # Protocol Overview
//!
//! pSol enables private token transfers on Solana through a shielded pool:
//!
//! ```text
//! ┌─────────────┐     deposit      ┌──────────────┐     withdraw     ┌─────────────┐
//! │   Public    │ ───────────────► │   Shielded   │ ───────────────► │   Public    │
//! │   Tokens    │                  │     Pool     │                  │   Tokens    │
//! └─────────────┘                  └──────────────┘                  └─────────────┘
//!       │                                │                                  ▲
//!       │                                │                                  │
//!       └─ commitment inserted ──────────┴─ ZK proof verified ──────────────┘
//! ```
//!
//! ## Core Concepts
//!
//! ### Commitments
//! A commitment is computed as: `commitment = Poseidon(secret, nullifier_preimage, amount)`
//! - `secret`: 32-byte random value (user must keep private)
//! - `nullifier_preimage`: 32-byte random value (user must keep private)
//! - `amount`: Token amount being deposited
//!
//! Commitments are stored in an incremental Merkle tree on-chain.
//!
//! ### Nullifiers
//! A nullifier is computed as: `nullifier_hash = Poseidon(nullifier_preimage, secret)`
//!
//! The nullifier is revealed on-chain during withdrawal to prevent double-spending.
//! Each nullifier can only be used once (enforced via PDAs).
//!
//! ### Zero-Knowledge Proofs
//! Withdrawals require a Groth16 proof demonstrating:
//! 1. Knowledge of `(secret, nullifier_preimage)` for a commitment in the tree
//! 2. The commitment was computed correctly with the claimed amount
//! 3. The nullifier_hash was derived correctly
//! 4. The merkle_root, recipient, amount, relayer, and fee match public inputs
//!
//! ## Protocol Flows
//!
//! ### Deposit Flow
//! ```text
//! 1. User generates (secret, nullifier_preimage) randomly
//! 2. User calls deposit(amount, secret, nullifier_preimage)
//! 3. On-chain:
//!    a. Compute commitment = Poseidon(secret, nullifier_preimage, amount)
//!    b. Transfer tokens from user to vault
//!    c. Insert commitment into Merkle tree
//!    d. Emit DepositEvent with leaf_index
//! 4. User saves (secret, nullifier_preimage, leaf_index) securely
//! ```
//!
//! ### Withdrawal Flow
//! ```text
//! 1. User fetches Merkle tree state to compute path to their leaf
//! 2. User generates Groth16 proof off-chain with:
//!    - Private inputs: secret, nullifier_preimage, merkle_path
//!    - Public inputs: merkle_root, nullifier_hash, recipient, amount, relayer, fee
//! 3. User (or relayer) calls withdraw with proof and public inputs
//! 4. On-chain:
//!    a. Verify merkle_root is in recent history
//!    b. Verify nullifier_hash PDA doesn't exist (not spent)
//!    c. Verify Groth16 proof
//!    d. Create SpentNullifier PDA (marks as spent)
//!    e. Transfer (amount - fee) to recipient
//!    f. Transfer fee to relayer
//! ```
//!
//! ## Security Properties
//!
//! - **Hiding**: Deposits and withdrawals cannot be linked
//! - **Binding**: Users cannot claim more than they deposited
//! - **Double-spend prevention**: Each commitment can only be withdrawn once
//! - **Amount privacy**: Withdrawal amounts are independent of deposit amounts
//! - **Fail-closed**: Invalid proofs are always rejected
//!
//! ## Build Configurations
//!
//! - Default: Production mode with full ZK verification
//! - `--features dev-mode`: Testing mode that bypasses proof verification
//!   (NEVER use in production!)
//!
//! ## Account Structure
//!
//! - `PoolConfig`: Pool settings and authority (PDA: ["pool", token_mint])
//! - `MerkleTree`: Commitment storage (PDA: ["merkle_tree", pool_config])
//! - `VerificationKeyAccount`: Groth16 VK (PDA: ["verification_key", pool_config])
//! - `SpentNullifier`: Per-nullifier marker (PDA: ["nullifier", pool_config, nullifier_hash])
//! - `Vault`: Token account (PDA: ["vault", pool_config])

use anchor_lang::prelude::*;

pub mod crypto;
pub mod error;
pub mod events;
pub mod instructions;
pub mod state;

#[cfg(test)]
mod tests;

use instructions::*;

// IMPORTANT: Replace with actual program ID after deployment
declare_id!("Ddokrq1M6hT9Vu63k4JWqVRSecyLeotNf8xKknKfRwvZ");

#[program]
pub mod psol_privacy {
    use super::*;

    /// Initialize a new privacy pool for a specific SPL token.
    ///
    /// Creates:
    /// - PoolConfig PDA (pool settings and authority)
    /// - MerkleTree PDA (commitment storage)
    /// - VerificationKey PDA (Groth16 VK storage)
    /// - Vault token account (holds deposited tokens)
    ///
    /// # Arguments
    /// * `tree_depth` - Merkle tree depth (4-24, recommended: 20 for ~1M leaves)
    /// * `root_history_size` - Number of historical roots to store (min: 30, recommended: 100+)
    ///
    /// # Security
    /// The caller becomes the pool authority with admin privileges.
    pub fn initialize_pool(
        ctx: Context<InitializePool>,
        tree_depth: u8,
        root_history_size: u16,
    ) -> Result<()> {
        instructions::initialize_pool::handler(ctx, tree_depth, root_history_size)
    }

    /// Set or update the Groth16 verification key for the pool.
    ///
    /// The VK must come from a trusted setup ceremony for the withdrawal circuit.
    ///
    /// # Arguments
    /// * `vk_alpha_g1` - α point in G1 (64 bytes uncompressed)
    /// * `vk_beta_g2` - β point in G2 (128 bytes uncompressed)
    /// * `vk_gamma_g2` - γ point in G2 (128 bytes uncompressed)
    /// * `vk_delta_g2` - δ point in G2 (128 bytes uncompressed)
    /// * `vk_ic` - IC points in G1 (should be 7 for 6 public inputs)
    ///
    /// # Security
    /// - Only callable by pool authority
    /// - VK integrity is critical - a malicious VK enables fund theft
    /// - In production, VK should be set once and made immutable
    pub fn set_verification_key(
        ctx: Context<SetVerificationKey>,
        vk_alpha_g1: [u8; 64],
        vk_beta_g2: [u8; 128],
        vk_gamma_g2: [u8; 128],
        vk_delta_g2: [u8; 128],
        vk_ic: Vec<[u8; 64]>,
    ) -> Result<()> {
        instructions::set_verification_key::handler(
            ctx,
            vk_alpha_g1,
            vk_beta_g2,
            vk_gamma_g2,
            vk_delta_g2,
            vk_ic,
        )
    }

    /// Deposit tokens into the privacy pool.
    ///
    /// The commitment is computed on-chain from the provided inputs,
    /// ensuring amount binding and circuit compatibility.
    ///
    /// # Arguments
    /// * `amount` - Token amount to deposit (must be > 0)
    /// * `secret` - Random secret (32 bytes) - USER MUST SAVE THIS!
    /// * `nullifier_preimage` - Nullifier preimage (32 bytes) - USER MUST SAVE THIS!
    ///
    /// # Returns
    /// Emits DepositEvent with leaf_index needed for withdrawal proof.
    ///
    /// # Security
    /// - `secret` and `nullifier_preimage` should be cryptographically random
    /// - User must store these values securely - they're needed for withdrawal
    /// - Lost secret/nullifier = lost funds (no recovery mechanism)
    ///
    /// # Commitment Computation
    /// ```text
    /// commitment = Poseidon(secret, nullifier_preimage, amount)
    /// ```
    pub fn deposit(
        ctx: Context<Deposit>,
        amount: u64,
        secret: [u8; 32],
        nullifier_preimage: [u8; 32],
    ) -> Result<()> {
        instructions::deposit::handler(ctx, amount, secret, nullifier_preimage)
    }

    /// Withdraw tokens from the privacy pool using a ZK proof.
    ///
    /// # Arguments
    /// * `proof_data` - Serialized Groth16 proof (256 bytes: A || B || C)
    /// * `merkle_root` - Root to prove membership against (must be in history)
    /// * `nullifier_hash` - Hash of nullifier (prevents double-spend)
    /// * `recipient` - Address to receive withdrawn tokens
    /// * `amount` - Amount to withdraw (before fee)
    /// * `relayer` - Relayer address (receives fee)
    /// * `relayer_fee` - Fee paid to relayer from withdrawal amount
    ///
    /// # Verification Steps
    /// 1. Check pool is not paused
    /// 2. Check VK is configured
    /// 3. Validate amount and fee
    /// 4. Verify vault has sufficient balance
    /// 5. Verify merkle_root is in recent history
    /// 6. Verify nullifier hasn't been spent (via PDA creation)
    /// 7. Verify Groth16 proof
    /// 8. Transfer tokens
    ///
    /// # Dev Mode
    /// When compiled with `dev-mode` feature, proof verification is bypassed.
    /// This is ONLY for testing - NEVER use in production!
    #[allow(clippy::too_many_arguments)]
    pub fn withdraw(
        ctx: Context<Withdraw>,
        proof_data: Vec<u8>,
        merkle_root: [u8; 32],
        nullifier_hash: [u8; 32],
        recipient: Pubkey,
        amount: u64,
        relayer: Pubkey,
        relayer_fee: u64,
    ) -> Result<()> {
        instructions::withdraw::handler(
            ctx,
            proof_data,
            merkle_root,
            nullifier_hash,
            recipient,
            amount,
            relayer,
            relayer_fee,
        )
    }

    /// Private transfer within the pool (Phase 3 feature).
    ///
    /// Allows transferring value between commitments without leaving
    /// the privacy pool. NOT IMPLEMENTED - returns error.
    ///
    /// # Future Design
    /// 1. User proves knowledge of N input commitments
    /// 2. User provides M output commitments
    /// 3. Circuit verifies: sum(input_amounts) = sum(output_amounts)
    /// 4. Input nullifiers are marked spent, output commitments inserted
    pub fn private_transfer(
        ctx: Context<PrivateTransfer>,
        _input_nullifiers: Vec<[u8; 32]>,
        _output_commitments: Vec<[u8; 32]>,
        _proof_data: Vec<u8>,
    ) -> Result<()> {
        instructions::private_transfer::handler(ctx)
    }

    // ========== Admin Instructions ==========

    /// Pause the pool (emergency stop).
    /// Only callable by pool authority.
    /// Blocks all deposits and withdrawals.
    pub fn pause_pool(ctx: Context<PausePool>) -> Result<()> {
        instructions::admin::pause::handler(ctx)
    }

    /// Unpause the pool.
    /// Only callable by pool authority.
    pub fn unpause_pool(ctx: Context<UnpausePool>) -> Result<()> {
        instructions::admin::unpause::handler(ctx)
    }

    /// Transfer pool authority to a new address.
    /// Only callable by current authority.
    pub fn update_authority(ctx: Context<UpdateAuthority>, new_authority: Pubkey) -> Result<()> {
        instructions::admin::update_authority::handler(ctx, new_authority)
    }
}
