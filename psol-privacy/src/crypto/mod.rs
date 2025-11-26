//! Cryptographic primitives for pSol Privacy Pool
//!
//! # Phase 2 Status
//! This module contains interface definitions and placeholder implementations.
//! Real cryptographic operations will be implemented in Phase 3.
//!
//! # Security Note
//! All verification functions are FAIL-CLOSED: they return errors until
//! real implementations are added. This prevents fund theft.

pub mod curve_utils;
pub mod groth16_verifier;
pub mod poseidon;
pub mod public_inputs;

pub use groth16_verifier::{verify_groth16_proof, Groth16Proof};
pub use poseidon::{hash_commitment, hash_nullifier, hash_two_to_one};
pub use public_inputs::ZkPublicInputs;
