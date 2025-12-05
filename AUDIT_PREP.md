> [!NOTE]
> This file describes internal audit preparation steps.
> It is **not** a formal audit report or assurance statement.

# pSol Privacy Pool - Audit Preparation Document

**Version:** Phase 4  
**Program ID:** `Ddokrq1M6hT9Vu63k4JWqVRSecyLeotNf8xKknKfRwvZ`

---

## 1. Overview

pSol is a privacy-preserving token pool for Solana allowing users to deposit tokens and later withdraw them without on-chain linkage.

### Trust Assumptions
1. ZK Circuit correctness (separate audit)
2. Trusted setup was honest
3. BN254 curve security
4. Solana runtime behaves correctly

---

## 2. Critical Invariants

| ID | Invariant | Description |
|----|-----------|-------------|
| INV-1 | Token Conservation | `vault.amount == deposits - withdrawals` |
| INV-2 | No Double-Spend | Each nullifier used only once (PDA) |
| INV-3 | Merkle Append-Only | Leaves immutable after insertion |
| INV-4 | Valid Root Required | Proofs use recent root only |
| INV-5 | VK Lock Permanence | Once locked, VK immutable |
| INV-6 | Authority Non-Zero | Always valid authority |

---

## 3. Account Structure

| Account | PDA Seeds | Purpose |
|---------|-----------|---------|
| PoolConfig | `["pool", token_mint]` | Pool state |
| MerkleTree | `["merkle_tree", pool]` | Commitments |
| VerificationKey | `["verification_key", pool]` | Groth16 VK |
| SpentNullifier | `["nullifier", pool, hash]` | Double-spend prevention |
| Vault | `["vault", pool]` | Token custody |

---

## 4. Security Controls

### Deposit
- Amount validation: `0 < amount <= MAX`
- Commitment non-zero check
- Tree capacity check

### Withdraw
- Proof verification (Groth16)
- Nullifier uniqueness (PDA init)
- Root freshness check
- Amount bounds

### Admin
- VK locking (irreversible)
- 2-step authority transfer
- Emergency pause

---

## 5. New in Phase 4

| Feature | Description |
|---------|-------------|
| VK Locking | `lock_verification_key()` - permanent |
| 2-Step Authority | `initiate` → `accept` pattern |
| New Error Codes | 6023-6032 for security |
| Bounds Checking | Max deposit, max fee |

---

## 6. Attack Surface

| Instruction | Risk | Key Checks |
|-------------|------|------------|
| set_verification_key | HIGH | VK lock check |
| withdraw | HIGH | Proof, nullifier, root |
| deposit | MEDIUM | Amount, commitment |
| initialize_pool | LOW | Single init per mint |

---

## 7. Known Limitations

1. Private transfer not implemented
2. Amounts visible in deposits
3. No fixed denominations
4. No compliance features

---

## 8. Error Codes

| Code | Name |
|------|------|
| 6000 | InvalidProof |
| 6004 | InvalidMerkleRoot |
| 6008 | NullifierAlreadySpent |
| 6017 | Unauthorized |
| 6023 | VerificationKeyLocked |
| 6024 | InvalidAuthority |
| 6025 | NoPendingAuthority |

---

## 9. File Map

```
src/
├── lib.rs
├── error.rs
├── events.rs
├── crypto/
│   ├── groth16_verifier.rs
│   ├── poseidon.rs
│   └── public_inputs.rs
├── instructions/
│   ├── deposit.rs
│   ├── withdraw.rs
│   ├── initialize_pool.rs
│   ├── set_verification_key.rs
│   └── admin/
│       ├── pause.rs
│       ├── unpause.rs
│       └── update_authority.rs
└── state/
    ├── pool_config.rs
    ├── merkle_tree.rs
    ├── verification_key.rs
    └── spent_nullifier.rs
```
