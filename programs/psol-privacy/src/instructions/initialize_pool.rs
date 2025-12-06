//! Initialize Pool Instruction - Phase 4 (Stack Optimized)

use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

use crate::error::PrivacyError;
use crate::events::PoolInitialized;
use crate::state::{MerkleTree, PoolConfig, VerificationKeyAccount};

pub const MIN_TREE_DEPTH: u8 = 4;
pub const MAX_TREE_DEPTH: u8 = 24;
pub const MIN_ROOT_HISTORY: u16 = 200;
pub const MAX_ROOT_HISTORY: u16 = 1000;

#[derive(Accounts)]
#[instruction(tree_depth: u8, root_history_size: u16)]
pub struct InitializePool<'info> {
    #[account(
        init,
        payer = authority,
        space = PoolConfig::LEN,
        seeds = [b"pool", token_mint.key().as_ref()],
        bump
    )]
    pub pool_config: Box<Account<'info, PoolConfig>>,

    #[account(
        init,
        payer = authority,
        space = MerkleTree::space(tree_depth, root_history_size),
        seeds = [b"merkle_tree", pool_config.key().as_ref()],
        bump
    )]
    pub merkle_tree: Box<Account<'info, MerkleTree>>,

    #[account(
        init,
        payer = authority,
        space = VerificationKeyAccount::space(VerificationKeyAccount::DEFAULT_MAX_IC_POINTS),
        seeds = [b"verification_key", pool_config.key().as_ref()],
        bump
    )]
    pub verification_key: Box<Account<'info, VerificationKeyAccount>>,

    #[account(
        init,
        payer = authority,
        token::mint = token_mint,
        token::authority = pool_config,
        seeds = [b"vault", pool_config.key().as_ref()],
        bump
    )]
    pub vault: Box<Account<'info, TokenAccount>>,

    /// CHECK: Token mint validated by Anchor's token::mint constraint
    pub token_mint: UncheckedAccount<'info>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn handler(
    ctx: Context<InitializePool>,
    tree_depth: u8,
    root_history_size: u16,
) -> Result<()> {
    require!(
        tree_depth >= MIN_TREE_DEPTH && tree_depth <= MAX_TREE_DEPTH,
        PrivacyError::InvalidTreeDepth
    );
    require!(
        root_history_size >= MIN_ROOT_HISTORY && root_history_size <= MAX_ROOT_HISTORY,
        PrivacyError::InvalidRootHistorySize
    );

    msg!("Initializing privacy pool...");

    // Store keys in local scope to minimize stack
    let pool_key = ctx.accounts.pool_config.key();
    let vault_key = ctx.accounts.vault.key();
    let tree_key = ctx.accounts.merkle_tree.key();
    let vk_key = ctx.accounts.verification_key.key();
    let auth_key = ctx.accounts.authority.key();
    let mint_key = ctx.accounts.token_mint.key();
    let bump = ctx.bumps.pool_config;

    ctx.accounts.pool_config.initialize(
        auth_key, mint_key, vault_key, tree_key, vk_key, tree_depth, bump,
    );

    ctx.accounts.merkle_tree.initialize(pool_key, tree_depth, root_history_size)?;
    ctx.accounts.verification_key.initialize(pool_key, ctx.bumps.verification_key);

    emit!(PoolInitialized {
        pool: pool_key,
        authority: auth_key,
        token_mint: mint_key,
        tree_depth,
        root_history_size,
        timestamp: Clock::get()?.unix_timestamp,
    });

    msg!("Pool initialized: {}", pool_key);
    Ok(())
}
