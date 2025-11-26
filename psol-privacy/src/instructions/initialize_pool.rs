//! Initialize Pool Instruction
//!
//! Creates a new privacy pool for a specific SPL token mint.
//! Sets up all required accounts: PoolConfig, MerkleTree, VerificationKey, Vault.

use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

use crate::error::PrivacyError;
use crate::events::PoolInitialized;
use crate::state::{
    merkle_tree::{MerkleTree, MAX_TREE_DEPTH, MIN_ROOT_HISTORY_SIZE, MIN_TREE_DEPTH},
    pool_config::PoolConfig,
    verification_key::VerificationKeyAccount,
};

/// Accounts for initialize_pool instruction.
#[derive(Accounts)]
#[instruction(tree_depth: u8, root_history_size: u16)]
pub struct InitializePool<'info> {
    /// Pool configuration account (PDA).
    /// Seeds: ["pool", token_mint]
    #[account(
        init,
        payer = authority,
        space = PoolConfig::LEN,
        seeds = [b"pool", token_mint.key().as_ref()],
        bump
    )]
    pub pool_config: Account<'info, PoolConfig>,

    /// Merkle tree state account (PDA).
    /// Seeds: ["merkle_tree", pool_config]
    #[account(
        init,
        payer = authority,
        space = MerkleTree::space(tree_depth, root_history_size),
        seeds = [b"merkle_tree", pool_config.key().as_ref()],
        bump
    )]
    pub merkle_tree: Account<'info, MerkleTree>,

    /// Verification key account (PDA).
    /// Seeds: ["verification_key", pool_config]
    #[account(
        init,
        payer = authority,
        space = VerificationKeyAccount::space(VerificationKeyAccount::DEFAULT_MAX_IC_POINTS),
        seeds = [b"verification_key", pool_config.key().as_ref()],
        bump
    )]
    pub verification_key: Account<'info, VerificationKeyAccount>,

    /// Token vault (PDA) - holds deposited tokens.
    /// Seeds: ["vault", pool_config]
    /// Authority: pool_config PDA
    #[account(
        init,
        payer = authority,
        token::mint = token_mint,
        token::authority = pool_config,
        seeds = [b"vault", pool_config.key().as_ref()],
        bump
    )]
    pub vault: Account<'info, TokenAccount>,

    /// SPL token mint for this pool.
    pub token_mint: Account<'info, Mint>,

    /// Pool authority (admin) - pays for account creation.
    #[account(mut)]
    pub authority: Signer<'info>,

    /// System program for account creation.
    pub system_program: Program<'info, System>,

    /// Token program for vault creation.
    pub token_program: Program<'info, Token>,
}

/// Handler for initialize_pool instruction.
pub fn handler(
    ctx: Context<InitializePool>,
    tree_depth: u8,
    root_history_size: u16,
) -> Result<()> {
    // Validate parameters
    require!(
        tree_depth >= MIN_TREE_DEPTH && tree_depth <= MAX_TREE_DEPTH,
        PrivacyError::InvalidTreeDepth
    );
    require!(
        root_history_size >= MIN_ROOT_HISTORY_SIZE,
        PrivacyError::InvalidRootHistorySize
    );

    let pool_config = &mut ctx.accounts.pool_config;
    let merkle_tree = &mut ctx.accounts.merkle_tree;
    let verification_key = &mut ctx.accounts.verification_key;

    // Initialize pool configuration
    pool_config.initialize(
        ctx.accounts.authority.key(),
        ctx.accounts.token_mint.key(),
        ctx.accounts.vault.key(),
        merkle_tree.key(),
        verification_key.key(),
        tree_depth,
        ctx.bumps.pool_config,
    );

    // Initialize Merkle tree
    merkle_tree.initialize(pool_config.key(), tree_depth, root_history_size)?;

    // Initialize verification key account (empty, to be configured later)
    verification_key.initialize(pool_config.key(), ctx.bumps.verification_key);

    // Emit initialization event
    emit!(PoolInitialized {
        pool: pool_config.key(),
        authority: ctx.accounts.authority.key(),
        token_mint: ctx.accounts.token_mint.key(),
        tree_depth,
        root_history_size,
        timestamp: Clock::get()?.unix_timestamp,
    });

    msg!("Privacy pool initialized successfully");
    msg!("Pool: {}", pool_config.key());
    msg!("Merkle tree depth: {}", tree_depth);
    msg!("Root history size: {}", root_history_size);
    msg!("Token mint: {}", ctx.accounts.token_mint.key());

    Ok(())
}
