# @psol/sdk

TypeScript SDK for pSol Privacy Pool on Solana.

## Installation

```bash
npm install @psol/sdk
# or
yarn add @psol/sdk
# or
pnpm add @psol/sdk
```

## Quick Start

```typescript
import { Connection, Keypair } from '@solana/web3.js';
import { PsolClient, createDepositNote, serializeNote } from '@psol/sdk';
import BN from 'bn.js';

// Connect to Solana
const connection = new Connection('https://api.devnet.solana.com');
const wallet = /* your wallet */;

// Create client
const client = new PsolClient(connection);
client.connect(wallet);

// Check if pool exists
const tokenMint = new PublicKey('So11111111111111111111111111111111111111112');
const status = await client.getPoolStatus(tokenMint);
console.log('Pool TVL:', status?.totalValueLocked.toString());
```

## Deposit Flow

```typescript
import { 
  PsolClient, 
  createDepositNote, 
  serializeNote,
  generateCommitment 
} from '@psol/sdk';
import BN from 'bn.js';

// 1. Generate a deposit note (SAVE THIS SECURELY!)
const note = await client.generateDepositNote(
  tokenMint,
  new BN(1_000_000_000) // 1 token with 9 decimals
);

// 2. Serialize the note for storage
const serialized = serializeNote(note);
console.log('SAVE THIS NOTE:', serialized);
// Output: psol:v1:base64encodeddata...

// 3. Execute deposit
const { signature, note: updatedNote } = await client.deposit(note);
console.log('Deposit confirmed:', signature);
console.log('Leaf index:', updatedNote.leafIndex);

// 4. Save the updated note (now includes leaf index)
const finalNote = serializeNote(updatedNote);
// Store finalNote securely - you need it to withdraw!
```

## Withdraw Flow

```typescript
import { 
  parseNote, 
  noteFromParsed,
  computeNullifierHash 
} from '@psol/sdk';

// 1. Parse your saved note
const parsed = await parseNote(savedNoteString);
const note = await noteFromParsed(parsed);

// 2. Check if withdrawal is possible
const canWithdraw = await client.canWithdraw(note);
if (!canWithdraw) {
  throw new Error('Note already spent or not deposited');
}

// 3. Generate ZK proof (requires circuit files)
const recipient = new PublicKey('...');
const proof = await client.generateWithdrawProof(
  note,
  recipient,
  recipient, // relayer (self-relay)
  new BN(0), // no relayer fee
  './circuits/withdraw.wasm',
  './circuits/withdraw_final.zkey'
);

// 4. Execute withdrawal
const result = await client.withdraw(proof, note.pool);
console.log('Withdrawal confirmed:', result.signature);
```

## Admin Operations

### Initialize a Pool

```typescript
import { PsolProgram } from '@psol/sdk';

const program = new PsolProgram(connection, wallet);

await program.initializePool(tokenMint, {
  treeDepth: 20,      // 2^20 = 1M deposits
  rootHistorySize: 100 // Keep last 100 roots
});
```

### Set Verification Key

```typescript
import { vkeyJsonToOnChain } from '@psol/sdk';
import vkeyJson from './verification_key.json';

const vk = vkeyJsonToOnChain(vkeyJson);

await program.setVerificationKey(tokenMint, {
  alphaG1: vk.alphaG1,
  betaG2: vk.betaG2,
  gammaG2: vk.gammaG2,
  deltaG2: vk.deltaG2,
  ic: vk.ic,
});
```

### Lock Verification Key (Phase 4 - Irreversible!)

```typescript
// WARNING: This is permanent and cannot be undone!
await program.lockVerificationKey(tokenMint);
```

### Two-Step Authority Transfer (Phase 4)

```typescript
// Step 1: Current authority initiates transfer
await program.initiateAuthorityTransfer(tokenMint, newAuthorityPubkey);

// Step 2: New authority accepts
const newAuthorityProgram = new PsolProgram(connection, newAuthorityWallet);
await newAuthorityProgram.acceptAuthorityTransfer(tokenMint);

// Or cancel if wrong address
await program.cancelAuthorityTransfer(tokenMint);
```

### Pause/Unpause Pool

```typescript
await program.pausePool(tokenMint);
// ... handle emergency ...
await program.unpausePool(tokenMint);
```

## Cryptographic Utilities

### Generate Commitment Off-Chain

```typescript
import { 
  randomFieldElement, 
  generateCommitment,
  generateNullifierHash,
  poseidonHash 
} from '@psol/sdk';

// Generate random secret and nullifier
const secret = randomFieldElement();
const nullifier = randomFieldElement();

// Compute commitment: hash(secret, nullifier)
const commitment = await generateCommitment(secret, nullifier);

// After deposit, compute nullifier hash for withdrawal
const leafIndex = 42; // from deposit event
const nullifierHash = await generateNullifierHash(nullifier, leafIndex);
```

### Build Merkle Tree

