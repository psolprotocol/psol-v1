//! Set Verification Key Instruction
//!
//! Allows pool authority to configure the Groth16 verification key.
//! The VK must come from a trusted setup ceremony for the withdrawal circuit.
//!
//! # Security
//! - Only callable by pool authority
//! - VK integrity is critical - compromised VK = compromised pool
//! - In production, consider making VK immutable after first set

use anchor_lang::prelude::*;

use crate::error::PrivacyError;
use crate::events::VerificationKeySet;
use crate::state::{PoolConfig, VerificationKeyAccount};

/// Accounts for set_verification_key instruction.
#[derive(Accounts)]
pub struct SetVerificationKey<'info> {
    /// Pool configuration account.
    #[account(
        mut,
        seeds = [b"pool", pool_config.token_mint.as_ref()],
        bump = pool_config.bump,
        has_one = authority @ PrivacyError::Unauthorized,
    )]
    pub pool_config: Account<'info, PoolConfig>,

    /// Verification key account to update.
    #[account(
        mut,
        seeds = [b"verification_key", pool_config.key().as_ref()],
        bump = verification_key.bump,
        constraint = verification_key.pool == pool_config.key() @ PrivacyError::Unauthorized,
    )]
    pub verification_key: Account<'info, VerificationKeyAccount>,

    /// Pool authority (must sign).
    pub authority: Signer<'info>,
}

/// Handler for set_verification_key instruction.
///
/// # Arguments
/// * `vk_alpha_g1` - Alpha point in G1 (64 bytes uncompressed)
/// * `vk_beta_g2` - Beta point in G2 (128 bytes uncompressed)
/// * `vk_gamma_g2` - Gamma point in G2 (128 bytes uncompressed)
/// * `vk_delta_g2` - Delta point in G2 (128 bytes uncompressed)
/// * `vk_ic` - IC points in G1 (variable length, each 64 bytes)
pub fn handler(
    ctx: Context<SetVerificationKey>,
    vk_alpha_g1: [u8; 64],
    vk_beta_g2: [u8; 128],
    vk_gamma_g2: [u8; 128],
    vk_delta_g2: [u8; 128],
    vk_ic: Vec<[u8; 64]>,
) -> Result<()> {
    let pool_config = &mut ctx.accounts.pool_config;
    let verification_key = &mut ctx.accounts.verification_key;

    // Validate IC length
    // For withdrawal circuit with 6 public inputs, we need 7 IC points
    require!(
        vk_ic.len() >= 2 && vk_ic.len() <= VerificationKeyAccount::DEFAULT_MAX_IC_POINTS as usize,
        PrivacyError::InvalidPublicInputs
    );

    // Basic validation: alpha should not be identity
    require!(
        !vk_alpha_g1.iter().all(|&b| b == 0),
        PrivacyError::InvalidProof
    );

    // Basic validation: IC[0] should not be identity
    require!(
        !vk_ic[0].iter().all(|&b| b == 0),
        PrivacyError::InvalidProof
    );

    // TODO [PHASE 3]: Add on-curve validation for all points
    // This requires implementing BN254 curve checks

    // Store verification key
    verification_key.set_vk(
        vk_alpha_g1,
        vk_beta_g2,
        vk_gamma_g2,
        vk_delta_g2,
        vk_ic.clone(),
    );

    // Mark pool as having VK configured
    pool_config.set_vk_configured(true);

    // Emit event
    emit!(VerificationKeySet {
        pool: pool_config.key(),
        authority: ctx.accounts.authority.key(),
        ic_length: vk_ic.len() as u8,
        timestamp: Clock::get()?.unix_timestamp,
    });

    msg!("Verification key set successfully");
    msg!("IC points: {}", vk_ic.len());
    msg!("Expected public inputs: {}", vk_ic.len() - 1);

    Ok(())
}
