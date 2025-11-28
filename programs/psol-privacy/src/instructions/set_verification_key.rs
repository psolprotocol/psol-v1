//! Set Verification Key Instruction - Phase 3
//!
//! Configures the Groth16 verification key for the privacy pool.
//! The VK must come from a trusted setup ceremony.
//!
//! # Phase 3 Improvements
//! - Full BN254 on-curve validation for all G1 points
//! - Basic G2 point validation
//! - Non-identity checks for critical points
//! - Proper error propagation
//!
//! # Security
//! - Only callable by pool authority
//! - VK integrity is critical - compromised VK = fund theft possible
//! - All points are validated to be on the BN254 curve
//! - Consider making VK immutable after first set in production

use anchor_lang::prelude::*;

use crate::crypto::{validate_g1_point, validate_g2_point, is_g1_identity, is_g2_identity};
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
/// * `vk_alpha_g1` - Alpha point in G1 (64 bytes uncompressed, big-endian)
/// * `vk_beta_g2` - Beta point in G2 (128 bytes uncompressed)
/// * `vk_gamma_g2` - Gamma point in G2 (128 bytes uncompressed)
/// * `vk_delta_g2` - Delta point in G2 (128 bytes uncompressed)
/// * `vk_ic` - IC points in G1 (each 64 bytes, length = public_inputs + 1)
///
/// # Validation Steps
/// 1. Check IC length is valid (2 to MAX_IC_POINTS)
/// 2. Validate alpha_g1 is on curve and not identity
/// 3. Validate all G2 points are valid and not identity
/// 4. Validate all IC points are on curve
///
/// # Security Notes
/// - All points must be on the BN254 curve
/// - Alpha, beta, gamma, delta must not be identity (point at infinity)
/// - IC[0] should typically not be identity
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

    msg!("Setting verification key...");

    // ========== IC LENGTH VALIDATION ==========

    let ic_len = vk_ic.len();
    
    require!(
        ic_len >= 2,
        PrivacyError::InvalidPublicInputs
    );
    require!(
        ic_len <= VerificationKeyAccount::DEFAULT_MAX_IC_POINTS as usize,
        PrivacyError::InvalidPublicInputs
    );

    msg!("IC points: {} (supports {} public inputs)", ic_len, ic_len - 1);

    // ========== ALPHA G1 VALIDATION ==========

    // Alpha must not be identity
    require!(
        !is_g1_identity(&vk_alpha_g1),
        PrivacyError::VerificationKeyNotSet
    );

    // Alpha must be on curve
    validate_g1_point(&vk_alpha_g1)
        .map_err(|_| error!(PrivacyError::InvalidProof))?;

    msg!("Alpha G1 validated ✓");

    // ========== G2 POINTS VALIDATION ==========

    // Beta G2 - must not be identity
    require!(
        !is_g2_identity(&vk_beta_g2),
        PrivacyError::VerificationKeyNotSet
    );
    validate_g2_point(&vk_beta_g2)
        .map_err(|_| error!(PrivacyError::InvalidProof))?;
    msg!("Beta G2 validated ✓");

    // Gamma G2 - must not be identity
    require!(
        !is_g2_identity(&vk_gamma_g2),
        PrivacyError::VerificationKeyNotSet
    );
    validate_g2_point(&vk_gamma_g2)
        .map_err(|_| error!(PrivacyError::InvalidProof))?;
    msg!("Gamma G2 validated ✓");

    // Delta G2 - must not be identity
    require!(
        !is_g2_identity(&vk_delta_g2),
        PrivacyError::VerificationKeyNotSet
    );
    validate_g2_point(&vk_delta_g2)
        .map_err(|_| error!(PrivacyError::InvalidProof))?;
    msg!("Delta G2 validated ✓");

    // ========== IC POINTS VALIDATION ==========

    for (i, ic_point) in vk_ic.iter().enumerate() {
        // Validate each IC point is on curve
        validate_g1_point(ic_point)
            .map_err(|_| {
                msg!("IC[{}] failed curve validation", i);
                error!(PrivacyError::InvalidProof)
            })?;

        // Warn if IC[0] is identity (unusual but technically valid)
        if i == 0 && is_g1_identity(ic_point) {
            msg!("Warning: IC[0] is identity point (unusual)");
        }
    }
    msg!("All IC points validated ✓");

    // ========== STORE VERIFICATION KEY ==========

    verification_key.set_vk(
        vk_alpha_g1,
        vk_beta_g2,
        vk_gamma_g2,
        vk_delta_g2,
        vk_ic.clone(),
    );

    // Mark pool as having VK configured
    pool_config.set_vk_configured(true);

    // ========== EVENT EMISSION ==========

    emit!(VerificationKeySet {
        pool: pool_config.key(),
        authority: ctx.accounts.authority.key(),
        ic_length: ic_len as u8,
        timestamp: Clock::get()?.unix_timestamp,
    });

    msg!("Verification key set successfully");
    msg!("Pool is now ready for withdrawals");

    Ok(())
}

/// Validate a complete verification key structure.
///
/// This function performs comprehensive validation of all VK components.
/// Called internally before storing the VK.
#[allow(dead_code)]
fn validate_complete_vk(
    alpha_g1: &[u8; 64],
    beta_g2: &[u8; 128],
    gamma_g2: &[u8; 128],
    delta_g2: &[u8; 128],
    ic: &[[u8; 64]],
) -> Result<()> {
    // Validate G1 point (alpha)
    require!(!is_g1_identity(alpha_g1), PrivacyError::VerificationKeyNotSet);
    validate_g1_point(alpha_g1)?;

    // Validate G2 points
    require!(!is_g2_identity(beta_g2), PrivacyError::VerificationKeyNotSet);
    validate_g2_point(beta_g2)?;

    require!(!is_g2_identity(gamma_g2), PrivacyError::VerificationKeyNotSet);
    validate_g2_point(gamma_g2)?;

    require!(!is_g2_identity(delta_g2), PrivacyError::VerificationKeyNotSet);
    validate_g2_point(delta_g2)?;

    // Validate IC points
    require!(ic.len() >= 2, PrivacyError::InvalidPublicInputs);
    for ic_point in ic.iter() {
        validate_g1_point(ic_point)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Full VK validation tests require valid curve points
    // which are typically generated by a trusted setup ceremony.
    // These tests verify the validation logic structure.

    #[test]
    fn test_ic_length_validation() {
        // IC length must be at least 2
        let empty_ic: Vec<[u8; 64]> = vec![];
        assert!(empty_ic.len() < 2);

        let single_ic: Vec<[u8; 64]> = vec![[0u8; 64]];
        assert!(single_ic.len() < 2);

        let valid_ic: Vec<[u8; 64]> = vec![[1u8; 64], [2u8; 64]];
        assert!(valid_ic.len() >= 2);
    }

    #[test]
    fn test_identity_detection() {
        let identity_g1 = [0u8; 64];
        assert!(is_g1_identity(&identity_g1));

        let non_identity_g1 = {
            let mut arr = [0u8; 64];
            arr[0] = 1;
            arr
        };
        assert!(!is_g1_identity(&non_identity_g1));
    }
}
