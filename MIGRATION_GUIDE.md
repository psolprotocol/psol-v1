# pSol Phase 3 Migration Guide

## Overview

This document describes the upgrade from Phase 2.5 to Phase 3 of the pSol Privacy Pool protocol.

## Breaking Changes

### 1. Deposit Instruction Signature Changed

**Before (Phase 2.5):**
```rust
pub fn deposit(
    ctx: Context<Deposit>,
    amount: u64,
    secret: [u8; 32],
    nullifier_preimage: [u8; 32],
) -> Result<()>
```

**After (Phase 3):**
```rust
pub fn deposit(
    ctx: Context<Deposit>,
    amount: u64,
    commitment: [u8; 32],
) -> Result<()>
```

**Migration Steps:**
1. Users now compute commitment OFF-CHAIN using Poseidon
2. Update client code to pass pre-computed commitment
3. Example JavaScript:
```javascript
const poseidon = require('circomlib').poseidon;
const commitment = poseidon([secret, nullifier_preimage, amount]);
await program.methods.deposit(amount, commitment).accounts({...}).rpc();
```

### 2. Private Transfer Now Implemented

**Before (Phase 2.5):**
- Returns `NotImplemented` error

**After (Phase 3):**
```rust
pub fn private_transfer(
    ctx: Context<PrivateTransfer>,
    proof_data: Vec<u8>,
    merkle_root: [u8; 32],
    nullifier_hash_0: [u8; 32],
    nullifier_hash_1: [u8; 32],
    output_commitment_0: [u8; 32],
    output_commitment_1: [u8; 32],
    fee: u64,
) -> Result<()>
```

### 3. No More dev-mode Bypass

**Before (Phase 2.5):**
```rust
#[cfg(feature = "dev-mode")]
{
    // Bypass verification
    return Ok(true);
}
```

**After (Phase 3):**
- `dev-mode` feature removed from production
- Test bypass only available in `#[cfg(test)]`
- All production builds perform full verification

### 4. VK Validation Enhanced

**Before (Phase 2.5):**
- Basic non-zero checks only
- `TODO [PHASE 3]` comments

**After (Phase 3):**
- Full BN254 on-curve validation
- All G1/G2 points validated
- Non-identity checks enforced

## Files Changed

| File | Change Type | Description |
|------|-------------|-------------|
| `instructions/deposit.rs` | **BREAKING** | New signature, off-chain commitment |
| `instructions/private_transfer.rs` | **NEW** | Full 2-in-2-out implementation |
| `instructions/set_verification_key.rs` | Enhanced | Full curve validation |
| `crypto/groth16_verifier.rs` | Enhanced | Removed dev-mode bypass |
| `crypto/poseidon.rs` | Changed | Documentation for off-chain model |
| `crypto/public_inputs.rs` | NEW | Public inputs structure |
| `lib.rs` | Updated | New instruction signatures |
| `instructions/mod.rs` | Updated | Clean exports |

## Deployment Steps

### 1. Update Client Code

```javascript
// Before
await program.methods.deposit(amount, secret, nullifierPreimage).rpc();

// After
const poseidon = require('circomlib').poseidon;
const commitment = poseidon([secret, nullifierPreimage, amount]);
await program.methods.deposit(amount, Array.from(commitment)).rpc();
```

### 2. Build and Deploy

```bash
# Build
cargo build-sbf

# Deploy (upgrade existing program)
solana program deploy target/deploy/psol_privacy.so \
  --program-id Ddokrq1M6hT9Vu63k4JWqVRSecyLeotNf8xKknKfRwvZ
```

### 3. Set Verification Key

The VK must come from a trusted setup ceremony matching your circuit:

```javascript
await program.methods.setVerificationKey(
  Array.from(vk.alpha_g1),
  Array.from(vk.beta_g2),
  Array.from(vk.gamma_g2),
  Array.from(vk.delta_g2),
  vk.ic.map(p => Array.from(p))
).accounts({...}).rpc();
```

## Circuit Requirements

### Withdrawal Circuit Public Inputs (6)
1. merkle_root
2. nullifier_hash
3. recipient
4. amount
5. relayer
6. relayer_fee

### Transfer Circuit Public Inputs (8)
1. merkle_root
2. nullifier_hash_0
3. nullifier_hash_1
4. output_commitment_0
5. output_commitment_1
6. fee
7. fee_recipient
8. reserved

### Hash Functions
- **Commitment**: `Poseidon(secret, nullifier_preimage, amount)` - OFF-CHAIN
- **Nullifier**: `Poseidon(nullifier_preimage, secret)` - OFF-CHAIN
- **Merkle Tree**: Keccak256 - ON-CHAIN

## Security Considerations

1. **Poseidon Parameters**: Must match circomlib exactly
   - Field: BN254 scalar field
   - t = 3 or 4 depending on inputs
   - RF = 8, RP = 57

2. **Verification Key**: Must come from trusted setup
   - Never use test VKs in production
   - Consider making VK immutable after first set

3. **No dev-mode in Production**
   - Build without any feature flags for production
   - `cargo build-sbf` (default, no features)

## Backward Compatibility

- Existing deposits remain valid (commitments already in tree)
- Users need updated client code for new deposits
- Existing nullifiers still work for withdrawals
- VK must be re-set if circuit changed

## Testing Checklist

- [ ] Deposit with off-chain commitment
- [ ] Withdrawal with valid proof
- [ ] Double-spend prevention (nullifier reuse)
- [ ] Invalid proof rejection
- [ ] VK validation (invalid points rejected)
- [ ] Private transfer (2-in-2-out)
- [ ] Fee transfer
- [ ] Pause/unpause
- [ ] Authority transfer
