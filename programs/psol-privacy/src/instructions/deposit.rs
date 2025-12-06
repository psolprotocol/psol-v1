//! Deposit Instruction - Phase 4 Hardened

use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

use crate::error::PrivacyError;
use crate::events::DepositEvent;
use crate::state::{MerkleTree, PoolConfig};

pub const MAX_DEPOSIT_AMOUNT: u64 = 1_000_000_000_000_000;

#[derive(Accounts)]
#[instruction(amount: u64, commitment: [u8; 32])]
pub struct Deposit<'info> {
    #[account(
        mut,
        seeds = [b"pool", pool_config.token_mint.as_ref()],
        bump = pool_config.bump,
    )]
    pub pool_config: Account<'info, PoolConfig>,

    #[account(
        mut,
        seeds = [b"merkle_tree", pool_config.key().as_ref()],
        bump,
        constraint = merkle_tree.pool == pool_config.key() @ PrivacyError::Unauthorized,
    )]
    pub merkle_tree: Account<'info, MerkleTree>,

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
        constraint = depositor_token_account.mint == pool_config.token_mint @ PrivacyError::InvalidMint,
        constraint = depositor_token_account.owner == depositor.key() @ PrivacyError::Unauthorized,
    )]
    pub depositor_token_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub depositor: Signer<'info>,

    pub token_program: Program<'info, Token>,
}

pub fn handler(ctx: Context<Deposit>, amount: u64, commitment: [u8; 32]) -> Result<()> {
    let pool_config = &mut ctx.accounts.pool_config;
    let merkle_tree = &mut ctx.accounts.merkle_tree;

    pool_config.require_not_paused()?;
    pool_config.require_vk_configured()?;

    require!(amount > 0, PrivacyError::InvalidAmount);
    require!(amount <= MAX_DEPOSIT_AMOUNT, PrivacyError::LimitExceeded);
    require!(commitment != [0u8; 32], PrivacyError::InvalidCommitment);
    require!(!merkle_tree.is_full(), PrivacyError::MerkleTreeFull);
    require!(
        ctx.accounts.depositor_token_account.amount >= amount,
        PrivacyError::InsufficientBalance
    );

    msg!("Processing deposit: {} tokens", amount);

    let cpi_accounts = Transfer {
        from: ctx.accounts.depositor_token_account.to_account_info(),
        to: ctx.accounts.vault.to_account_info(),
        authority: ctx.accounts.depositor.to_account_info(),
    };
    let cpi_ctx = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        cpi_accounts,
    );
    token::transfer(cpi_ctx, amount)?;

    let leaf_index = merkle_tree.insert_leaf(commitment)?;
    
    msg!("Commitment inserted at leaf index: {}", leaf_index);

    pool_config.record_deposit(amount)?;

    emit!(DepositEvent {
        pool: pool_config.key(),
        commitment,
        leaf_index,
        amount,
        timestamp: Clock::get()?.unix_timestamp,
    });

    msg!("Deposit successful");
    Ok(())
}
