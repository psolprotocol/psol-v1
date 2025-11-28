# pSol Phase 3 - GitHub Implementation Guide

Step-by-step instructions to upgrade your repository from Phase 2.5 to Phase 3.

---

## Prerequisites

- Git installed
- Your psol-v1 repository cloned
- Solana CLI configured
- ~5 SOL in your wallet (devnet/testnet)

---

## Step 1: Create New Branch

```bash
cd /workspaces/psol-v1
git checkout -b phase-3

# Or if you want to start fresh from main:
git checkout main
git pull
git checkout -b phase-3
```

---

## Step 2: Download Phase 3 Files

Download `psol-phase3-complete.zip` from Claude and extract:

```bash
# Upload the zip to your codespace or local machine
# Then extract:
unzip psol-phase3-complete.zip -d phase3-temp

# Copy files over (this will overwrite Phase 2.5 files)
cp -r phase3-temp/programs/psol-privacy/src/* programs/psol-privacy/src/
cp phase3-temp/programs/psol-privacy/Cargo.toml programs/psol-privacy/Cargo.toml
cp phase3-temp/Cargo.toml Cargo.toml
cp phase3-temp/Anchor.toml Anchor.toml
cp phase3-temp/README.md README.md
cp phase3-temp/MIGRATION_GUIDE.md MIGRATION_GUIDE.md

# Clean up
rm -rf phase3-temp
rm psol-phase3-complete.zip
```

---

## Step 3: Verify File Structure

```bash
tree programs/psol-privacy/src/
```

Expected output:
```
programs/psol-privacy/src/
├── crypto/
│   ├── curve_utils.rs
│   ├── groth16_verifier.rs
│   ├── mod.rs
│   ├── poseidon.rs
│   └── public_inputs.rs
├── error.rs
├── events.rs
├── instructions/
│   ├── admin/
│   │   ├── mod.rs
│   │   ├── pause.rs
│   │   ├── unpause.rs
│   │   └── update_authority.rs
│   ├── deposit.rs
│   ├── initialize_pool.rs
│   ├── mod.rs
│   ├── private_transfer.rs
│   ├── set_verification_key.rs
│   └── withdraw.rs
├── lib.rs
├── state/
│   ├── merkle_tree.rs
│   ├── mod.rs
│   ├── pool_config.rs
│   ├── spent_nullifier.rs
│   └── verification_key.rs
└── tests.rs
```

---

## Step 4: Build

```bash
cargo build-sbf
```

Expected output:
```
   Compiling psol-privacy v1.0.0
    Finished release [optimized] target(s)
```

**Warnings are OK**, errors are not.

---

## Step 5: Test Build

If you have Rust tests:
```bash
cargo test --lib
```

---

## Step 6: Deploy (Upgrade)

```bash
# Check your balance
solana balance

# If needed, get more devnet SOL
solana airdrop 2

# Deploy (upgrade existing program)
solana program deploy target/deploy/psol_privacy.so \
  --program-id Ddokrq1M6hT9Vu63k4JWqVRSecyLeotNf8xKknKfRwvZ
```

**Note**: Use `--program-id` to upgrade the existing deployed program.

---

## Step 7: Commit and Push

```bash
git add .
git commit -m "Upgrade to Phase 3 - Production ZK implementation

Breaking changes:
- Deposit now takes pre-computed commitment (off-chain Poseidon)
- Private transfer fully implemented (2-in-2-out)
- Removed dev-mode bypass
- Full VK curve validation

See MIGRATION_GUIDE.md for details."

git push origin phase-3
```

---

## Step 8: Create Pull Request

1. Go to your GitHub repository
2. Click "Compare & pull request" for `phase-3` branch
3. Title: "Phase 3: Production ZK Implementation"
4. Add description from MIGRATION_GUIDE.md
5. Request review if working with a team

---

## Step 9: Update Client Code (if applicable)

If you have TypeScript/JavaScript clients:

### Old Deposit (Phase 2.5):
```javascript
await program.methods
  .deposit(amount, Array.from(secret), Array.from(nullifierPreimage))
  .accounts({...})
  .rpc();
```

### New Deposit (Phase 3):
```javascript
// Install circomlib: npm install circomlib
const { poseidon } = require('circomlib');

// Compute commitment OFF-CHAIN
const secret = crypto.randomBytes(32);
const nullifierPreimage = crypto.randomBytes(32);
const commitment = poseidon([
  BigInt('0x' + secret.toString('hex')),
  BigInt('0x' + nullifierPreimage.toString('hex')),
  BigInt(amount)
]);

// Convert to bytes
const commitmentBytes = Buffer.from(commitment.toString(16).padStart(64, '0'), 'hex');

// Deposit with pre-computed commitment
await program.methods
  .deposit(new BN(amount), Array.from(commitmentBytes))
  .accounts({...})
  .rpc();
```

---

## Troubleshooting

### Build Errors

1. **"unresolved import"**: Check file paths in mod.rs files
2. **"type mismatch"**: Ensure instruction signatures match lib.rs
3. **"stack overflow"**: Already fixed in Phase 3 (no on-chain Poseidon)

### Deployment Errors

1. **"insufficient funds"**: Get more SOL with `solana airdrop 2`
2. **"program already exists"**: Use `--program-id` flag
3. **"account already in use"**: Wait for previous tx to confirm

### Runtime Errors

1. **"InvalidProof"**: Proof doesn't match VK or public inputs
2. **"InvalidMerkleRoot"**: Root not in history (regenerate proof)
3. **"NullifierAlreadySpent"**: Trying to double-spend

---

## Verification Checklist

After deployment, verify:

- [ ] Program ID matches in Anchor.toml and lib.rs
- [ ] `cargo build-sbf` succeeds without errors
- [ ] `solana program show <PROGRAM_ID>` shows deployed program
- [ ] Initialize pool works
- [ ] Set verification key works
- [ ] Deposit with off-chain commitment works
- [ ] (If VK set) Withdrawal with valid proof works

---

## Next Steps

1. **Generate ZK Circuit**: Use circom to create withdrawal/transfer circuits
2. **Trusted Setup**: Perform powers-of-tau ceremony for production
3. **Client SDK**: Build TypeScript SDK for easy integration
4. **Frontend**: Create user interface for deposits/withdrawals
5. **Audit**: Get security audit before mainnet
