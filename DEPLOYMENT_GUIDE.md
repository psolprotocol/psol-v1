# pSOL Privacy Pool - Deployment Guide

## 🔐 Overview

pSOL is a privacy-preserving token pool for Solana using zero-knowledge proofs (Groth16). Users can deposit tokens and later withdraw them to a different address without on-chain linkage between deposit and withdrawal.

## 📋 Prerequisites

1. **Rust** (1.75+): `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
2. **Solana CLI** (1.18+): `sh -c "$(curl -sSfL https://release.solana.com/v1.18.18/install)"`
3. **Anchor** (0.30.1): `cargo install --git https://github.com/coral-xyz/anchor avm --locked && avm install 0.30.1 && avm use 0.30.1`
4. **Node.js** (18+) and npm

## 🚀 Quick Start

### 1. Build the Program

```bash
# Clone and enter project
cd psol-v1-main

# Build Anchor program
anchor build

# View program ID
solana-keygen pubkey target/deploy/psol_privacy-keypair.json
```

### 2. Update Program ID

After first build, update the program ID in:
- `programs/psol-privacy/src/lib.rs`: `declare_id!("YOUR_PROGRAM_ID")`
- `Anchor.toml`: `psol_privacy = "YOUR_PROGRAM_ID"`

Then rebuild:
```bash
anchor build
```

### 3. Generate IDL

```bash
# IDL is auto-generated in target/idl/psol_privacy.json
mkdir -p psol-sdk/src/idl
cp target/idl/psol_privacy.json psol-sdk/src/idl/
```

### 4. Deploy to Devnet

```bash
# Configure for devnet
solana config set --url devnet

# Create/use keypair
solana-keygen new  # or use existing

# Get devnet SOL
solana airdrop 2

# Deploy
anchor deploy --provider.cluster devnet
```

## 🧪 Testing

### Run Integration Tests

```bash
# Install dependencies
npm install

# Run tests on localnet
anchor test

# Or on devnet
anchor test --provider.cluster devnet
```

### Test Specific Features

```bash
# Run specific test file
anchor test tests/psol-privacy.ts
```

## 📦 SDK Usage

### Installation

```bash
cd psol-sdk
npm install
npm run build
```

### Basic Usage

```typescript
import { PsolClient, generateNote, serializeNote } from 'psol-sdk';
import { Connection, Keypair } from '@solana/web3.js';
import { BN } from 'bn.js';

// Initialize client
const connection = new Connection('https://api.devnet.solana.com');
const wallet = Keypair.generate(); // or load from file
const client = new PsolClient(connection, wallet);

// Generate deposit note
const amount = new BN(1_000_000_000); // 1 token
const note = generateNote(amount);

// Deposit
const { signature } = await client.deposit(tokenMint, amount, note);
console.log('Deposit tx:', signature);

// Save note securely (needed for withdrawal)
const noteJson = serializeNote(note);
console.log('Save this note:', noteJson);
```

## 🔄 Relayer Setup

### Configure Relayer

```bash
# Make scripts executable
chmod +x scripts/*.sh

# Setup relayer
./scripts/relayer.sh setup

# Edit configuration
nano services/psol-relayer/.env

# Run relayer
./scripts/relayer.sh run
```

### Docker Deployment

```bash
cd services/psol-relayer
docker-compose up -d
```

### Relayer API Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/health` | GET | Health check |
| `/status` | GET | Relayer status |
| `/fee` | GET | Current fee info |
| `/withdraw` | POST | Submit withdrawal |
| `/job/:id` | GET | Check job status |

## ⚙️ Pool Administration

### Initialize Pool

```typescript
const client = new PsolClient(connection, adminKeypair);

// Initialize pool for a token
await client.initializePool(
  tokenMint,
  20,  // tree depth (2^20 = ~1M deposits)
  100  // root history size
);
```

### Set Verification Key

