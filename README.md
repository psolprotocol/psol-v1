# pSol Privacy Protocol

A privacy-preserving token pool for Solana using zero-knowledge proofs. Inspired by Tornado Cash, adapted for Solana's architecture.

## Status

**Current Phase:** Devnet Testing  
**Program Deployed:** Yes (devnet only)  
**Production Ready:** No

### What Works

- Pool initialization
- Token deposits with commitment tracking
- Merkle tree state management (depth 20, 100 root history)
- Nullifier-based double-spend prevention
- Admin controls (pause/unpause, authority transfer)
- Event emission for all operations

### What Needs Work

- ZK proof verification is currently in test mode (bypassed)
- Circuits need compilation and trusted setup
- Verification key not yet loaded on-chain
- Withdraw function untested (requires proof generation)
- SDK incomplete (missing proof generation)
- Relayer service not deployed
- No professional security audit

## Architecture

### On-Chain Program (Rust/Anchor)

Solana program implementing:
- Deposit: Accept tokens and store commitment in Merkle tree
- Withdraw: Verify ZK proof, check nullifier, transfer tokens
- Merkle tree: Keccak256-based, 20 levels deep
- Admin: Authority management, emergency pause

**Program ID (devnet):** `2uPHpGmCNoTk6mnzzuP3DGbVyMiDPrQYRxkYBHMxwhBi`

### SDK (TypeScript)

Client library for:
- Generating commitments and nullifiers
- Building Merkle proofs
- Proof generation (planned, not implemented)
- Transaction construction

### Relayer Service (Node.js)

Backend service for:
- Submitting withdrawals on behalf of users
- Fee collection
- Rate limiting
- Job queue management

Not yet deployed.

### ZK Circuits (Circom)

Two circuit versions:
- `withdraw.circom` - Poseidon-based Merkle tree
- `withdraw_keccak.circom` - Keccak256-based (matches on-chain)

Status: Written but not compiled. No trusted setup performed.

## Deployment Info

**Network:** Solana Devnet  
**Program:** 2uPHpGmCNoTk6mnzzuP3DGbVyMiDPrQYRxkYBHMxwhBi  
**Test Token:** 9cnm3fpXqBBUU8byYq8rZbeCbMxvCReh5LF6XSjqaaoJ  
**Pool Config:** EmeSBaC18Arn626HjyvYicGjXzjg4cx1wA1jV91w1NFD

