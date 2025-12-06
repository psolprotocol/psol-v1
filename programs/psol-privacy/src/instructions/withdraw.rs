//! Withdraw Instruction - Devnet Alpha Hardened

use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

use crate::crypto::{verify_groth16_proof, ZkPublicInputs};
use crate::error::PrivacyError;
use crate::events::WithdrawEvent;
use crate::state::{
    verification_key::VerificationKey, MerkleTree, PoolConfig, SpentNullifier,
    VerificationKeyAccount,
};

pub const MIN_WITHDRAWAL_AMOUNT: u64 = 1;
pub const MAX_RELAYER_FEE_BPS: u64 = 1000; // 10%

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
        payer = payer,
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
        constraint = relayer_token_account.owner == relayer @ PrivacyError::Unauthorized,
    )]
    pub relayer_token_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

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

    // Basic state guards
    pool_config.require_not_paused()?;
    pool_config.require_vk_configured()?;

    // Amount and fee sanity
    require!(amount >= MIN_WITHDRAWAL_AMOUNT, PrivacyError::InvalidAmount);
    require!(
        relayer_fee <= amount,
        PrivacyError::RelayerFeeExceedsAmount
    );

    // Enforce maximum relayer fee (10% = 1000 basis points)
    let max_fee = amount
        .checked_mul(MAX_RELAYER_FEE_BPS)
        .and_then(|v| v.checked_div(10_000))
        .ok_or(error!(PrivacyError::ArithmeticOverflow))?;
    require!(
        relayer_fee <= max_fee,
        PrivacyError::RelayerFeeExceedsAmount
    );

    // Vault and tree checks
    require!(
        ctx.accounts.vault.amount >= amount,
        PrivacyError::InsufficientBalance
    );
    require!(
        merkle_tree.is_known_root(&merkle_root),
        PrivacyError::InvalidMerkleRoot
    );
    require!(
        nullifier_hash != [0u8; 32],
        PrivacyError::InvalidNullifier
    );

    // Public inputs and ZK verification
    let public_inputs =
        ZkPublicInputs::new(merkle_root, nullifier_hash, recipient, amount, relayer, relayer_fee);
    public_inputs.validate()?;

    let vk: VerificationKey = VerificationKey::from(verification_key.as_ref());
    let proof_valid = verify_groth16_proof(&proof_data, &vk, &public_inputs)?;
    require!(proof_valid, PrivacyError::InvalidProof);

    // Nullifier marking
    let clock = Clock::get()?;
    spent_nullifier.initialize(
        pool_config.key(),
        nullifier_hash,
        clock.unix_timestamp,
        clock.slot,
        ctx.bumps.spent_nullifier,
    );

    // Compute net amount after relayer fee
    let net_amount = amount
        .checked_sub(relayer_fee)
        .ok_or(error!(PrivacyError::ArithmeticOverflow))?;

    // PDA signer seeds
    let pool_seeds = &[
        b"pool".as_ref(),
        pool_config.token_mint.as_ref(),
        &[pool_config.bump],
    ];
    let signer_seeds = &[&pool_seeds[..]];

    // Transfer to recipient
    if net_amount > 0 {
        let cpi_accounts = Transfer {
            from: ctx.accounts.vault.to_account_info(),
            to: ctx
                .accounts
                .recipient_token_account
                .to_account_info(),
            authority: pool_config.to_account_info(),
        };
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
            signer_seeds,
        );
        token::transfer(cpi_ctx, net_amount)?;
    }

    // Transfer relayer fee
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

    // Update pool stats (gross amount for accounting)
    pool_config.record_withdrawal(amount)?;

    // Emit event (net amount to user is usually what consumers care about)
    emit!(WithdrawEvent {
        pool: pool_config.key(),
        nullifier_hash,
        recipient,
        amount: net_amount,
        relayer,
        relayer_fee,
        timestamp: clock.unix_timestamp,
    });

    msg!("Withdrawal successful");
    Ok(())
}
