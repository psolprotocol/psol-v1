pSol Privacy Protocol

Zero-knowledge privacy pool for Solana with Groth16 proof verification, nullifier-based double-spend protection, a TypeScript SDK, and a hardened relayer service.

Network: Solana devnet

Program ID: <DEVNET_PROGRAM_ID>

Explorer: https://explorer.solana.com/address/<DEVNET_PROGRAM_ID>?cluster=devnet

Repository: https://github.com/psolprotocol/psol-v1

X / Twitter: https://x.com/psolprotocol

Update the Program ID above from Anchor.toml after deployment.

Components

This repository contains three main pieces:

On-chain program (programs/psol-privacy)
Anchor-based privacy pool with deposits, private transfers, and withdrawals.

TypeScript SDK (psol-sdk)
Client library for building commitments, generating proofs, and integrating pSol into dApps.

Relayer service (services/psol-relayer)
Node.js service that submits private withdrawals on behalf of users and handles fees, rate limits, and job tracking.

Features
Core protocol

Private deposits into a shielded pool of SPL tokens.

Private 2-in / 2-out transfers inside the pool.

Private withdrawals with relayer and fee support.

Merkle tree of commitments with root history.

Cryptography

Groth16 zkSNARK verification on-chain.

BN254 curve via Solana alt_bn128 precompiles.

Poseidon-based commitments and nullifiers (off-chain, circomlib-compatible).

Keccak256-based Merkle tree on-chain.

Security

Fail-closed design, invalid proofs are always rejected.

Verification key validation for all curve points.

Nullifier PDAs to prevent double spends.

No dev-mode bypass in production builds.

Repository layout

psol-v1/
  Anchor.toml
  Cargo.toml
  package.json
  programs/
    psol-privacy/ # On-chain Anchor program
  psol-sdk/ # TypeScript SDK
  services/
    psol-relayer/ # Relayer service (Node + Redis)

Getting started
1. Prerequisites

Rust toolchain

Solana CLI

Anchor CLI

Node.js and npm

Docker (for Redis used by the relayer)

2. Build the program and generate IDL
git clone https://github.com/psolprotocol/psol-v1.git
cd psol-v1

solana config set --url devnet

# Clean and build
cargo clean
anchor build


This produces:

target/deploy/psol_privacy.so

target/idl/psol_privacy.json

Update Anchor.toml and the README Program ID with the id you want to use.

3. Deploy to devnet
# Use the same program id configured in Anchor.toml
solana program deploy target/deploy/psol_privacy.so \
  --program-id <DEVNET_PROGRAM_ID>


Verify deployment on Solana Explorer with the devnet link above.

SDK usage (local)
cd psol-sdk
npm install
npm run build


Typical flow from a dApp:

Create a note with secret and nullifier preimage.

Compute Poseidon commitment off-chain.

Call the Anchor program to deposit into the pool.

Use the SDK to build Merkle paths and Groth16 proofs for transfers and withdrawals.

Submit withdrawals either directly or via the relayer.

See psol-sdk/examples/ for a sample deposit/withdraw script targeting devnet.

Relayer service (devnet)

The relayer runs as a separate service that:

Accepts withdrawal requests over HTTP with a zk proof and withdrawal parameters.

Validates Merkle root and nullifier state.

Optionally verifies proofs off-chain (when VK JSON is provided).

Submits transactions from its own wallet.

Tracks job status in Redis and exposes health/metrics endpoints.

Run locally
cd services/psol-relayer

# Start Redis
docker compose up -d redis

# Install dependencies and start relayer
npm install
npm run dev


Health check:

curl http://localhost:3000/health


Protected endpoints require an API key configured via environment variables (see services/psol-relayer/README.md).

High-level roadmap

Phase 1: Prototype deposit / withdraw

Phase 2: Anchor program structure and state model

Phase 3: Full privacy protocol (deposits, transfers, withdrawals)

Phase 4: Security hardening, tests, relayer integration

Phase 5: SDK, example flows, and preparation for audited mainnet deployment

License

MIT
