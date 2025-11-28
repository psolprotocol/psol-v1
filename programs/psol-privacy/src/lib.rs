//! pSol Privacy Pool - Phase 3

use anchor_lang::prelude::*;

pub mod crypto;
pub mod error;
pub mod events;
pub mod instructions;
pub mod state;

#[cfg(test)]
mod tests;

use instructions::*;

declare_id!("Ddokrq1M6hT9Vu63k4JWqVRSecyLeotNf8xKknKfRwvZ");

#[program]
pub mod psol_privacy {
    use super::*;

    pub fn initialize_pool(
        ctx: Context<InitializePool>,
        tree_depth: u8,
        root_history_size: u16,
    ) -> Result<()> {
        instructions::initialize_pool::handler(ctx, tree_depth, root_history_size)
    }

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

    pub fn deposit(
        ctx: Context<Deposit>,
        amount: u64,
        commitment: [u8; 32],
    ) -> Result<()> {
        instructions::deposit::handler(ctx, amount, commitment)
    }

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

    pub fn private_transfer(
        ctx: Context<PrivateTransfer>,
        _input_nullifiers: Vec<[u8; 32]>,
        _output_commitments: Vec<[u8; 32]>,
        _proof_data: Vec<u8>,
    ) -> Result<()> {
        instructions::private_transfer::handler(ctx)
    }

    pub fn pause_pool(ctx: Context<PausePool>) -> Result<()> {
        instructions::admin::pause::handler(ctx)
    }

    pub fn unpause_pool(ctx: Context<UnpausePool>) -> Result<()> {
        instructions::admin::unpause::handler(ctx)
    }

    pub fn update_authority(ctx: Context<UpdateAuthority>, new_authority: Pubkey) -> Result<()> {
        instructions::admin::update_authority::handler(ctx, new_authority)
    }
}
