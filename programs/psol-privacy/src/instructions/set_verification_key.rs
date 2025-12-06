//! Set Verification Key Instruction - Devnet Alpha Hardened

use anchor_lang::prelude::*;

use crate::crypto::{is_g1_identity, is_g2_identity, public_inputs::ZkPublicInputs, validate_g1_point, validate_g2_point};
use crate::error::PrivacyError;
use crate::events::{VerificationKeyLocked, VerificationKeySet};
use crate::state::{PoolConfig, VerificationKeyAccount};

pub const MAX_IC_POINTS: usize = 16;
pub const MIN_IC_POINTS: usize = 2;

#[derive(Accounts)]
pub struct SetVerificationKey<'info> {
    #[account(
        mut,
        seeds = [b"pool", pool_config.token_mint.as_ref()],
        bump = pool_config.bump,
        has_one = authority @ PrivacyError::Unauthorized,
    )]
    pub pool_config: Account<'info, PoolConfig>,

    #[account(
        mut,
        seeds = [b"verification_key", pool_config.key().as_ref()],
        bump = verification_key.bump,
        constraint = verification_key.pool == pool_config.key() @ PrivacyError::Unauthorized,
    )]
    pub verification_key: Account<'info, VerificationKeyAccount>,

    pub authority: Signer<'info>,
}

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

    // Hardened lifecycle:
    // In production, the verification key must be set once, before any deposits exist.
    // This prevents an attacker who compromises the authority later from swapping in
    // a malicious VK while the pool holds user funds.
    //
    // We enforce that VK cannot be changed once there have been any deposits.
    // (Assumes PoolConfig tracks total_deposits.)
    require!(
        pool_config.total_deposits == 0,
        PrivacyError::VerificationKeyLocked
    );

    // Still require the VK to be unlocked (not permanently locked)
    pool_config.require_vk_unlocked()?;

    let ic_len = vk_ic.len();
    require!(ic_len >= MIN_IC_POINTS, PrivacyError::InvalidPublicInputs);
    require!(ic_len <= MAX_IC_POINTS, PrivacyError::InputTooLarge);
    require!(
        ic_len == ZkPublicInputs::COUNT + 1,
        PrivacyError::InvalidPublicInputs
    );

    // Basic structural validation of VK points

    require!(
        !is_g1_identity(&vk_alpha_g1),
        PrivacyError::VerificationKeyNotSet
    );
    validate_g1_point(&vk_alpha_g1).map_err(|_| error!(PrivacyError::InvalidProof))?;

    require!(
        !is_g2_identity(&vk_beta_g2),
        PrivacyError::VerificationKeyNotSet
    );
    validate_g2_point(&vk_beta_g2).map_err(|_| error!(PrivacyError::InvalidProof))?;

    require!(
        !is_g2_identity(&vk_gamma_g2),
        PrivacyError::VerificationKeyNotSet
    );
    validate_g2_point(&vk_gamma_g2).map_err(|_| error!(PrivacyError::InvalidProof))?;

    require!(
        !is_g2_identity(&vk_delta_g2),
        PrivacyError::VerificationKeyNotSet
    );
    validate_g2_point(&vk_delta_g2).map_err(|_| error!(PrivacyError::InvalidProof))?;

    for (i, ic_point) in vk_ic.iter().enumerate() {
        validate_g1_point(ic_point).map_err(|_| {
            msg!("IC[{}] failed validation", i);
            error!(PrivacyError::InvalidProof)
        })?;
    }

    // Store VK on-chain
    verification_key.set_vk(
        vk_alpha_g1,
        vk_beta_g2,
        vk_gamma_g2,
        vk_delta_g2,
        vk_ic.clone(),
    );
    pool_config.set_vk_configured(true);

    emit!(VerificationKeySet {
        pool: pool_config.key(),
        authority: ctx.accounts.authority.key(),
        ic_length: ic_len as u8,
        timestamp: Clock::get()?.unix_timestamp,
    });

    msg!("Verification key set successfully");
    Ok(())
}

#[derive(Accounts)]
pub struct LockVerificationKey<'info> {
    #[account(
        mut,
        seeds = [b"pool", pool_config.token_mint.as_ref()],
        bump = pool_config.bump,
        has_one = authority @ PrivacyError::Unauthorized,
    )]
    pub pool_config: Account<'info, PoolConfig>,

    pub authority: Signer<'info>,
}

pub fn lock_vk_handler(ctx: Context<LockVerificationKey>) -> Result<()> {
    let pool_config = &mut ctx.accounts.pool_config;

    pool_config.require_vk_configured()?;
    require!(
        !pool_config.vk_locked,
        PrivacyError::VerificationKeyLocked
    );

    pool_config.lock_vk();

    emit!(VerificationKeyLocked {
        pool: pool_config.key(),
        authority: ctx.accounts.authority.key(),
        timestamp: Clock::get()?.unix_timestamp,
    });

    msg!("VERIFICATION KEY LOCKED PERMANENTLY");
    Ok(())
}
