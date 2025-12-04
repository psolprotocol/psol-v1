# pSOL Privacy Pool - Security Audit Report

**Date:** December 2024  
**Version:** 1.0.0 (Phase 4)  
**Program ID:** `7kK3aVXN9nTv1dNubmr85FB85fK6PeRrDBsisu9Z4gQ9`  
**Auditor:** Claude AI Security Review

---

## Executive Summary

pSOL is a Tornado Cash-inspired privacy pool for Solana using Groth16 zero-knowledge proofs. The protocol allows users to deposit SPL tokens, receive a cryptographic note, and later withdraw to any address without on-chain linkage.

**Overall Assessment:** The codebase demonstrates strong security practices with well-structured Anchor code, proper error handling, and comprehensive validation. However, several issues require attention before mainnet deployment.

| Severity | Count |
|----------|-------|
| 🔴 Critical | 2 |
| 🟠 High | 3 |
| 🟡 Medium | 4 |
| 🔵 Low | 5 |
| ℹ️ Informational | 4 |

---

## Critical Findings

### C-1: Missing ZK Circuit Files

**Severity:** 🔴 Critical  
**Location:** Project root  
**Status:** ⏳ Action Required

**Description:**  
The protocol requires a Groth16 circuit for withdrawal verification, but no circuit files are present in the repository:
- `withdraw.circom` - Circuit definition
- `withdraw.wasm` - Compiled circuit
- `withdraw_final.zkey` - Proving key
- `verification_key.json` - Verification key

**Impact:**  
Without the circuit, withdrawals cannot be verified. The protocol is non-functional.

**Recommendation:**  
1. Develop and audit the withdrawal circuit
2. Perform a trusted setup ceremony
3. Include circuit files in deployment package
4. Ensure circuit parameters match on-chain verification

---

### C-2: Hash Function Mismatch Risk

**Severity:** 🔴 Critical  
**Location:** `crypto/poseidon.rs`, `state/merkle_tree.rs`  
**Status:** ⚠️ Needs Verification

**Description:**  
The on-chain Merkle tree uses Keccak256 (`hash_two_to_one`), while commitments are expected to use Poseidon. The circuit must correctly handle this hybrid approach.

```rust
// poseidon.rs:51-56
pub fn hash_two_to_one(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let mut combined = [0u8; 64];
    combined[..32].copy_from_slice(left);
    combined[32..].copy_from_slice(right);
    keccak::hash(&combined).to_bytes()
}
```

**Impact:**  
If the circuit uses different hash functions, proofs will fail verification.

**Recommendation:**  
1. Document hash function requirements clearly
2. Ensure circuit uses Keccak256 for Merkle path
3. Ensure circuit uses Poseidon for commitment/nullifier
4. Add integration tests verifying hash consistency

---

## High Severity

### H-1: Relayer Address Bug in Transaction Builder

**Severity:** 🟠 High  
**Location:** `services/psol-relayer/src/transaction.ts:142-143`  
**Status:** 🐛 Bug

**Description:**  
In `buildWithdrawInstructionData`, the relayer address is incorrectly set to the recipient address:

```typescript
// Line 142-143 (INCORRECT)
request.recipient.toBuffer().copy(buffer, offset); // Will be overwritten below
offset += 32;
```

This bug is in a deprecated function but could cause confusion.

**Recommendation:**  
The correct implementation exists in `buildWithdrawInstructionDataFull`. Remove or fix the deprecated function.

---

### H-2: No Minimum Relayer Fee Enforcement

**Severity:** 🟠 High  
**Location:** `instructions/withdraw.rs`  
**Status:** ⚠️ Missing Check

**Description:**  
The withdrawal instruction accepts any `relayer_fee` value, including zero. This allows:
1. Front-running by submitting zero-fee transactions
2. Relayer economic attacks
3. Denial of service to honest relayers

**Impact:**  
MEV bots can extract value from the relayer network.

