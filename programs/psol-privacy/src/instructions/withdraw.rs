//! Withdraw Instruction
//!
//! Withdraws tokens from the privacy pool using a Groth16 zero-knowledge proof.
//!
//! # Phase 3 Implementation
//!
<<<<<<< HEAD
//! This instruction currently ALWAYS FAILS because Groth16 verification
//! is not yet implemented. This is intentional security behavior.
=======
//! Full Groth16 verification is now implemented using Solana's alt_bn128 precompiles.
//! In production builds, all proofs are cryptographically verified.
//!
//! # Architecture
//! 1. User generates ZK proof off-chain proving:
//!    - Knowledge of (secret, nullifier_preimage) for a commitment in the tree
//!    - The commitment was computed correctly with the claimed amount
//!    - The nullifier_hash is correctly derived
//!
//! 2. User (or relayer) submits withdrawal transaction with:
//!    - Proof data (256 bytes: A || B || C curve points)
//!    - Public inputs (merkle_root, nullifier_hash, recipient, amount, relayer, fee)
//!
//! 3. On-chain verification:
//!    - Check merkle_root is in recent history
//!    - Check nullifier not already spent (via PDA existence)
//!    - Verify Groth16 proof using pairing check
//!    - Create SpentNullifier PDA to mark as spent
//!    - Transfer tokens to recipient
//!
//! # Relayer Model
//! Relayers submit transactions on behalf of users for privacy:
//! - User's address never appears on-chain
//! - Relayer receives fee from withdrawal amount
//! - Relayer pays transaction fees
//!
//! # Dev Mode
//! When built with `--features dev-mode`, proof verification is bypassed.
//! This is ONLY for testing - NEVER use in production!
>>>>>>> psol-phase-2,5

use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

use crate::crypto::{verify_groth16_proof, ZkPublicInputs};
use crate::error::PrivacyError;
use crate::events::WithdrawEvent;
use crate::state::{
    verification_key::VerificationKey, MerkleTree, PoolConfig, SpentNullifier,
    VerificationKeyAccount,
};

/// Accounts for withdraw instruction.
#[derive(Accounts)]
#[instruction(
    proof_data: Vec<u8>,
    merkle_root: [u8; 32],
    nullifier_hash: [u8; 32],
    recipient: Pubkey,
    amount: u64,
    relayer: Pubkey,
    relayer_fee: u64,
)]
pub struct Withdraw<'info> {
    #[account(
        mut,
        seeds = [b"pool", pool_config.token_mint.as_ref()],
        bump = pool_config.bump,
    )]
    pub pool_config: Account<'info, PoolConfig>,

    #[account(
        seeds = [b"merkle_tree", pool_config.key().as_ref()],
        bump,
        constraint = merkle_tree.pool == pool_config.key() @ PrivacyError::Unauthorized,
    )]
    pub merkle_tree: Account<'info, MerkleTree>,

    #[account(
        seeds = [b"verification_key", pool_config.key().as_ref()],
        bump = verification_key.bump,
        constraint = verification_key.pool == pool_config.key() @ PrivacyError::Unauthorized,
        constraint = verification_key.is_initialized @ PrivacyError::VerificationKeyNotSet,
    )]
    pub verification_key: Account<'info, VerificationKeyAccount>,

    #[account(
        init,
        payer = withdrawer,
        space = SpentNullifier::LEN,
        seeds = [b"nullifier", pool_config.key().as_ref(), nullifier_hash.as_ref()],
        bump
    )]
    pub spent_nullifier: Account<'info, SpentNullifier>,

    #[account(
        mut,
        seeds = [b"vault", pool_config.key().as_ref()],
        bump,
        constraint = vault.mint == pool_config.token_mint @ PrivacyError::InvalidMint,
        constraint = vault.owner == pool_config.key() @ PrivacyError::Unauthorized,
    )]
    pub vault: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = recipient_token_account.mint == pool_config.token_mint @ PrivacyError::InvalidMint,
        constraint = recipient_token_account.owner == recipient @ PrivacyError::RecipientMismatch,
    )]
    pub recipient_token_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = relayer_token_account.mint == pool_config.token_mint @ PrivacyError::InvalidMint,
        constraint = relayer_token_account.owner == relayer @ PrivacyError::RecipientMismatch,
    )]
    pub relayer_token_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub withdrawer: Signer<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

