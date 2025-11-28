//! Instruction handlers for pSol Privacy Pool
//!
//! # Instructions
//! - `initialize_pool` - Create new privacy pool
//! - `set_verification_key` - Configure Groth16 VK
//! - `deposit` - Deposit tokens with commitment
//! - `withdraw` - Withdraw tokens with ZK proof (FAIL-CLOSED)
//! - `private_transfer` - Internal transfer (NOT IMPLEMENTED)
//!
//! # Admin Instructions
//! - `pause` / `unpause` - Emergency controls
//! - `update_authority` - Transfer admin rights

pub mod admin;
pub mod deposit;
pub mod initialize_pool;
pub mod private_transfer;
pub mod set_verification_key;
pub mod withdraw;

pub use admin::*;
pub use deposit::*;
pub use initialize_pool::*;
pub use private_transfer::*;
pub use set_verification_key::*;
pub use withdraw::*;