**Recommendation:**  
Add minimum fee validation:
```rust
const MIN_RELAYER_FEE_BPS: u64 = 10; // 0.1% minimum
require!(
    relayer_fee >= amount.checked_mul(MIN_RELAYER_FEE_BPS).unwrap_or(0) / 10000,
    PrivacyError::RelayerFeeTooLow
);
```

---

### H-3: VK Not Auto-Locked After Production Deploy

**Severity:** 🟠 High  
**Location:** `instructions/set_verification_key.rs`  
**Status:** ℹ️ Operational Risk

**Description:**  
The verification key must be manually locked after setting. If forgotten, an attacker who compromises the authority key could:
1. Update the VK to accept invalid proofs
2. Drain the entire pool

**Recommendation:**  
1. Add a deployment checklist step for VK locking
2. Consider auto-locking after first successful withdrawal
3. Add monitoring alerts if VK is set but not locked

---

## Medium Severity

### M-1: Root History Circular Buffer Race Condition

**Severity:** 🟡 Medium  
**Location:** `state/merkle_tree.rs:198-199`  
**Status:** ⚠️ Edge Case

**Description:**  
With `root_history_size = 100`, heavy deposit activity could expire a user's proof root before they submit their withdrawal:

```rust
self.root_history_index = (self.root_history_index + 1) % self.root_history_size;
self.root_history[self.root_history_index as usize] = current_hash;
```

100 deposits in quick succession would invalidate proofs generated against older roots.

**Recommendation:**  
1. Increase default `root_history_size` to 500+
2. Add timestamp-based expiry as secondary check
3. Document root expiry behavior in user docs

---

### M-2: No Deposit Amount Normalization

**Severity:** 🟡 Medium  
**Location:** `instructions/deposit.rs`  
**Status:** 📊 Privacy Impact

**Description:**  
Unlike Tornado Cash fixed denominations, pSOL accepts any amount. Variable amounts reduce the anonymity set significantly.

**Example:**  
If Alice deposits 1.234567 tokens, and only one withdrawal of ~1.23 tokens occurs, the linkage is trivial.

**Recommendation:**  
1. Implement fixed deposit denominations (0.1, 1, 10, 100 tokens)
2. Or document privacy implications clearly
3. Add optional denomination enforcement flag

---

### M-3: Missing Finalization Check on Deposits

**Severity:** 🟡 Medium  
**Location:** `instructions/deposit.rs`  
**Status:** ⚠️ Reorg Risk

**Description:**  
Deposits are immediately added to the Merkle tree without waiting for finalization. In case of a chain reorg:
1. Deposit could be reverted
2. Commitment remains in tree
3. User cannot withdraw (no matching deposit)

**Recommendation:**  
1. Add finalization delay (32 slots) before inserting commitment
2. Or implement commitment queue with slot-based insertion
3. Document risk in user guides

---

### M-4: `MAX_RELAYER_FEE_BPS` Constant Unused

**Severity:** 🟡 Medium  
**Location:** `instructions/withdraw.rs:15`  
**Status:** 🐛 Dead Code

**Description:**  
```rust
pub const MAX_RELAYER_FEE_BPS: u64 = 1000; // Defined but never used
```

**Recommendation:**  
Enforce maximum fee:
```rust
require!(
    relayer_fee <= amount.checked_mul(MAX_RELAYER_FEE_BPS).unwrap_or(u64::MAX) / 10000,
    PrivacyError::RelayerFeeExcessive
);
```

---

## Low Severity

### L-1: PoolConfig Space Calculation Off by 3

**Severity:** 🔵 Low  
**Location:** `state/pool_config.rs:68`  
**Status:** 🔢 Minor Bug

**Description:**  
The `LEN` constant calculation appears to have incorrect padding:
```rust
pub const LEN: usize = 8 + 32 + 32 + 32 + 32 + 32 + 32 + 1 + 1 + 1 + 1 + 1 + 3 + 8 + 8 + 8 + 8 + 1 + 64;
//                                                                         ^^^
// The +3 seems to be manual padding that should be validated
```

