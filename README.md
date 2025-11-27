# pSol Privacy Protocol (v1) - Phase 3

A ZK-based privacy pool for Solana using Poseidon commitments and Groth16 proof verification.

## Overview

pSol enables private token transfers on Solana through a shielded pool:

```
┌─────────────┐     deposit      ┌──────────────┐     withdraw     ┌─────────────┐
│   Public    │ ───────────────► │   Shielded   │ ───────────────► │   Public    │
│   Tokens    │                  │     Pool     │                  │   Tokens    │
└─────────────┘                  └──────────────┘                  └─────────────┘
      │                                │                                  ▲
      │                                │                                  │
      └─ commitment inserted ──────────┴─ ZK proof verified ──────────────┘
```

## Features

### Cryptographic Components
- **Poseidon Hashing**: Circuit-compatible hash function using `light-poseidon`
- **Groth16 Verification**: Full pairing-based ZK proof verification via Solana's alt_bn128 precompiles
- **BN254 Curve Operations**: G1/G2 point validation, negation, and pairing

### Protocol Features
- **Incremental Merkle Tree**: O(log n) insertions with root history
- **Per-Nullifier PDAs**: O(1) double-spend prevention
- **Relayer Support**: Privacy-preserving transaction submission
- **Admin Controls**: Pause/unpause, authority transfer

## Structure

```
programs/psol-privacy/
├── src/
│   ├── lib.rs              # Program entrypoint and documentation
│   ├── crypto/
│   │   ├── poseidon.rs     # Poseidon hash functions
│   │   ├── curve_utils.rs  # BN254 curve operations
│   │   ├── groth16_verifier.rs  # Groth16 proof verification
│   │   └── public_inputs.rs     # ZK circuit public inputs
│   ├── state/
│   │   ├── pool_config.rs  # Pool settings
│   │   ├── merkle_tree.rs  # Commitment storage
│   │   ├── verification_key.rs  # Groth16 VK
│   │   └── spent_nullifier.rs   # Nullifier tracking
│   ├── instructions/       # Deposit, withdraw, admin ops
│   ├── error.rs           # Error definitions
│   └── events.rs          # Event definitions
```

## Build Modes

### Production (default)
```bash
anchor build
```
Full Groth16 verification enabled. Invalid proofs are rejected.

### Development Mode
```bash
anchor build -- --features dev-mode
```
⚠️ **WARNING**: Proof verification is bypassed. NEVER deploy to mainnet!

## Protocol Flows

### Deposit
1. User generates random `(secret, nullifier_preimage)`
2. Call `deposit(amount, secret, nullifier_preimage)`
3. On-chain: commitment = Poseidon(secret, nullifier_preimage, amount)
4. Token transferred to vault, commitment inserted into Merkle tree
5. User saves `(secret, nullifier_preimage, leaf_index)` - **loss = loss of funds**

### Withdrawal
1. User computes Merkle path to their commitment
2. Generate Groth16 proof with:
   - Private: secret, nullifier_preimage, merkle_path
   - Public: merkle_root, nullifier_hash, recipient, amount, relayer, fee
3. Call `withdraw(proof, merkle_root, nullifier_hash, ...)`
4. On-chain: verify proof, mark nullifier spent, transfer tokens

## Account PDAs

| Account | Seeds | Purpose |
|---------|-------|---------|
| PoolConfig | `["pool", token_mint]` | Pool settings |
| MerkleTree | `["merkle_tree", pool_config]` | Commitment storage |
| VerificationKey | `["verification_key", pool_config]` | Groth16 VK |
| SpentNullifier | `["nullifier", pool_config, nullifier_hash]` | Double-spend prevention |
| Vault | `["vault", pool_config]` | Token custody |

## Security

- **Fail-closed**: Invalid proofs always rejected
- **Amount binding**: Commitment binds the deposit amount
- **Double-spend prevention**: Each nullifier usable once
- **Root history**: Allows for concurrent proof generation

## Testing

```bash
# Run unit tests
cargo test --package psol-privacy

# Run with dev-mode for integration tests
cargo test --package psol-privacy --features dev-mode
```

## Deployment

1. Generate program keypair
2. Update `declare_id!` in `lib.rs`
3. Set verification key via `set_verification_key`
4. Initialize pools via `initialize_pool`

## Dependencies

- `anchor-lang` 0.30.1
- `light-poseidon` 0.2.0 (Poseidon hashing)
- Solana 1.18 (alt_bn128 precompiles)

## License

MIT
