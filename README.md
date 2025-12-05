> [!IMPORTANT]
> Status: Devnet-only, unaudited, work in progress.
> Current capabilities: deposit + WIP withdraw, prototype relayer.
> Not production-ready. Do not use with real funds.

pSol Privacy Protocol

Zero-Knowledge Privacy Pool for Solana

pSol is a private transfer protocol for Solana.
It provides confidential deposits and withdrawals using Merkle-based commitments, nullifiers, and zk-SNARK proofs.
The protocol is fully non-custodial and operates as a smart contract deployed on Solana.

This repository contains the on-chain program, the TypeScript SDK, client examples, and devnet deployment references.

Repository Structure
psol-v1/
│
├── programs/
│   └── psol-privacy/          # Anchor-based on-chain program
│
├── psol-sdk/
│   ├── src/
│   │   ├── idl/               # Generated IDL (psol_privacy.json)
│   │   ├── utils/             # Merkle helpers, encoding, parsing
│   │   ├── client.ts          # High-level SDK interface
│   │   └── index.ts
│   └── package.json
│
├── services/
│   ├── relayer/               # Relayer prototype (withdraw execution)
│   └── prover/                # Stub for ZK prover integration
│
├── migrations/                # Future pool migrations
│
├── test/                      # JS/TS integration tests
│
├── Anchor.toml                # Anchor project config
├── Cargo.toml                 # Rust workspace config
├── README.md                  # You are here
└── ...

Features
1. Private Deposits

Users deposit SPL tokens into the pool.
The program generates a commitment leaf and inserts it into the Merkle tree.

2. Shielded Withdrawals

Withdrawals require:

Merkle proof of deposit

ZK proof verifying ownership

Nullifier to prevent double spending

3. Relayer Support

Users may withdraw through a relayer that pays gas and receives a fee.

4. Pause / Unpause

Admin controls for emergency response.

5. Authority Transfer

Two-phase authority rotation.

Devnet Deployment

Build and deployment were performed using Solana Playground (beta) due to environment compatibility and ZK workflows.

Program
Component	Address
Program ID	2uPHpGmCNoTk6mnzzuP3DGbVyMiDPrQYRxkYBHMxwhBi

Explorer:
https://explorer.solana.com/address/2uPHpGmCNoTk6mnzzuP3DGbVyMiDPrQYRxkYBHMxwhBi?cluster=devnet

Test Pool Addresses
Item	Address
Token Mint	9cnm3fpXqBBUU8byYq8rZbeCbMxvCReh5LF6XSjqaaoJ
Pool Config	EmeSBaC18Arn626HjyvYicGjXzjg4cx1wA1jV91w1NFD
Merkle Tree	D1Fx4ts24q81dKc9UMAyDoXhT5b7nyFTBVccmCJWy85H
Vault	82amuqkKZQUnMnaq2Z9dDotafYicCoAwtPLyYatgbS3B
Verified Transactions
Action	Link
Pool Init	https://explorer.solana.com/tx/2yGycUqWK88apbA3ftoKpfUdm1d8gXjTbqzrAkKbK9W887CNJzfvMXYZJNFC4wVLPuFDbUgRoXHWTgYHK8epZFdJ?cluster=devnet

First Deposit	https://explorer.solana.com/tx/3ruP1eDGR68QVkW44RsJSw6iuGBLN8GUDXPw2RduyT1sPjrWFHVp23MyTbKZUy6AMURsBGcqQzV73JTqE5drdwtk?cluster=devnet
Current Status
Feature	Status
Deposits	Working
Withdrawals	Pending ZK prover integration
Relayer	Prototype only, not deployed
Building and Deploying (Recommended Method)

Anchor builds require very specific toolchains that break easily.
The recommended flow is:

Option A: Use Solana Playground (official recommendation)

This avoids local toolchain issues entirely.

Upload program folder

Build with Anchor 0.30 backend

Deploy to Devnet

Download IDL and .so file

Sync them into /psol-sdk/src/idl

This is already proven to work for pSol.

Option B: Docker Build (if experienced)

Anchor CLI 0.30.1:

anchor build


This uses:

docker pull backpackapp/build:v0.30.1


No Rust installation required.

SDK Usage Example

Basic deposit test:

import { pSolClient } from "./psol-sdk";
import { Keypair, PublicKey, Connection } from "@solana/web3.js";

const PROGRAM_ID = new PublicKey("2uPHpGmCNoTk6mnzzuP3DGbVyMiDPrQYRxkYBHMxwhBi");

async function run() {
  const connection = new Connection("https://api.devnet.solana.com");
  const user = Keypair.generate();

  const client = await pSolClient({
    connection,
    wallet: user,
    programId: PROGRAM_ID
  });

  const result = await client.deposit({
    amount: 1_000_000,
    commitment: crypto.randomBytes(32),
  });

  console.log("Deposit signature:", result);
}

run();

ZK / Merkle Architecture
Commitment Tree

Sparse Merkle tree

Default depth: 20

History window: 100 roots

Stored as PDA: ["merkle_tree", pool_config]

Verification Key

Groth16 keypair stored as PDA

set_verification_key loads vk into PDA

lock_verification_key freezes vk permanently

Withdrawal Proof

Proof inputs:

Merkle root

Nullifier hash

Recipient

Amount

Relayer address

Relayer fee

Program validations:

Nullifier has not been spent

Root is valid in history

Proof is valid

Vault has enough balance

Relayer Architecture (WIP)

The relayer:

Submits withdrawal on behalf of user

Takes a fee

Prevents deanonymization

Optionally verifies proofs off-chain before forwarding

Folder services/relayer contains the initial implementation.

Roadmap

v1.0 Completed

Core logic

Deposits

Merkle tree insertion

Config PDAs

Devnet deployment

SDK basic tests

v1.1 Next

Withdraw (ZK circuits)

Prover service

Relayer deployment

Monitoring dashboard

Enhanced audit logs

v2.0

Multi-token support

Multiple pools

Shielded accounts

Private swaps

Contributing

Contributors must install:

Node.js 18+

Anchor CLI 0.30.1 (recommended: Docker mode)

Solana CLI 1.18.x

Lint:

npm run lint


Test:

npm run test


Current Limitations and Remaining Work (Grant Scope)

pSol is deployed on devnet and the core logic is functional (pool creation, deposits, Merkle tree updates, SDK, relayer implementation).
The remaining components require specialized ZK engineering effort and security review.

1. Zero Knowledge Circuits (Not yet implemented)

The withdrawal circuit is not complete. The following elements still need to be delivered:

Circom circuit for membership proof and nullifier enforcement

Poseidon hashing inside circuit

Public inputs: root, nullifier hash, recipient, relayer, relayer fee

Witness generation pipeline

Trusted setup (Powers of Tau)

ZKey and WASM artifacts

On-chain verification key compression

2. Withdrawal Flow

The full ZK withdrawal path is not live. Required steps:

Generate proof using final circuits

Verify proof inside worker

Submit anchored withdrawal instruction

End to end devnet test of deposit → proof → withdrawal

3. Relayer Hardening

Relayer works, but not production-ready.

Needed:

Multi-relayer support

Fee market

Job receipts

Censorship resistance

Rate limiting tuning

Better monitoring

4. Security Review

Required before mainnet:

Formal audit of Anchor program

Independent ZK circuit audit

Review of relayer infrastructure

Deterministic CI pipeline

5. Performance Tuning

Compute budget optimization

Handling large Merkle trees (depth 20–22)

Reducing proof generation time


License

MIT