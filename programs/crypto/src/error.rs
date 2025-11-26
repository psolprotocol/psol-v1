use anchor_lang::prelude::*;

#[error_code]
pub enum PrivacyError {
    #[msg("Invalid proof provided")]
    InvalidProof,

    #[msg("Nullifier already spent")]
    NullifierAlreadySpent,

    #[msg("Invalid Merkle root")]
    InvalidMerkleRoot,

    #[msg("Merkle tree is full")]
    MerkleTreeFull,

    #[msg("Invalid tree depth")]
    InvalidTreeDepth,

    #[msg("Invalid amount")]
    InvalidAmount,

    #[msg("Arithmetic overflow")]
    ArithmeticOverflow,

    #[msg("Invalid commitment")]
    InvalidCommitment,

    #[msg("Pool is paused")]
    PoolPaused,

    #[msg("Unauthorized operation")]
    Unauthorized,

    #[msg("Invalid public inputs")]
    InvalidPublicInputs,

    #[msg("Root history buffer full")]
    RootHistoryFull,

    #[msg("Insufficient vault balance")]
    InsufficientBalance,

    #[msg("Invalid proof format")]
    InvalidProofFormat,

    #[msg("Nullifier set full")]
    NullifierSetFull,
}
