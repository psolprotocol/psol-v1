@psol/sdk

TypeScript SDK for interacting with the pSol Privacy Pool on Solana.
Supports note generation, deposit, withdrawal, Merkle tree operations, proof integration, and admin functions.

Installation
npm install @psol/sdk
# or
yarn add @psol/sdk
# or
pnpm add @psol/sdk

Quick Start
import { Connection, Keypair } from '@solana/web3.js';
import { PsolClient } from '@psol/sdk';
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

Deposit Flow
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

// 3. Execute deposit
const { signature, note: updatedNote } = await client.deposit(note);
console.log('Deposit confirmed:', signature);
console.log('Leaf index:', updatedNote.leafIndex);

// 4. Save updated note
const finalNote = serializeNote(updatedNote);

Withdraw Flow
import { 
  parseNote, 
  noteFromParsed,
  computeNullifierHash 
} from '@psol/sdk';

// 1. Parse saved note
const parsed = await parseNote(savedNoteString);
const note = await noteFromParsed(parsed);

// 2. Check if withdrawal is possible
const canWithdraw = await client.canWithdraw(note);
if (!canWithdraw) {
  throw new Error('Note already spent or not deposited');
}

// 3. Generate ZK proof
const recipient = new PublicKey('...');
const proof = await client.generateWithdrawProof(
  note,
  recipient,
  recipient,
  new BN(0),
  './circuits/withdraw.wasm',
  './circuits/withdraw_final.zkey'
);

// 4. Execute withdrawal
const result = await client.withdraw(proof, note.pool);
console.log('Withdrawal confirmed:', result.signature);

Admin Operations
Initialize a Pool
import { PsolProgram } from '@psol/sdk';

const program = new PsolProgram(connection, wallet);

await program.initializePool(tokenMint, {
  treeDepth: 20,
  rootHistorySize: 100
});

Set Verification Key
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

Lock Verification Key
await program.lockVerificationKey(tokenMint);

Two-Step Authority Transfer
await program.initiateAuthorityTransfer(tokenMint, newAuthorityPubkey);

const newAuthorityProgram = new PsolProgram(connection, newAuthorityWallet);
await newAuthorityProgram.acceptAuthorityTransfer(tokenMint);

// Or cancel
await program.cancelAuthorityTransfer(tokenMint);

Pause/Unpause Pool
await program.pausePool(tokenMint);
await program.unpausePool(tokenMint);

Cryptographic Utilities
Generate Commitment Off-Chain
import { 
  randomFieldElement, 
  generateCommitment,
  generateNullifierHash,
  poseidonHash 
} from '@psol/sdk';

const secret = randomFieldElement();
const nullifier = randomFieldElement();

const commitment = await generateCommitment(secret, nullifier);

const leafIndex = 42;
const nullifierHash = await generateNullifierHash(nullifier, leafIndex);

Merkle Tree
import { MerkleTree } from '@psol/sdk';

const tree = new MerkleTree(20);
await tree.initialize();

const index = await tree.insert(commitment);
const proof = await tree.generateProof(index);
const isValid = await tree.verifyProof(commitment, proof);

PDA Derivation
import { 
  derivePoolConfig,
  deriveVault,
  deriveMerkleTree,
  deriveVerificationKey,
  deriveSpentNullifier,
  deriveAllPoolPDAs,
} from '@psol/sdk';

const pdas = deriveAllPoolPDAs(tokenMint);
console.log('Pool:', pdas.poolConfig[0].toBase58());
console.log('Vault:', pdas.vault[0].toBase58());

Note Management
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

const note = await createDepositNote(pool, tokenMint, amount);

validateNote(note);

const serialized = serializeNote(note);

console.log(noteToSummary(note));

const encrypted = encryptNote(serialized, 'my-password');
const decrypted = decryptNote(encrypted, 'my-password');

Error Handling
import { PsolError, PsolErrorCode } from '@psol/sdk';

try {
  await client.withdraw(proof, poolAddress);
} catch (error) {
  if (error instanceof PsolError) {
    switch (error.code) {
      case PsolErrorCode.NullifierAlreadySpent:
        console.error('Note already withdrawn');
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

Constants
import { PSOL_CONSTANTS, PROGRAM_ID } from '@psol/sdk';

console.log('Program ID:', PROGRAM_ID.toBase58());
console.log('Max deposit:', PSOL_CONSTANTS.MAX_DEPOSIT_AMOUNT.toString());
console.log('Default tree depth:', PSOL_CONSTANTS.DEFAULT_TREE_DEPTH);
console.log('Field size:', PSOL_CONSTANTS.FIELD_SIZE.toString());

Integration with Anchor IDL

Build your Anchor program:

anchor build


Copy the IDL:

cp target/idl/psol_privacy.json sdk/src/idl/


Generate TypeScript types:

anchor idl type target/idl/psol_privacy.json -o sdk/src/idl/


Update client import:

import { PsolPrivacy } from '../idl/psol_privacy';
import IDL from '../idl/psol_privacy.json';

Circuit Files

For withdrawal proof generation, the SDK expects:

withdraw.wasm

withdraw_final.zkey

Generated separately from the circom circuits.

API Reference
PsolClient
Method	Description
connect(wallet)	Connect wallet
getPool(tokenMint)	Fetch pool configuration
getPoolStatus(tokenMint)	Return pool status
generateDepositNote(...)	Create note
deposit(note)	Execute deposit
canWithdraw(note)	Check spendability
generateWithdrawProof(...)	Create ZK proof
withdraw(proof, pool)	Execute withdrawal
PsolProgram
Method	Description
initializePool(...)	Create new pool
setVerificationKey(...)	Upload verifying key
lockVerificationKey(...)	Make VK permanent
deposit(...)	Deposit
withdraw(...)	Withdraw
initiateAuthorityTransfer(...)	Start transfer
acceptAuthorityTransfer(...)	Accept transfer
cancelAuthorityTransfer(...)	Cancel transfer
pausePool(...)	Pause
unpausePool(...)	Unpause
License

MIT

Contributing

See CONTRIBUTING.md.