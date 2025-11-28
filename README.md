# pSol Privacy Pool – Phase 3 (devnet)

Production-ready zero-knowledge privacy pool for Solana with Groth16 proof verification and nullifier-based double-spend protection.

- Network: Solana **devnet**
- Program ID: `Ddokrq1M6hT9Vu63k4JWqVRSecyLeotNf8xKknKfRwvZ`
- Explorer: https://explorer.solana.com/address/Ddokrq1M6hT9Vu63k4JWqVRSecyLeotNf8xKknKfRwvZ?cluster=devnet  
- Repository: https://github.com/psolprotocol/psol-v1  
- Twitter / X: https://x.com/psolprotocol  

---

## Features

### Core

- **Private deposits** – shield SPL tokens into the pool.
- **Private withdrawals** – unshield with a Groth16 proof.
- **Private transfers** – 2-in / 2-out shielded transfers inside the pool.
- **Relayer fields** – withdraw flow includes relayer and fee fields for privacy-preserving submission.

### Cryptographic

- **Groth16 verification** – full pairing-based ZK verification on-chain.
- **BN254 curve** – via Solana alt_bn128 precompiles.
- **Poseidon commitments** – off-chain commitments and nullifiers compatible with standard circomlib Poseidon.
- **Keccak256 Merkle** – on-chain incremental Merkle tree with root history.

### Security

- **Fail-closed** – invalid proofs are always rejected.
- **Verification key validation** – on-curve and non-identity checks for all VK points.
- **Double-spend protection** – spent nullifiers tracked via PDAs.
- **No dev-mode bypass** – production builds always execute full Groth16 verification.

---

## Architecture Overview

The program is an Anchor-based Solana contract that maintains:

- A **pool configuration** account describing the pool and authority.
- A **Merkle tree** account storing commitments and root history.
- A **verification key** account storing Groth16 VK data.
- A **vault** account holding the pooled SPL tokens.
- **Spent nullifier** accounts preventing double spends.

High-level flows:

1. **Deposit**
   - User computes a commitment off-chain using Poseidon.
   - Tokens are transferred into the vault.
   - Commitment is inserted into the on-chain Merkle tree.

2. **Private transfer (2-in / 2-out)**
   - User proves in zero-knowledge that two existing notes are spent
     and two new notes are created, preserving value.
   - Nullifiers for inputs are marked as spent.
   - New commitments are added to the Merkle tree.

3. **Withdraw**
   - User provides a proof that a commitment exists in the tree
     and has not been spent.
   - Nullifier is recorded.
   - Tokens are released from the vault to the recipient, optionally via a relayer.

---

## Quick Start (Local Build and Devnet Deploy)

### 1. Prerequisites

- Rust toolchain
- Solana CLI
- Anchor CLI
- Node.js and Yarn or npm

### 2. Clone and build

```bash
git clone https://github.com/psolprotocol/psol-v1.git
cd psol-v1

solana config set --url devnet
cargo build-sbf
3. Deploy to devnet (fixed program ID)
bash
Copy code
solana program deploy target/deploy/psol_privacy.so \
  --program-id Ddokrq1M6hT9Vu63k4JWqVRSecyLeotNf8xKknKfRwvZ
After deployment, verify on Solana Explorer using the link above.

4. Initialize pool (Anchor client example)
ts
Copy code
await program.methods.initializePool(depth, rootHistorySize).accounts({
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
5. Set verification key
ts
Copy code
await program.methods.setVerificationKey(
  vkAlphaG1,
  vkBetaG2,
  vkGammaG2,
  vkDeltaG2,
  vkIc,
).accounts({
  poolConfig,
  verificationKey,
  authority,
}).rpc();
6. Deposit (off-chain Poseidon, on-chain insert)
Off-chain (example only):

ts
Copy code
const poseidon = require("circomlib").poseidon;

const secret = crypto.randomBytes(32);
const nullifierPreimage = crypto.randomBytes(32);

const commitment = poseidon([secret, nullifierPreimage, amount]);
// Save (secret, nullifierPreimage, leafIndex) securely off-chain
On-chain:

ts
Copy code
await program.methods.deposit(amount, commitment).accounts({
  poolConfig,
  merkleTree,
  vault,
  userTokenAccount,
  user,
  tokenProgram,
}).rpc();
7. Withdraw (Groth16 proof)
Off-chain, generate proof with snarkjs or equivalent:

ts
Copy code
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
On-chain:

ts
Copy code
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
Note: This repository contains the on-chain program only. Valid proofs require circuits and proving/verifying keys that match the public input layout described in MIGRATION_GUIDE.md.

Account Model
Account	PDA Seeds	Purpose
PoolConfig	["pool", token_mint]	Pool configuration and owner
MerkleTree	["merkle_tree", pool_config]	Commitment storage + roots
VerificationKey	["verification_key", pool_config]	Groth16 verification key
SpentNullifier	["nullifier", pool_config, nullifier]	Double-spend prevention
Vault	["vault", pool_config]	Token custody account

Adjust seeds above if the on-chain definitions change; the Rust state modules are the source of truth.

Circuit Compatibility
Hash Functions
Purpose	Function	Location
Commitment	Poseidon(secret, nullifier, amount)	Off-chain
Nullifier	Poseidon(nullifier, secret)	Off-chain
Merkle parent	`Keccak256(left	

Poseidon Parameters
Curve: BN254

Field: Scalar field (Fr)

Parameters: Standard circomlib Poseidon over BN254 (Fr)

Circuits and client-side hashing must match these parameters and the ordering described in MIGRATION_GUIDE.md.

Security Model
Privacy – deposits, transfers, and withdrawals are unlinkable at the protocol level if used correctly.

Soundness – users cannot withdraw more value than they deposited; value conservation is enforced by the circuit and Groth16 verification.

No double-spend – nullifiers are enforced via PDAs; reused nullifiers are rejected.

Fail-closed – malformed proofs, invalid VKs, or inconsistent inputs cause instruction failure.

Roadmap (high level)
Phase 1 – Prototype (basic deposit / withdraw PoC)

Phase 2 – Security skeleton (state models, Anchor structure)

Phase 2.5 – Cryptographic wiring (Groth16, Poseidon, Merkle tree)

Phase 3 – Core privacy protocol (current devnet deployment)

Phase 4 – Security hardening, tests, audit preparation

Phase 5 – SDK, example dApps, and mainnet deployment

License
MIT
