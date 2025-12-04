# pSOL Privacy Pool - AI Agent Coding Instructions

## Project Overview

**pSOL** is a production-ready zero-knowledge privacy pool for Solana enabling private deposits, transfers, and withdrawals of SPL tokens. It uses Groth16 proofs verified via Solana's alt_bn128 precompiles, with BN254 curve cryptography and Poseidon commitments. The system is designed to prevent double-spending through nullifier tracking and includes a privacy-preserving relayer service.

**Current State:** Phase 4 (Audit-Ready) on Solana devnet. Program ID: `7kK3aVXN9nTv1dNubmr85FB85fK6PeRrDBsisu9Z4gQ9`

## Architecture: Key Components

### 1. On-Chain Program (`programs/psol-privacy/src/`)

**Core System Accounts:**
- `PoolConfig`: Stores pool metadata, authority, token mint, and status flags (is_paused, vk_locked, vk_configured)
- `MerkleTree`: Maintains incremental Merkle tree of commitments with root history (prevents replay)
- `VerificationKey`: Stores Groth16 proof verification parameters (VK alpha/beta/gamma/delta + IC points)
- `Vault`: SPL token account holding all pooled tokens
- `SpentNullifier`: PDAs tracking spent nullifiers to prevent double-spend

**Critical Pattern:** All PDAs are derived with seeds like `[b"pool", token_mint]` or `[b"merkle_tree", pool_config]`. Always validate account ownership and constraints in Anchor structs.

### 2. Instruction Flow (in `instructions/mod.rs`)

1. **initialize_pool**: Creates pool infrastructure (one-time admin operation)
2. **set_verification_key**: Configures Groth16 VK (can be locked immutably)
3. **deposit**: Transfers tokens to vault, inserts commitment into Merkle tree
4. **private_transfer**: 2-in/2-out shielded transfer (burns input nullifiers, creates output commitments)
5. **withdraw**: Verifies proof, checks nullifier not spent, transfers tokens out via optional relayer

### 3. Cryptographic Foundation (`crypto/`)

**Groth16 Verification** (`groth16_verifier.rs`):
- Implements BN254 pairing check: `e(-A, B) · e(α, β) · e(vk_x, γ) · e(C, δ) = 1`
- Uses Solana precompiles for efficient elliptic curve operations
- **Security Critical:** Always validates curve points, rejects identity points, enforces in-range public inputs
- VK points must be validated on-chain before first use

**Merkle Tree** (`merkle_tree.rs`):
- Incremental tree for commitment storage (similar to Tornado Cash)
- Maintains root history (prevents replay, allows past-root validity windows)
- Uses Keccak256 hashing on-chain for efficiency

**Poseidon Hashing** (`poseidon.rs`):
- Off-chain commitments computed as: `commitment = Poseidon(secret, nullifier_preimage)`
- Nullifier hash: `nullifier_hash = Poseidon(secret)`
- Must use circomlib-compatible Poseidon (BN254 field prime modulus)

### 4. TypeScript SDK (`psol-sdk/src/`)

**Main Entry Points:**
- `PsolClient`: Connection manager for all pool operations
- `PsolProgram`: Low-level instruction builders
- Crypto helpers: Commitment/nullifier generation, proof serialization
- PDA utilities: Derives all program-derived addresses deterministically

**Note Structure** (see `utils/note.ts`):
```typescript
{
  secret: string,           // User's secret (keep private!)
  nullifier: string,        // Nullifier preimage
  amount: BN,              // Deposit amount
  commitment: string,      // Poseidon(secret, nullifier)
  nullifierHash: string,   // Poseidon(secret)
}
```

### 5. Relayer Service (`services/psol-relayer/`)

**Purpose:** Allows users to withdraw without linking their withdrawal address on-chain (breaks privacy otherwise).

**Architecture:**
- Express.js server with Nginx rate-limiting frontend
- BullMQ job queue with Redis persistence for durability
- Workers validate proofs (via snarkjs), check nullifier deduplication, submit transactions
- Charges small relayer fee from withdrawal amount

**Security:** Workers must reject duplicate nullifiers across all submitted proofs to prevent network spam.

## Critical Workflows & Commands

### Build & Deploy

```bash
# Build the Anchor program
anchor build

# Run test suite (localnet)
anchor test

# Deploy to devnet
solana config set --url devnet
anchor deploy --provider.cluster devnet

# Generate/update IDL (auto-generated in target/idl/)
mkdir -p psol-sdk/src/idl
cp target/idl/psol_privacy.json psol-sdk/src/idl/
```

### Testing

```bash
# Integration tests (TypeScript) - see tests/psol-privacy.ts
npm install && npm run test

# Debug single test (high timeout for Groth16 verification)
npm run ts-mocha -- -t 100000 tests/psol-privacy.ts --grep "deposit"
```

### SDK Usage Pattern

```typescript
// 1. Initialize client
const connection = new Connection(clusterApiUrl('devnet'));
const client = new PsolClient(connection);
client.connect(wallet);

// 2. Generate deposit note (OFF-CHAIN, secure storage critical)
const note = await client.generateDepositNote(tokenMint, amount);

// 3. Deposit via on-chain instruction
await client.deposit(tokenMint, amount, note.commitment);

// 4. Later: Generate withdrawal proof
const proof = await client.generateWithdrawProof(
  note,
  merkleProof,
  recipient,
  amount
);

// 5. Submit via relayer or directly
await relayerClient.submitWithdrawal(proof, fee);
```

