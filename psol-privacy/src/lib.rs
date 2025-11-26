//! pSol Privacy Pool - Phase 2 Skeleton
//!
//! A ZK-based privacy pool for Solana using commitment/nullifier scheme.
//! This is a FAIL-CLOSED skeleton: withdrawals are disabled until real
//! Groth16 verification is implemented in Phase 3.
//!
//! Architecture:
//! - Incremental Merkle tree for commitment storage
//! - Per-nullifier PDA for O(1) double-spend prevention
//! - Groth16 proof verification (stub - fails closed)
//! - Poseidon hashing interface (placeholder implementation)

use anchor_lang::prelude::*;

pub mod crypto;
pub mod error;
pub mod events;
pub mod instructions;
pub mod state;

use instructions::*;

// IMPORTANT: Replace with actual program ID after deployment
declare_id!("PSoL1111111111111111111111111111111111111111");

#[program]
pub mod psol_privacy {
    use super::*;

    /// Initialize a new privacy pool for a specific SPL token.
    ///
    /// Creates:
    /// - PoolConfig PDA (pool settings and authority)
    /// - MerkleTree PDA (commitment storage)
    /// - Vault token account (holds deposited tokens)
    ///
    /// # Arguments
    /// * `tree_depth` - Merkle tree depth (recommended: 20 for ~1M leaves)
    /// * `root_history_size` - Number of historical roots to store (recommended: 100+)
    pub fn initialize_pool(
        ctx: Context<InitializePool>,
        tree_depth: u8,
        root_history_size: u16,
    ) -> Result<()> {
        instructions::initialize_pool::handler(ctx, tree_depth, root_history_size)
    }

    /// Set or update the Groth16 verification key for the pool.
    ///
    /// Only callable by pool authority. The VK must come from a trusted
    /// setup ceremony for the withdrawal circuit.
    ///
    /// # Security
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
    /// The user provides secret and nullifier preimages, and the on-chain
    /// program computes the commitment. This ensures commitment binding
    /// and allows the circuit to replicate the same computation.
    ///
    /// # Arguments
    /// * `amount` - Token amount to deposit
    /// * `secret` - Random secret (32 bytes) - user must save this!
    /// * `nullifier_preimage` - Nullifier preimage (32 bytes) - user must save this!
    ///
    /// # Returns
    /// Emits Deposit event with leaf_index needed for withdrawal proof
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
    /// # PHASE 2 STATUS: FAIL-CLOSED
    /// This instruction currently ALWAYS FAILS because the Groth16
    /// verifier is not yet implemented. This is intentional security
    /// behavior - no funds can be withdrawn until real ZK verification
    /// is added in Phase 3.
    ///
    /// # Arguments
    /// * `proof_data` - Serialized Groth16 proof (A, B, C points)
    /// * `merkle_root` - Root to prove membership against
    /// * `nullifier_hash` - Hash of nullifier (prevents double-spend)
    /// * `recipient` - Address to receive withdrawn tokens
    /// * `amount` - Amount to withdraw
    /// * `relayer` - Relayer address (receives fee)
    /// * `relayer_fee` - Fee paid to relayer from withdrawal amount
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
