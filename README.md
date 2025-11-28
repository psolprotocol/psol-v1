# pSol Privacy Pool - Phase 3

Production-ready ZK-based privacy pool for Solana with Groth16 proof verification.

## Features

### Core
- **Private deposits**: Shield tokens in the pool
- **Private withdrawals**: Unshield with ZK proof
- **Private transfers**: 2-in-2-out transfers within pool
- **Relayer support**: Privacy-preserving transaction submission

### Cryptographic
- **Groth16 verification**: Full pairing-based ZK verification
- **BN254 curve**: alt_bn128 precompiles for efficiency
- **Poseidon hashing**: Circuit-compatible commitments (off-chain)
- **Keccak256 Merkle**: Efficient on-chain tree

### Security
- **Fail-closed**: Invalid proofs always rejected
- **VK validation**: All curve points validated
- **Double-spend prevention**: Nullifier PDAs
- **No dev-mode bypass**: Production builds always verify

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        pSol Protocol                             │
│                                                                  │
│  ┌──────────┐      ┌──────────────┐      ┌──────────────┐       │
│  │  Deposit │      │   Shielded   │      │   Withdraw   │       │
│  │  Tokens  │ ───► │     Pool     │ ───► │   Tokens     │       │
│  └──────────┘      └──────────────┘      └──────────────┘       │
│       │                   │                     ▲                │
│       │            ┌──────┴──────┐              │                │
│       │            │   Private   │              │                │
│       │            │  Transfer   │              │                │
│       │            └─────────────┘              │                │
│       │                                         │                │
│       └─────── commitment ──────────── ZK proof ┘                │
└─────────────────────────────────────────────────────────────────┘
```

## Quick Start

### 1. Build

```bash
cargo build-sbf
```

### 2. Deploy

```bash
solana program deploy target/deploy/psol_privacy.so
```

### 3. Initialize Pool

```javascript
await program.methods.initializePool(20, 100).accounts({
  poolConfig,
  merkleTree,
  verificationKey,
  vault,
  tokenMint,
  authority,
  tokenProgram,
  systemProgram,
  rent,
}).rpc();
```

### 4. Set Verification Key

```javascript
await program.methods.setVerificationKey(
  vk_alpha_g1,
  vk_beta_g2,
  vk_gamma_g2,
  vk_delta_g2,
  vk_ic,
).accounts({
  poolConfig,
  verificationKey,
  authority,
}).rpc();
```

### 5. Deposit

```javascript
// Compute commitment OFF-CHAIN
const poseidon = require('circomlib').poseidon;
const secret = crypto.randomBytes(32);
const nullifierPreimage = crypto.randomBytes(32);
const commitment = poseidon([secret, nullifierPreimage, amount]);

// Deposit on-chain
await program.methods.deposit(amount, commitment).accounts({
  poolConfig,
  merkleTree,
  vault,
  userTokenAccount,
  user,
  tokenProgram,
}).rpc();

// SAVE: (secret, nullifierPreimage, leafIndex)
```

### 6. Withdraw

```javascript
// Generate proof OFF-CHAIN using snarkjs
const proof = await generateWithdrawalProof({
  secret,
  nullifierPreimage,
  merklePath,
  merkleRoot,
  recipient,
  amount,
  relayer,
  relayerFee,
});

// Withdraw on-chain
await program.methods.withdraw(
  proof,
  merkleRoot,
  nullifierHash,
  recipient,
  amount,
  relayer,
  relayerFee,
).accounts({
  poolConfig,
  merkleTree,
  verificationKey,
  spentNullifier,
  vault,
  recipientTokenAccount,
  relayerTokenAccount,
  withdrawer,
  tokenProgram,
  systemProgram,
}).rpc();
```

## Account Structure

| Account | PDA Seeds | Purpose |
|---------|-----------|---------|
| PoolConfig | `["pool", token_mint]` | Pool settings |
| MerkleTree | `["merkle_tree", pool_config]` | Commitment storage |
| VerificationKey | `["verification_key", pool_config]` | Groth16 VK |
| SpentNullifier | `["nullifier", pool_config, nullifier_hash]` | Double-spend prevention |
| Vault | `["vault", pool_config]` | Token custody |

## Circuit Compatibility

### Hash Functions

| Purpose | Function | Location |
|---------|----------|----------|
| Commitment | Poseidon(secret, nullifier, amount) | Off-chain |
| Nullifier | Poseidon(nullifier, secret) | Off-chain |
| Merkle tree | Keccak256(left, right) | On-chain |

### Poseidon Parameters (circomlib)
- Curve: BN254
- Field: Scalar field (Fr)
- Full rounds: 8
- Partial rounds: 57

## Security Model

1. **Privacy**: Deposits and withdrawals unlinkable
2. **Soundness**: Cannot withdraw more than deposited
3. **No double-spend**: Nullifiers enforced via PDAs
4. **Fail-closed**: Invalid proofs always rejected

## License

MIT