```typescript
// Load verification key from trusted setup
const vkeyJson = await fs.readFile('circuits/withdraw/verification_key.json');
const vkey = vkeyJsonToOnChain(JSON.parse(vkeyJson));

// Set VK on-chain
await program.methods
  .setVerificationKey(
    Array.from(vkey.alphaG1),
    Array.from(vkey.betaG2),
    Array.from(vkey.gammaG2),
    Array.from(vkey.deltaG2),
    vkey.ic.map(ic => Array.from(ic))
  )
  .accounts({
    authority: admin.publicKey,
    poolConfig,
    verificationKey,
  })
  .signers([admin])
  .rpc();

// Lock VK (irreversible!)
await program.methods
  .lockVerificationKey()
  .accounts({
    authority: admin.publicKey,
    poolConfig,
    verificationKey,
  })
  .signers([admin])
  .rpc();
```

### Emergency Controls

```typescript
// Pause pool
await client.pausePool(tokenMint);

// Unpause pool
await client.unpausePool(tokenMint);

// Transfer authority (2-step)
await program.methods.initiateAuthorityTransfer(newAuthority.publicKey)...
await program.methods.acceptAuthorityTransfer()...  // by new authority
```

## 🔑 ZK Circuit Setup

### Required Circuit Files

The protocol requires a Groth16 circuit for withdrawals:

```
circuits/
├── withdraw.circom      # Circuit definition
├── withdraw.wasm        # Compiled circuit
├── withdraw_final.zkey  # Proving key
└── verification_key.json # Verification key
```

### Circuit Parameters

- **Field**: BN254 scalar field
- **Hash**: Poseidon (circomlib compatible)
- **Public Inputs**: merkleRoot, nullifierHash, recipient, amount, relayer, relayerFee

### Generate Circuit (requires circom/snarkjs)

```bash
# Install circom
cargo install circom

# Compile circuit
circom circuits/withdraw.circom --r1cs --wasm --sym

# Powers of Tau ceremony (or use existing)
snarkjs powersoftau new bn128 14 pot14_0000.ptau

# Phase 2
snarkjs groth16 setup withdraw.r1cs pot14_final.ptau withdraw_0000.zkey

# Export verification key
snarkjs zkey export verificationkey withdraw_final.zkey verification_key.json
```

## 📊 Monitoring

### Pool Statistics

```typescript
const poolInfo = await client.getPoolInfo(tokenMint);
console.log('Total deposits:', poolInfo.totalDeposits.toString());
console.log('Total value locked:', poolInfo.totalValueDeposited.sub(poolInfo.totalValueWithdrawn).toString());
```

### Events

Subscribe to pool events:
- `DepositEvent`: commitment, leafIndex, amount, timestamp
- `WithdrawEvent`: nullifierHash, recipient, amount, relayer, relayerFee

## 🛡️ Security Considerations

1. **Never share notes**: They contain secrets for withdrawal
2. **Lock VK in production**: Prevents verification key changes
3. **Use 2-step authority transfer**: Prevents accidental lockout
4. **Wait for finality**: Don't rely on unconfirmed deposits
5. **Verify circuit**: Audit ZK circuit before mainnet

## 📁 Project Structure

```
psol-v1-main/
├── programs/psol-privacy/    # Anchor program
│   └── src/
│       ├── lib.rs            # Entry point
│       ├── instructions/     # Instruction handlers
│       ├── state/            # Account structures
│       ├── crypto/           # ZK verification
│       └── error.rs          # Error codes
├── psol-sdk/                 # TypeScript SDK
│   └── src/
│       ├── client/           # Client implementation
│       ├── crypto/           # Proof generation
│       └── types/            # Type definitions
├── services/psol-relayer/    # Relayer service
├── tests/                    # Integration tests
└── scripts/                  # Build/deploy scripts
```

## 🐛 Troubleshooting

### Common Issues

1. **"Program not found"**: Deploy program first
2. **"Verification key not set"**: Set and lock VK before withdrawals
3. **"Invalid proof"**: Check circuit compatibility and proof generation
4. **"Nullifier already spent"**: Note was already used

### Debug Commands

```bash
# Check program deployment
solana program show YOUR_PROGRAM_ID

# View account data
anchor account psol_privacy.PoolConfig YOUR_POOL_CONFIG_ADDRESS

# Check logs
solana logs YOUR_PROGRAM_ID
```

## 📄 License

MIT License - see LICENSE file

## 🤝 Support

- Documentation: [docs.psol.dev](https://docs.psol.dev)
- Issues: GitHub Issues
- Security: security@psol.dev