```typescript
import { MerkleTree } from '@psol/sdk';

const tree = new MerkleTree(20); // depth 20
await tree.initialize();

// Insert leaves
const index = await tree.insert(commitment);

// Generate proof
const proof = await tree.generateProof(index);

// Verify proof
const isValid = await tree.verifyProof(commitment, proof);
```

### PDA Derivation

```typescript
import { 
  derivePoolConfig,
  deriveVault,
  deriveMerkleTree,
  deriveVerificationKey,
  deriveSpentNullifier,
  deriveAllPoolPDAs,
} from '@psol/sdk';

// Get all PDAs for a pool
const pdas = deriveAllPoolPDAs(tokenMint);
console.log('Pool:', pdas.poolConfig[0].toBase58());
console.log('Vault:', pdas.vault[0].toBase58());

// Check if nullifier is spent
const isSpent = await isNullifierSpent(
  connection,
  poolAddress,
  nullifierHash
);
```

## Note Management

```typescript
import {
  createDepositNote,
  serializeNote,
  parseNote,
  noteFromParsed,
  validateNote,
  noteToSummary,
  encryptNote,
  decryptNote,
} from '@psol/sdk';

// Create note
const note = await createDepositNote(pool, tokenMint, amount);

// Validate
validateNote(note); // throws if invalid

// Serialize for storage
const serialized = serializeNote(note);

// Display summary (safe to show)
console.log(noteToSummary(note));
// "pSol Note: 1000000000 tokens | Pool: Ddokrq1M... | Deposited (leaf #42)"

// Encrypt for secure storage
const encrypted = encryptNote(serialized, 'my-password');
const decrypted = decryptNote(encrypted, 'my-password');
```

## Error Handling

```typescript
import { PsolError, PsolErrorCode } from '@psol/sdk';

try {
  await client.withdraw(proof, poolAddress);
} catch (error) {
  if (error instanceof PsolError) {
    switch (error.code) {
      case PsolErrorCode.NullifierAlreadySpent:
        console.error('Note already withdrawn!');
        break;
      case PsolErrorCode.InvalidProof:
        console.error('ZK proof verification failed');
        break;
      case PsolErrorCode.PoolPaused:
        console.error('Pool is paused');
        break;
      default:
        console.error('Error:', error.message);
    }
  }
}
```

## Constants

```typescript
import { PSOL_CONSTANTS, PROGRAM_ID } from '@psol/sdk';

console.log('Program ID:', PROGRAM_ID.toBase58());
console.log('Max deposit:', PSOL_CONSTANTS.MAX_DEPOSIT_AMOUNT.toString());
console.log('Default tree depth:', PSOL_CONSTANTS.DEFAULT_TREE_DEPTH);
console.log('Field size:', PSOL_CONSTANTS.FIELD_SIZE.toString());
```

## Setup with Anchor IDL

To use the full program integration:

1. Build your Anchor program to generate the IDL:
   ```bash
   anchor build
   ```

2. Copy the IDL to your project:
   ```bash
   cp target/idl/psol_privacy.json sdk/src/idl/
   ```

3. Generate TypeScript types:
   ```bash
   anchor idl type target/idl/psol_privacy.json -o sdk/src/idl/
   ```

4. Update `src/client/program.ts` to import the IDL:
   ```typescript
   import { PsolPrivacy } from '../idl/psol_privacy';
   import IDL from '../idl/psol_privacy.json';
   ```

## Circuit Files

For ZK proof generation, you need:
- `withdraw.wasm` - Circuit compiled to WASM
- `withdraw_final.zkey` - Proving key from trusted setup

These are generated from the circom circuit (separate repository).

## API Reference

### PsolClient

| Method | Description |
|--------|-------------|
| `connect(wallet)` | Connect wallet to client |
| `getPool(tokenMint)` | Get pool configuration |
| `getPoolStatus(tokenMint)` | Get pool status for UI |
| `generateDepositNote(...)` | Create new deposit note |
| `deposit(note)` | Execute deposit transaction |
| `canWithdraw(note)` | Check if note can be withdrawn |
| `generateWithdrawProof(...)` | Generate ZK proof |
| `withdraw(proof, pool)` | Execute withdrawal |

### PsolProgram

| Method | Description |
|--------|-------------|
| `initializePool(...)` | Create new pool |
| `setVerificationKey(...)` | Set ZK verification key |
| `lockVerificationKey(...)` | Lock VK (irreversible) |
| `deposit(...)` | Deposit tokens |
| `withdraw(...)` | Withdraw with proof |
| `initiateAuthorityTransfer(...)` | Start authority transfer |
| `acceptAuthorityTransfer(...)` | Accept authority transfer |
| `cancelAuthorityTransfer(...)` | Cancel pending transfer |
| `pausePool(...)` | Pause pool |
| `unpausePool(...)` | Unpause pool |

## License

MIT

## Contributing

See [CONTRIBUTING.md](./CONTRIBUTING.md)

## Security

Found a vulnerability? Please report via [security@psolprotocol.xyz](mailto:security@psolprotocol.xyz)
