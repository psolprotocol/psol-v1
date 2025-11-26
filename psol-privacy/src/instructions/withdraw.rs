//! Withdraw Instruction
//!
//! Withdraws tokens from the privacy pool using a zero-knowledge proof.
//!
//! # PHASE 2 STATUS: FAIL-CLOSED
//!
//! This instruction currently ALWAYS FAILS because Groth16 verification
//! is not yet implemented. This is intentional security behavior.
//!
//! # Architecture
//! 1. User generates ZK proof off-chain proving:
//!    - Knowledge of (secret, nullifier_preimage) for a commitment in the tree
//!    - The commitment was computed correctly with the claimed amount
//!    - The nullifier_hash is correctly derived
//!
//! 2. User (or relayer) submits withdrawal transaction with:
//!    - Proof data (256 bytes)
//!    - Public inputs (merkle_root, nullifier_hash, recipient, amount, relayer, fee)
//!
//! 3. On-chain verification:
//!    - Check merkle_root is in recent history
//!    - Check nullifier not already spent (via PDA existence)
//!    - Verify ZK proof (CURRENTLY FAILS - Phase 3)
//!    - Create SpentNullifier PDA to mark as spent
//!    - Transfer tokens to recipient
//!
//! # Relayer Model
//! Relayers submit transactions on behalf of users for privacy:
//! - User's address never appears on-chain
//! - Relayer receives fee from withdrawal amount
//! - Relayer pays transaction fees

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
    /// Pool configuration.
    #[account(
        mut,
        seeds = [b"pool", pool_config.token_mint.as_ref()],
        bump = pool_config.bump,
    )]
    pub pool_config: Account<'info, PoolConfig>,

    /// Merkle tree state (for root verification).
    #[account(
        seeds = [b"merkle_tree", pool_config.key().as_ref()],
        bump,
        constraint = merkle_tree.pool == pool_config.key() @ PrivacyError::Unauthorized,
    )]
    pub merkle_tree: Account<'info, MerkleTree>,

    /// Verification key for proof verification.
    #[account(
        seeds = [b"verification_key", pool_config.key().as_ref()],
        bump = verification_key.bump,
        constraint = verification_key.pool == pool_config.key() @ PrivacyError::Unauthorized,
        constraint = verification_key.is_initialized @ PrivacyError::VerificationKeyNotSet,
    )]
    pub verification_key: Account<'info, VerificationKeyAccount>,

    /// Spent nullifier PDA - created by this instruction.
    /// If account already exists, the nullifier was already spent.
    /// Seeds: ["nullifier", pool_config, nullifier_hash]
    #[account(
        init,
        payer = withdrawer,
        space = SpentNullifier::LEN,
        seeds = [b"nullifier", pool_config.key().as_ref(), nullifier_hash.as_ref()],
        bump
    )]
    pub spent_nullifier: Account<'info, SpentNullifier>,

    /// Token vault (source of withdrawal).
    #[account(
        mut,
        seeds = [b"vault", pool_config.key().as_ref()],
        bump,
        constraint = vault.mint == pool_config.token_mint @ PrivacyError::InvalidMint,
        constraint = vault.owner == pool_config.key() @ PrivacyError::Unauthorized,
    )]
    pub vault: Account<'info, TokenAccount>,

    /// Recipient's token account (receives amount - fee).
    #[account(
        mut,
        constraint = recipient_token_account.mint == pool_config.token_mint @ PrivacyError::InvalidMint,
        constraint = recipient_token_account.owner == recipient @ PrivacyError::RecipientMismatch,
    )]
    pub recipient_token_account: Account<'info, TokenAccount>,

    /// Relayer's token account (receives fee).
    /// Can be same as recipient_token_account if self-relaying.
    #[account(
        mut,
        constraint = relayer_token_account.mint == pool_config.token_mint @ PrivacyError::InvalidMint,
        constraint = relayer_token_account.owner == relayer @ PrivacyError::RecipientMismatch,
    )]
    pub relayer_token_account: Account<'info, TokenAccount>,

    /// Withdrawer (relayer) - pays for transaction and nullifier account.
    #[account(mut)]
    pub withdrawer: Signer<'info>,

    /// Token program.
    pub token_program: Program<'info, Token>,

    /// System program (for nullifier account creation).
    pub system_program: Program<'info, System>,
}