View on [Solana Explorer](https://explorer.solana.com/address/2uPHpGmCNoTk6mnzzuP3DGbVyMiDPrQYRxkYBHMxwhBi?cluster=devnet)

## Tested Transactions

**Pool Initialization:**  
https://explorer.solana.com/tx/2yGycUqWK88apbA3ftoKpfUdm1d8gXjTbqzrAkKbK9W887CNJzfvMXYZJNFC4wVLPuFDbUgRoXHWTgYHK8epZFdJ?cluster=devnet

**First Deposit:**  
https://explorer.solana.com/tx/3ruP1eDGR68QVkW44RsJSw6iuGBLN8GUDXPw2RduyT1sPjrWFHVp23MyTbKZUy6AMURsBGcqQzV73JTqE5drdwtk?cluster=devnet

## Security

### Self-Audit Completed

A comprehensive security review was conducted, identifying:
- 2 critical issues (hash function alignment, missing ZK setup)
- 3 high severity issues (2 fixed, 1 operational)
- 4 medium severity issues
- 5 low severity issues

**Fixes Applied:**
- H-1: Relayer address bug in transaction builder
- H-2: MAX_RELAYER_FEE_BPS enforcement (10% cap)

See `SECURITY_AUDIT.md` for full details.

### Known Limitations

**Critical:**
- ZK proof verification currently bypassed for testing
- No trusted setup ceremony performed
- Verification key not configured

**Important:**
- Root history size may be insufficient under heavy load
- No deposit denomination enforcement (privacy leak)
- No finalization delay on deposits (reorg risk)

### Required Before Mainnet

1. Professional security audit (estimated $100-150k)
2. Trusted setup ceremony with multiple participants
3. Circuit compilation and verification key generation
4. Full end-to-end testing with real proofs
5. Relayer infrastructure deployment
6. Comprehensive integration tests

## Building

Note: Due to Rust version requirements, building requires a compatible toolchain. The program was successfully built and deployed using Solana Playground.

### Requirements

- Rust 1.77+
- Solana CLI 1.18+
- Anchor CLI 0.30.1
- Node.js 18+

### Build Steps

```bash
# Install dependencies
npm install

# Build program (requires compatible Rust version)
anchor build

# Deploy to devnet
anchor deploy --provider.cluster devnet

# Run tests
anchor test --provider.cluster devnet
```

**Note:** Building locally may encounter Rust version conflicts with Solana's BPF toolchain. Solana Playground provides a compatible environment.

## Project Structure

```
psol-v1/
├── programs/psol-privacy/     # Anchor program (Rust)
│   ├── src/
│   │   ├── instructions/      # Program instructions
│   │   ├── state/            # Account structures
│   │   ├── crypto/           # ZK verification
│   │   └── lib.rs
│   └── Cargo.toml
├── psol-sdk/                  # TypeScript SDK
│   ├── src/
│   │   ├── client/           # Program client
│   │   ├── crypto/           # Hashing, Merkle trees
│   │   ├── types/            # Type definitions
│   │   └── idl/              # Generated IDL
│   └── package.json
├── services/psol-relayer/     # Relayer service
│   ├── src/
│   │   ├── server.ts         # Express server
│   │   ├── transaction.ts    # TX builder
│   │   ├── queue.ts          # Job queue
│   │   └── validation.ts     # Request validation
│   └── package.json
├── circuits/                  # ZK circuits
│   ├── withdraw.circom
│   └── withdraw_keccak.circom
├── tests/                     # Integration tests
└── scripts/                   # Build/deploy scripts
```

## Usage

### Initialize Pool

```typescript
import { Program } from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";

const tx = await program.methods
  .initializePool(20, 100)  // depth, root history size
  .accounts({
    poolConfig,
    merkleTree,
    verificationKey,
    vault,
    tokenMint,
    authority: wallet.publicKey,
    // ...
  })
  .rpc();
```

### Deposit

```typescript
import { BN } from "@coral-xyz/anchor";

const commitment = generateCommitment(secret, nullifier);

const tx = await program.methods
  .deposit(new BN(amount), commitment)
  .accounts({
    poolConfig,
    merkleTree,
    vault,
    depositorTokenAccount,
    depositor: wallet.publicKey,
    // ...
  })
  .rpc();
```

### Withdraw

Not yet functional. Requires:
1. Proof generation from circuit
2. Verification key loaded on-chain
3. Valid Merkle proof of commitment

## Roadmap

### Phase 1: Core Implementation (Complete)
- Anchor program structure
- Basic deposit/withdraw logic
- Merkle tree implementation
- State management

### Phase 2: Security Hardening (Complete)
- Self security audit
- Bug fixes
- Error handling
- Event system

### Phase 3: ZK Integration (In Progress)
- Circuit compilation
- Trusted setup
- Verification key configuration
- Proof generation in SDK

### Phase 4: Production Preparation (Not Started)
- Professional security audit
- Relayer deployment
- Comprehensive testing
- Documentation

### Phase 5: Mainnet (Not Started)
- Final audit review
- Mainnet deployment
- Monitoring setup
- Bug bounty program

## Dependencies

### On-Chain
- `anchor-lang` 0.30.1
- `anchor-spl` 0.30.1
- `solana-program` 1.18

### SDK
- `@coral-xyz/anchor` ^0.30.1
- `@solana/web3.js` ^1.95.0
- `@solana/spl-token` ^0.4.0

### Relayer
- `express` ^4.18.0
- `ioredis` ^5.3.0
- `@solana/web3.js` ^1.95.0

## Contributing

This project is in early development. Contributions welcome but expect breaking changes.

### Development Setup

1. Clone repository
2. Install dependencies: `npm install`
3. Build program (see Building section)
4. Run tests: `anchor test`

## License

MIT

## Disclaimer

This software is experimental and unaudited. Do not use with real funds. No warranties provided.

## Contact

GitHub: https://github.com/psolprotocol/psol-v1  
X/Twitter: https://x.com/psolprotocol

---

**Last Updated:** December 2024  
**Program Version:** 1.0.0  
**Status:** Development/Testing