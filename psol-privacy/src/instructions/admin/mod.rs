//! Admin Instructions for pSol Privacy Pool
//!
//! Administrative functions for pool management:
//! - `pause` - Emergency stop (disable deposits/withdrawals)
//! - `unpause` - Resume operations
//! - `update_authority` - Transfer admin rights

pub mod pause;
pub mod unpause;
pub mod update_authority;

pub use pause::*;
pub use unpause::*;
pub use update_authority::*;