**Recommendation:**  
Use Anchor's `#[account]` automatic sizing or add explicit padding field.

---

### L-2: `private_transfer` is a Non-Functional Stub

**Severity:** 🔵 Low  
**Location:** `lib.rs:74-81`, `instructions/private_transfer.rs`  
**Status:** ⏳ Incomplete

**Description:**  
The `private_transfer` instruction exists but does nothing:
```rust
pub fn private_transfer(
    ctx: Context<PrivateTransfer>,
    _input_nullifiers: Vec<[u8; 32]>,
    _output_commitments: Vec<[u8; 32]>,
    _proof_data: Vec<u8>,
) -> Result<()> {
    instructions::private_transfer::handler(ctx) // Returns Ok(())
}
```

**Recommendation:**  
Remove the stub or implement fully. Leaving it creates false expectations.

---

### L-3: No Input Length Limits on VK IC Vector

**Severity:** 🔵 Low  
**Location:** `instructions/set_verification_key.rs`  
**Status:** ⚠️ DoS Vector

**Description:**  
The `vk_ic` parameter is unbounded:
```rust
vk_ic: Vec<[u8; 64]>
```

A malicious authority could pass an extremely large vector, causing compute budget exhaustion.

**Recommendation:**  
```rust
require!(vk_ic.len() <= 20, PrivacyError::InputTooLarge);
```

---

### L-4: Missing Event for Authority Transfer Initiation

**Severity:** 🔵 Low  
**Location:** `instructions/admin/update_authority.rs`  
**Status:** 📝 Missing Event

**Description:**  
The `initiate_authority_transfer` function doesn't emit an event, making it harder to monitor for potential unauthorized access.

**Recommendation:**  
Add `AuthorityTransferInitiated` event.

---

### L-5: Inconsistent Error Messages

**Severity:** 🔵 Low  
**Location:** `error.rs`  
**Status:** 📝 UX Issue

**Description:**  
Some error messages don't provide enough context:
- `InvalidProof` - Doesn't specify which check failed
- `Unauthorized` - Doesn't say who the expected authority was

**Recommendation:**  
Enhance error messages with more details.

---

## Informational

### I-1: Good Use of Checked Arithmetic ✅

The codebase consistently uses `checked_add`, `checked_sub`, etc., preventing integer overflow vulnerabilities.

### I-2: Excellent 2-Step Authority Transfer ✅

The `initiate` → `accept` pattern for authority transfer is a security best practice that prevents accidental lockout.

### I-3: VK Locking Mechanism Well Designed ✅

The irreversible VK lock prevents post-deployment tampering.

### I-4: Comprehensive Error Codes ✅

Error codes 6000-6032 provide good coverage of failure modes.

---

## Recommendations Summary

### Pre-Mainnet Checklist

1. ✅ Develop and audit ZK withdrawal circuit
2. ✅ Perform trusted setup with multi-party ceremony
3. ✅ Fix relayer address bug in transaction builder
4. ✅ Add minimum/maximum relayer fee enforcement
5. ✅ Increase root history size to 500+
6. ✅ Remove or implement `private_transfer`
7. ✅ Add input length limits to VK IC vector
8. ✅ Document hash function requirements
9. ✅ Add authority transfer events
10. ✅ Create operational runbook for VK locking

### Deployment Steps

1. Deploy to devnet
2. Set verification key from trusted setup
3. Verify withdrawals work with test proofs
4. Lock verification key
5. Monitor for 1 week
6. Deploy to mainnet
7. Immediately lock VK
8. Enable monitoring and alerts

---

## Disclaimer

This security review was conducted based on the provided source code snapshot. It does not guarantee the absence of vulnerabilities. The ZK circuit (not provided) requires separate audit. Smart contract audits should be performed by multiple independent parties before mainnet deployment.

---

**End of Report**
