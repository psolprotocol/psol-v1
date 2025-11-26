use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};
use crate::{error::PrivacyError, events, state::*};

#[derive(Accounts)]
pub struct Deposit<'info> {
    /// Pool configuration
    #[account(
        mut,
        seeds = [b"pool", pool_config.token_mint.as_ref()],
        bump = pool_config.bump,
    )]
    pub pool_config: Account<'info, PoolConfig>,

    /// Merkle tree state
    #[account(
        mut,
        seeds = [b"merkle_tree", pool_config.key().as_ref()],
        bump,
        constraint = merkle_tree.pool == pool_config.key() @ PrivacyError::Unauthorized,
    )]
    pub merkle_tree: Account<'info, MerkleTree>,

    /// Token vault
    #[account(
        mut,
        seeds = [b"vault", pool_config.key().as_ref()],
        bump,
        constraint = vault.mint == pool_config.token_mint @ PrivacyError::InvalidAmount,
    )]
    pub vault: Account<'info, TokenAccount>,

    /// User's source token account
    #[account(
        mut,
        constraint = user_token_account.mint == pool_config.token_mint @ PrivacyError::InvalidAmount,
    )]
    pub user_token_account: Account<'info, TokenAccount>,

    /// User making the deposit
    #[account(mut)]
    pub user: Signer<'info>,

    /// Token program
    pub token_program: Program<'info, Token>,
}

pub fn handler(ctx: Context<Deposit>, amount: u64, commitment: [u8; 32]) -> Result<()> {
    let pool_config = &mut ctx.accounts.pool_config;
    let merkle_tree = &mut ctx.accounts.merkle_tree;

    pool_config.require_not_paused()?;

    require!(amount > 0, PrivacyError::InvalidAmount);

    require!(commitment != [0u8; 32], PrivacyError::InvalidCommitment);

    let cpi_accounts = Transfer {
        from: ctx.accounts.user_token_account.to_account_info(),
        to: ctx.accounts.vault.to_account_info(),
        authority: ctx.accounts.user.to_account_info(),
    };
    let cpi_ctx = CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);
    token::transfer(cpi_ctx, amount)?;

    let leaf_index = merkle_tree.insert_leaf(commitment)?;

    pool_config.update_root(merkle_tree.get_current_root());

    pool_config.increment_deposits()?;

    emit!(events::Deposit {
        pool: pool_config.key(),
        commitment,
        leaf_index,
        merkle_root: merkle_tree.get_current_root(),
        amount,
        timestamp: Clock::get()?.unix_timestamp,
    });

    msg!("Deposit successful");
    msg!("Commitment: {:?}", commitment);
    msg!("Leaf index: {}", leaf_index);
    msg!("New root: {:?}", merkle_tree.get_current_root());

    Ok(())
}
