//! Initialize Pool Instruction

use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

use crate::error::PrivacyError;
use crate::events::PoolInitialized;
use crate::state::{
    merkle_tree::{MerkleTree, MAX_TREE_DEPTH, MIN_ROOT_HISTORY_SIZE, MIN_TREE_DEPTH},
    pool_config::PoolConfig,
    verification_key::VerificationKeyAccount,
};

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

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
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
        root_history_size >= MIN_ROOT_HISTORY_SIZE,
        PrivacyError::InvalidRootHistorySize
    );

    let pool_config = &mut ctx.accounts.pool_config;
    let merkle_tree = &mut ctx.accounts.merkle_tree;
    let verification_key = &mut ctx.accounts.verification_key;

    pool_config.initialize(
        ctx.accounts.authority.key(),
        ctx.accounts.token_mint.key(),
        ctx.accounts.vault.key(),
        merkle_tree.key(),
        verification_key.key(),
        tree_depth,
        ctx.bumps.pool_config,
    );

    merkle_tree.initialize(pool_config.key(), tree_depth, root_history_size)?;
    verification_key.initialize(pool_config.key(), ctx.bumps.verification_key);

    emit!(PoolInitialized {
        pool: pool_config.key(),
        authority: ctx.accounts.authority.key(),
        token_mint: ctx.accounts.token_mint.key(),
        tree_depth,
        root_history_size,
        timestamp: Clock::get()?.unix_timestamp,
    });

    msg!("Privacy pool initialized");
    Ok(())
}
