use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};
use crate::{error::PrivacyError, events, state::*};

/// Nullifier capacity constant - MUST match space calculation
/// Small for Playground, later you can increase for real deployment
const NULLIFIER_CAPACITY: u32 = 16;

#[derive(Accounts)]
#[instruction(tree_depth: u8, root_history_size: u16)]
pub struct InitializePool<'info> {
    /// Pool configuration account
    #[account(
        init,
        payer = authority,
        space = PoolConfig::LEN,
        seeds = [b"pool", token_mint.key().as_ref()],
        bump
    )]
    pub pool_config: Account<'info, PoolConfig>,

    /// Merkle tree state account
    #[account(
        init,
        payer = authority,
        space = MerkleTree::space(tree_depth, root_history_size),
        seeds = [b"merkle_tree", pool_config.key().as_ref()],
        bump
    )]
    pub merkle_tree: Account<'info, MerkleTree>,

    /// Nullifier set account
    #[account(
        init,
        payer = authority,
        space = crate::crypto::NullifierSet::space(NULLIFIER_CAPACITY),
        seeds = [b"nullifiers", pool_config.key().as_ref()],
        bump
    )]
    pub nullifier_set: Account<'info, crate::crypto::NullifierSet>,

    /// Token vault (PDA holding deposited tokens)
    #[account(
        init,
        payer = authority,
        token::mint = token_mint,
        token::authority = pool_config,
        seeds = [b"vault", pool_config.key().as_ref()],
        bump
    )]
    pub vault: Account<'info, TokenAccount>,

    /// Token mint for the pool
    pub token_mint: Account<'info, Mint>,

    /// Pool authority (admin)
    #[account(mut)]
    pub authority: Signer<'info>,

    /// System program
    pub system_program: Program<'info, System>,

    /// Token program
    pub token_program: Program<'info, Token>,

    /// Rent sysvar
    pub rent: Sysvar<'info, Rent>,
}

pub fn handler(ctx: Context<InitializePool>, tree_depth: u8, root_history_size: u16) -> Result<()> {
    // Relaxed ranges for Playground
    require!(
        tree_depth >= 4 && tree_depth <= 32,
        PrivacyError::InvalidTreeDepth
    );
    require!(root_history_size >= 5, PrivacyError::RootHistoryFull);

    let pool_config = &mut ctx.accounts.pool_config;
    let merkle_tree = &mut ctx.accounts.merkle_tree;
    let nullifier_set = &mut ctx.accounts.nullifier_set;

    pool_config.initialize(
        ctx.accounts.authority.key(),
        ctx.accounts.token_mint.key(),
        ctx.accounts.vault.key(),
        tree_depth,
        ctx.bumps.pool_config,
    )?;

    merkle_tree.initialize(pool_config.key(), tree_depth, root_history_size)?;

    pool_config.update_root(merkle_tree.get_current_root());

    // IMPORTANT: pass NULLIFIER_CAPACITY here
    nullifier_set.initialize(pool_config.key(), NULLIFIER_CAPACITY);

    emit!(events::PoolInitialized {
        pool: pool_config.key(),
        authority: ctx.accounts.authority.key(),
        token_mint: ctx.accounts.token_mint.key(),
        tree_depth,
        timestamp: Clock::get()?.unix_timestamp,
    });

    msg!("Privacy pool initialized successfully");
    msg!("Pool: {}", pool_config.key());
    msg!("Tree depth: {}", tree_depth);

    Ok(())
}