## Project-Specific Patterns & Conventions

### 1. Error Handling (see `error.rs`)

```rust
// Always use PrivacyError enum with descriptive messages
#[error_code]
pub enum PrivacyError {
    #[msg("Invalid proof: verification failed")]
    InvalidProof,  // 6000
    #[msg("Nullifier already spent")]
    NullifierAlreadySpent,  // 6008
}

// Usage in constraints
#[account(constraint = vault.owner == pool_config.key() @ PrivacyError::Unauthorized)]
```

**Error Code Convention:** Custom codes start at 6000 for all PrivacyError variants.

### 2. Account Constraints Pattern

```rust
#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(mut, seeds = [b"pool", pool_config.token_mint.as_ref()], bump = pool_config.bump)]
    pub pool_config: Account<'info, PoolConfig>,

    #[account(mut, constraint = vault.mint == pool_config.token_mint @ PrivacyError::InvalidMint)]
    pub vault: Account<'info, TokenAccount>,
}
```

**Convention:** PDA derivation always uses specific seeds, bump is stored in config and validated. Constraints validate ownership and state before operations.

### 3. Two-Step Authority Transfer (security pattern)

```rust
// Step 1: Current authority calls propose_authority_transfer
// Step 2: Pending authority calls accept_authority_transfer
```

This pattern prevents accidental lock-out and is used throughout admin operations.

### 4. Verification Key Immutability

Once the VK is "locked" via `lock_verification_key()`, it cannot be updated. This is critical for production: prevents post-deployment circuit manipulation.

### 5. Anchor IDL Generation

After any instruction/struct changes:
```bash
anchor build  # Regenerates IDL
cp target/idl/psol_privacy.json psol-sdk/src/idl/
```

The SDK imports from `src/idl/psol_privacy.json`, always kept in sync.

## Integration Points & Data Flows

### Deposit Flow
```
User → Off-chain: Generate commitment = Poseidon(secret, nullifier)
     → On-chain: deposit(amount, commitment)
     → Merkle Tree: Insert commitment, emit DepositEvent
     → Vault: Receive tokens (custody via pool_config authority)
```

### Withdrawal Flow
```
User → Off-chain: Generate Groth16 proof (prove: commitment in tree ∧ not spent)
     → Relayer: Submit proof + recipient + fee
     → On-chain: Verify proof, check nullifier, mark spent, transfer tokens
```

### Private Transfer Flow (not fully implemented in Phase 4)
```
User → Off-chain: Prove 2 inputs spent, 2 outputs created, value conserved
     → On-chain: Verify proof, burn input nullifiers, add output commitments
```

## External Dependencies & Versions

- **Anchor:** 0.30.1 (locked in avm)
- **Solana CLI:** 1.18+
- **Rust:** 1.75+
- **Key crates:** borsh 1.5.1, anchor-lang, anchor-spl, alt_bn128 precompiles (built-in)
- **SDK deps:** @coral-xyz/anchor 0.30.1, @solana/web3.js 1.78.4, @solana/spl-token 0.4.3

**Precompile Interaction:** Groth16 verification calls Solana's native alt_bn128 precompile (program ID: `0x05`). No additional contracts needed.

## Deployment Considerations

### Before Mainnet

1. **Verify circuit files:** Ensure withdraw.circom, proving key, and verification key are audited
2. **Test on testnet:** Full integration test on Solana testnet before mainnet deployment
3. **Audit:** Security audit of Groth16 verifier and state management
4. **Relayer readiness:** Deploy relayer service with proper rate-limiting and monitoring
5. **Lock VK:** After first correct VK is set, call lock_verification_key() to prevent tampering

### Config Updates Needed

- Update `declare_id!()` in `programs/psol-privacy/src/lib.rs` after first devnet build
- Update `Anchor.toml` cluster and program ID mappings
- Update SDK `PROGRAM_ID` constant in `psol-sdk/src/client/psolClient.ts`

## Common Pitfalls for AI Agents

1. **Commitment vs Nullifier:** Commitment is Poseidon(secret, nullifier); nullifier hash is Poseidon(secret). Both are 32-byte arrays but serve different purposes.
2. **PDA Validation:** Always verify PDA bumps and seeds match. Mismatched seeds will cause "AccountNotFound" or cryptic constraint errors.
3. **Merkle Root History:** Withdrawals must use a root within the recent history window. Very old deposits cannot be withdrawn without tree history expansion.
4. **Relayer Fee:** Relayer fee is deducted FROM the withdrawal amount, not added on top. Verify `fee < amount` before approval.
5. **Proof Format:** Proofs are 256 bytes (serialized). Length validation happens on-chain; incorrect lengths are rejected silently.

## Reference Files

- **Core logic:** `programs/psol-privacy/src/{instructions,crypto,state}/*.rs`
- **Tests:** `tests/psol-privacy.ts` (comprehensive integration tests)
- **Examples:** `examples/complete-flow.ts` (full deposit→withdraw cycle)
- **SDK types:** `psol-sdk/src/types/index.ts`
- **Security docs:** `SECURITY_AUDIT.md`, `DEPLOYMENT_GUIDE.md`
