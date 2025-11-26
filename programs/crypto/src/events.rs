use anchor_lang::prelude::*;

#[event]
pub struct PoolInitialized {
    pub pool: Pubkey,
    pub authority: Pubkey,
    pub token_mint: Pubkey,
    pub tree_depth: u8,
    pub timestamp: i64,
}

#[event]
pub struct Deposit {
    pub pool: Pubkey,
    pub commitment: [u8; 32],
    pub leaf_index: u32,
    pub merkle_root: [u8; 32],
    pub amount: u64,
    pub timestamp: i64,
}

#[event]
pub struct Withdraw {
    pub pool: Pubkey,
    pub nullifier: [u8; 32],
    pub recipient: Pubkey,
    pub amount: u64,
    pub timestamp: i64,
}

#[event]
pub struct PrivateTransfer {
    pub pool: Pubkey,
    pub input_count: u8,
    pub output_count: u8,
    pub timestamp: i64,
}
