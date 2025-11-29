//! Initialize Pool Instruction - Phase 4 (Stack Optimized)

use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

use crate::error::PrivacyError;
use crate::events::PoolInitialized;
use crate::state::{MerkleTree, PoolConfig, VerificationKeyAccount};

pub const MIN_TREE_DEPTH: u8 = 4;
pub const MAX_TREE_DEPTH: u8 = 24;
pub const MIN_ROOT_HISTORY: u16 = 30;
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

    pub token_mint: Box<Account<'info, Mint>>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
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

    let keys = (
        ctx.accounts.pool_config.key(),
        ctx.accounts.vault.key(),
        ctx.accounts.merkle_tree.key(),
        ctx.accounts.verification_key.key(),
        ctx.accounts.authority.key(),
        ctx.accounts.token_mint.key(),
    );

    ctx.accounts.pool_config.initialize(
        keys.4, keys.5, keys.1, keys.2, keys.3, tree_depth, ctx.bumps.pool_config,
    );

    ctx.accounts.merkle_tree.initialize(keys.0, tree_depth, root_history_size)?;
    ctx.accounts.verification_key.initialize(keys.0, ctx.bumps.verification_key);

    emit!(PoolInitialized {
        pool: keys.0,
        authority: keys.4,
        token_mint: keys.5,
        tree_depth,
        root_history_size,
        timestamp: Clock::get()?.unix_timestamp,
    });

    msg!("Pool initialized: {}", keys.0);
    Ok(())
}