/// Handler for withdraw instruction.
///
/// # PHASE 2 STATUS: ALWAYS FAILS
///
/// This function performs all validation but will fail at proof verification
/// because Groth16 is not yet implemented.
///
/// # Arguments
/// * `proof_data` - Serialized Groth16 proof (256 bytes)
/// * `merkle_root` - Root to prove membership against
/// * `nullifier_hash` - Hash of nullifier (prevents double-spend)
/// * `recipient` - Address to receive withdrawn tokens
/// * `amount` - Amount to withdraw (before fee)
/// * `relayer` - Relayer address (receives fee)
/// * `relayer_fee` - Fee paid to relayer
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

    // 1. Check pool is not paused
    pool_config.require_not_paused()?;

    // 2. Check verification key is configured
    pool_config.require_vk_configured()?;

    // 3. Validate amount
    require!(amount > 0, PrivacyError::InvalidAmount);

    // 4. Validate fee doesn't exceed amount
    require!(relayer_fee <= amount, PrivacyError::RelayerFeeExceedsAmount);

    // 5. Check vault has sufficient balance
    require!(
        ctx.accounts.vault.amount >= amount,
        PrivacyError::InsufficientBalance
    );

    // 6. Verify merkle root is in recent history
    require!(
        merkle_tree.is_known_root(&merkle_root),
        PrivacyError::InvalidMerkleRoot
    );

    // 7. Nullifier uniqueness is enforced by PDA creation
    // If spent_nullifier account already exists, transaction will fail
    // with "already initialized" error

    // ========== ZK PROOF VERIFICATION ==========

    // Construct public inputs
    let public_inputs = ZkPublicInputs::new(
        merkle_root,
        nullifier_hash,
        recipient,
        amount,
        relayer,
        relayer_fee,
    );

    // Validate public inputs structure
    public_inputs.validate()?;

    // Get verification key
    let vk: VerificationKey = verification_key.into();

    // Verify proof
    // PHASE 2: This ALWAYS returns Err(CryptoNotImplemented)
    let proof_valid = verify_groth16_proof(&proof_data, &vk, &public_inputs)?;

    // This line is technically unreachable in Phase 2
    // (verify_groth16_proof always returns Err)
    require!(proof_valid, PrivacyError::InvalidProof);

    // ========== STATE UPDATES (only reached if proof valid) ==========

    // Initialize spent nullifier record
    spent_nullifier.initialize(
        pool_config.key(),
        nullifier_hash,
        Clock::get()?.unix_timestamp,
        Clock::get()?.slot,
        ctx.bumps.spent_nullifier,
    );

    // Calculate net amount after fee
    let net_amount = amount
        .checked_sub(relayer_fee)
        .ok_or(error!(PrivacyError::ArithmeticOverflow))?;

    // Prepare PDA signer seeds for vault transfer
    let pool_seeds = &[
        b"pool".as_ref(),
        pool_config.token_mint.as_ref(),
        &[pool_config.bump],
    ];
    let signer_seeds = &[&pool_seeds[..]];

    // Transfer net amount to recipient
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

    // Transfer fee to relayer (if fee > 0)
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

    // Update pool statistics
    pool_config.increment_withdrawals()?;

    // Emit withdrawal event
    emit!(WithdrawEvent {
        pool: pool_config.key(),
        nullifier_hash,
        recipient,
        amount: net_amount,
        relayer,
        relayer_fee,
        timestamp: Clock::get()?.unix_timestamp,
    });

    msg!("Withdrawal successful");
    msg!("Recipient: {}", recipient);
    msg!("Net amount: {}", net_amount);
    msg!("Relayer fee: {}", relayer_fee);

    Ok(())
}
