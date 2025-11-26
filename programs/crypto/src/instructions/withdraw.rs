use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};
use crate::{crypto, error::PrivacyError, events, state::*};

#[derive(Accounts)]
pub struct Withdraw<'info> {
    /// Pool configuration
    #[account(
        mut,
        seeds = [b"pool", pool_config.token_mint.as_ref()],
        bump = pool_config.bump,
    )]
    pub pool_config: Account<'info, PoolConfig>,

    /// Merkle tree state (for root verification)
    #[account(
        seeds = [b"merkle_tree", pool_config.key().as_ref()],
        bump,
        constraint = merkle_tree.pool == pool_config.key() @ PrivacyError::Unauthorized,
    )]
    pub merkle_tree: Account<'info, MerkleTree>,

    /// Nullifier set
    #[account(
        mut,
        seeds = [b"nullifiers", pool_config.key().as_ref()],
        bump,
        constraint = nullifier_set.pool == pool_config.key() @ PrivacyError::Unauthorized,
    )]
    pub nullifier_set: Account<'info, crypto::NullifierSet>,

    /// Token vault
    #[account(
        mut,
        seeds = [b"vault", pool_config.key().as_ref()],
        bump,
        constraint = vault.mint == pool_config.token_mint @ PrivacyError::InvalidAmount,
    )]
    pub vault: Account<'info, TokenAccount>,

    /// Recipient's token account
    #[account(
        mut,
        constraint = recipient_token_account.mint == pool_config.token_mint @ PrivacyError::InvalidAmount,
    )]
    pub recipient_token_account: Account<'info, TokenAccount>,

    /// Token program
    pub token_program: Program<'info, Token>,

    /// Withdrawer can be any relayer
    #[account(mut)]
    pub withdrawer: Signer<'info>,
}

pub fn handler(
    ctx: Context<Withdraw>,
    nullifier: [u8; 32],
    recipient: Pubkey,
    amount: u64,
    merkle_root: [u8; 32],
    proof_data: Vec<u8>,
) -> Result<()> {
    let pool_config = &mut ctx.accounts.pool_config;
    let merkle_tree = &ctx.accounts.merkle_tree;
    let nullifier_set = &mut ctx.accounts.nullifier_set;

    pool_config.require_not_paused()?;

    require!(amount > 0, PrivacyError::InvalidAmount);
    require!(
        ctx.accounts.vault.amount >= amount,
        PrivacyError::InsufficientBalance
    );

    require!(
        merkle_tree.is_known_root(&merkle_root),
        PrivacyError::InvalidMerkleRoot
    );

    require!(
        !nullifier_set.is_spent(&nullifier),
        PrivacyError::NullifierAlreadySpent
    );

    require!(
        ctx.accounts.recipient_token_account.owner == recipient,
        PrivacyError::Unauthorized
    );

    let public_inputs = crypto::ZkPublicInputs {
        merkle_root,
        nullifier,
        recipient,
        amount,
    };

    let proof_valid = crypto::verify_proof(&proof_data, &public_inputs)?;
    require!(proof_valid, PrivacyError::InvalidProof);

    nullifier_set.mark_spent(nullifier)?;

    let seeds = &[
        b"pool",
        pool_config.token_mint.as_ref(),
        &[pool_config.bump],
    ];
    let signer_seeds = &[&seeds[..]];

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
    token::transfer(cpi_ctx, amount)?;

    pool_config.increment_withdrawals()?;

    emit!(events::Withdraw {
        pool: pool_config.key(),
        nullifier,
        recipient,
        amount,
        timestamp: Clock::get()?.unix_timestamp,
    });

    msg!("Withdrawal successful");
    msg!("Nullifier: {:?}", nullifier);
    msg!("Recipient: {}", recipient);
    msg!("Amount: {}", amount);

    Ok(())
}