<<<<<<< HEAD
=======
/// Handler for withdraw instruction.
///
/// Withdraws tokens from the privacy pool using a Groth16 ZK proof.
///
/// # Verification Flow
/// 1. Validate pool is not paused and VK is configured
/// 2. Validate amount, fee, and vault balance
/// 3. Verify merkle_root is in recent history
/// 4. Verify Groth16 proof (or bypass in dev-mode)
/// 5. Create SpentNullifier PDA (marks as spent)
/// 6. Transfer tokens to recipient and relayer
///
/// # Arguments
/// * `proof_data` - Serialized Groth16 proof (256 bytes: A || B || C)
/// * `merkle_root` - Root to prove membership against
/// * `nullifier_hash` - Hash of nullifier (prevents double-spend)
/// * `recipient` - Address to receive withdrawn tokens
/// * `amount` - Amount to withdraw (before fee)
/// * `relayer` - Relayer address (receives fee)
/// * `relayer_fee` - Fee paid to relayer
///
/// # Security
/// - Proof verification is cryptographically enforced in production
/// - In dev-mode builds, proof verification is bypassed for testing
>>>>>>> psol-phase-2,5
#[allow(clippy::too_many_arguments)]
pub fn handler(
    ctx: Context<Withdraw>,
    proof_data: Vec<u8>,
    merkle_root: [u8; 32],
    nullifier_hash: [u8; 32],
    recipient: Pubkey,
    amount: u64,
    relayer: Pubkey,
    relayer_fee: u64,
) -> Result<()> {
    let pool_config = &mut ctx.accounts.pool_config;
    let merkle_tree = &ctx.accounts.merkle_tree;
    let verification_key = &ctx.accounts.verification_key;
    let spent_nullifier = &mut ctx.accounts.spent_nullifier;

    // ========== VALIDATION CHECKS ==========

    pool_config.require_not_paused()?;
    pool_config.require_vk_configured()?;
    require!(amount > 0, PrivacyError::InvalidAmount);
    require!(relayer_fee <= amount, PrivacyError::RelayerFeeExceedsAmount);
    require!(
        ctx.accounts.vault.amount >= amount,
        PrivacyError::InsufficientBalance
    );

    require!(
        merkle_tree.is_known_root(&merkle_root),
        PrivacyError::InvalidMerkleRoot
    );

    // ========== ZK PROOF VERIFICATION ==========

    let public_inputs = ZkPublicInputs::new(
        merkle_root,
        nullifier_hash,
        recipient,
        amount,
        relayer,
        relayer_fee,
    );

    public_inputs.validate()?;

    // FIXED CODE: Load & parse VK from account bytes
    let vk: VerificationKey = VerificationKey::from(verification_key.as_ref());

<<<<<<< HEAD
    // Phase 2: ALWAYS returns Err
    let proof_valid = verify_groth16_proof(&proof_data, &vk, &public_inputs)?;
=======
    // Verify Groth16 proof
    // In production: performs full pairing verification
    // In dev-mode: bypasses proof check (for testing only!)
    let proof_valid = verify_groth16_proof(&proof_data, &vk, &public_inputs)?;

    // Reject if proof is invalid
>>>>>>> psol-phase-2,5
    require!(proof_valid, PrivacyError::InvalidProof);

    // ========== STATE UPDATES ==========

    spent_nullifier.initialize(
        pool_config.key(),
        nullifier_hash,
        Clock::get()?.unix_timestamp,
        Clock::get()?.slot,
        ctx.bumps.spent_nullifier,
    );

    let net_amount = amount
        .checked_sub(relayer_fee)
        .ok_or(error!(PrivacyError::ArithmeticOverflow))?;

    let pool_seeds = &[
        b"pool".as_ref(),
        pool_config.token_mint.as_ref(),
        &[pool_config.bump],
    ];
    let signer_seeds = &[&pool_seeds[..]];

    if net_amount > 0 {
        let cpi_accounts = Transfer {
            from: ctx.accounts.vault.to_account_info(),
            to: ctx.accounts.recipient_token_account.to_account_info(),
            authority: pool_config.to_account_info(),
        };
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
            signer_seeds,
        );
        token::transfer(cpi_ctx, net_amount)?;
    }

    if relayer_fee > 0 {
        let cpi_accounts = Transfer {
            from: ctx.accounts.vault.to_account_info(),
            to: ctx.accounts.relayer_token_account.to_account_info(),
            authority: pool_config.to_account_info(),
        };
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
            signer_seeds,
        );
        token::transfer(cpi_ctx, relayer_fee)?;
    }

    pool_config.increment_withdrawals()?;

    emit!(WithdrawEvent {
        pool: pool_config.key(),
        nullifier_hash,
        recipient,
        amount: net_amount,
        relayer,
        relayer_fee,
        timestamp: Clock::get()?.unix_timestamp,
    });

    msg!("Withdrawal successful (Phase 2 fail-closed mode)");

    Ok(())
}
