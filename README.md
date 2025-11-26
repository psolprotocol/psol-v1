# pSol Privacy Protocol (v1)

pSol is a privacy-preserving token pool on Solana.  
This version implements deposit, withdrawal with zero-knowledge proof verification (development verifier), Merkle tree state, and nullifier tracking.

## Structure
- programs/psol-privacy  
  On-chain program: pool config, Merkle tree, nullifier set, deposit, withdrawal.
- tests/psol-privacy.ts  
  Full test suite for Solana Playground and local Anchor.

## Features
- Merkle tree (incremental, Keccak hash)
- Nullifier set with capacity control
- Deposit with commitment insertion
- Withdrawal flow (development verifier placeholder)
- PDA-based vault management
- Event logging for all operations

## Testing
To run tests in Solana Playground:
1. Open project  
2. Build  
3. Deploy  
4. Run test suite

For local Anchor:
anchor build
anchor test

markdown
Copy code

## Deployment
Program ID is configured in `lib.rs` via `declare_id!`.  
Update ID after deployment and rebuild before testing.

## Notes
- Zero-knowledge verifier is a development stub; production requires Groth16 verifier integration.
- Tree depth and history size are reduced for testing.
