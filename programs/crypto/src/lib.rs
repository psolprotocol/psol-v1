use anchor_lang::prelude::*;

pub mod crypto;
pub mod error;
pub mod events;
pub mod instructions;
pub mod state;

use instructions::*;

// IMPORTANT: Replace this with your actual program ID after deployment
// In Solana Playground you will update this later with the deployed ID
declare_id!("Ex714yinMJgdXThcw9G3pxCeuVxZrHVPuYhq6p1chsiE");

#[program]
pub mod psol_privacy {
    use super::*;

    /// Initialize the privacy pool with configuration
    pub fn initialize_pool(
        ctx: Context<InitializePool>,
        tree_depth: u8,
        root_history_size: u16,
    ) -> Result<()> {
        instructions::init::handler(ctx, tree_depth, root_history_size)
    }

    /// Deposit tokens into the privacy pool
    pub fn deposit(ctx: Context<Deposit>, amount: u64, commitment: [u8; 32]) -> Result<()> {
        instructions::deposit::handler(ctx, amount, commitment)
    }

    /// Withdraw tokens from the privacy pool with zero-knowledge proof
    pub fn withdraw(
        ctx: Context<Withdraw>,
        nullifier: [u8; 32],
        recipient: Pubkey,
        amount: u64,
        merkle_root: [u8; 32],
        proof_data: Vec<u8>,
    ) -> Result<()> {
        instructions::withdraw::handler(ctx, nullifier, recipient, amount, merkle_root, proof_data)
    }

    /// Private transfer within the pool (Phase 2 - stub for v1)
    pub fn private_transfer(
        ctx: Context<PrivateTransfer>,
        _input_nullifiers: Vec<[u8; 32]>,
        _output_commitments: Vec<[u8; 32]>,
        _proof_data: Vec<u8>,
    ) -> Result<()> {
        instructions::transfer::handler(ctx)
    }
}