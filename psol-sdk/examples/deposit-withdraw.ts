/**
 * Complete Deposit and Withdraw Example
 * 
 * This example demonstrates the full flow:
 * 1. Generate a deposit note
 * 2. Execute deposit
 * 3. Save note securely
 * 4. Generate withdrawal proof
 * 5. Execute withdrawal
 */

import { Connection, PublicKey, Keypair } from '@solana/web3.js';
import { Wallet } from '@coral-xyz/anchor';
import BN from 'bn.js';
import * as fs from 'fs';

import {
  PsolClient,
  createDepositNote,
  serializeNote,
  parseNote,
  noteFromParsed,
  computeNullifierHash,
  MerkleTree,
  generateWithdrawProof,
  derivePoolConfig,
  isNullifierSpent,
  PsolError,
  PsolErrorCode,
  PROGRAM_ID,
} from '@psol/sdk';

// Configuration
const RPC_URL = 'https://api.devnet.solana.com';
const TOKEN_MINT = new PublicKey('So11111111111111111111111111111111111111112'); // Wrapped SOL
const DEPOSIT_AMOUNT = new BN(100_000_000); // 0.1 SOL

// Circuit files (you need these from circom compilation)
const WASM_PATH = './circuits/withdraw.wasm';
const ZKEY_PATH = './circuits/withdraw_final.zkey';

async function main() {
  console.log('=== pSol Privacy Pool Demo ===\n');

  // 1. Setup connection and wallet
  const connection = new Connection(RPC_URL, 'confirmed');
  const keypair = Keypair.generate(); // In production, load from file
  const wallet = new Wallet(keypair);

  console.log('Wallet:', wallet.publicKey.toBase58());

  // 2. Create client
  const client = new PsolClient(connection);
  client.connect(wallet);

  // 3. Check pool status
  const status = await client.getPoolStatus(TOKEN_MINT);
  if (!status) {
    console.error('Pool not found for token mint');
    return;
  }

  console.log('\n--- Pool Status ---');
  console.log('Pool Address:', status.address.toBase58());
  console.log('Is Paused:', status.isPaused);
  console.log('VK Configured:', status.vkConfigured);
  console.log('VK Locked:', status.vkLocked);
  console.log('Total Deposits:', status.totalDeposits);
  console.log('Tree Utilization:', (status.treeUtilization * 100).toFixed(2) + '%');

  if (status.isPaused) {
    console.error('Pool is paused, cannot proceed');
    return;
  }

  // 4. Generate deposit note
  console.log('\n--- Generating Deposit Note ---');
  const note = await createDepositNote(
    status.address,
    TOKEN_MINT,
    DEPOSIT_AMOUNT
  );

  console.log('Secret (first 8 bytes):', Buffer.from(note.secret.slice(0, 8)).toString('hex'));
  console.log('Nullifier (first 8 bytes):', Buffer.from(note.nullifier.slice(0, 8)).toString('hex'));
  console.log('Commitment:', Buffer.from(note.commitment).toString('hex'));
  console.log('Amount:', note.amount.toString());

  // 5. Serialize note for storage
  const serializedNote = serializeNote(note);
  console.log('\n--- Serialized Note (SAVE THIS!) ---');
  console.log(serializedNote);

  // Save to file in development
  fs.writeFileSync('note.txt', serializedNote);
  console.log('Note saved to note.txt');

  // 6. Execute deposit
  console.log('\n--- Executing Deposit ---');
  try {
    const { signature, note: depositedNote } = await client.deposit(note);
    console.log('Deposit TX:', signature);
    console.log('Leaf Index:', depositedNote.leafIndex);

    // Save updated note with leaf index
    const finalNote = serializeNote(depositedNote);
    fs.writeFileSync('note-deposited.txt', finalNote);
    console.log('Updated note saved to note-deposited.txt');
  } catch (error) {
    if (error instanceof PsolError) {
      console.error('Deposit failed:', error.code, error.message);
    } else {
      console.error('Deposit failed:', error);
    }
    return;
  }

  // 7. Wait before withdrawal (in real use, this could be days/weeks later)
  console.log('\n--- Waiting before withdrawal ---');
  await sleep(5000);

  // 8. Load note for withdrawal
  console.log('\n--- Preparing Withdrawal ---');
  const loadedNote = fs.readFileSync('note-deposited.txt', 'utf-8');
  const parsed = await parseNote(loadedNote);
  const withdrawNote = await noteFromParsed(parsed);

  // 9. Check if can withdraw
  const canWithdraw = await client.canWithdraw(withdrawNote);
  if (!canWithdraw) {
    console.error('Cannot withdraw - note may already be spent');
    return;
  }
  console.log('Note can be withdrawn ✓');

  // 10. Generate withdrawal proof
  console.log('\n--- Generating ZK Proof ---');
  const recipient = wallet.publicKey; // Withdraw to self
  const relayer = wallet.publicKey; // Self-relay
  const relayerFee = new BN(0);

  try {
    const proof = await client.generateWithdrawProof(
      withdrawNote,
      recipient,
      relayer,
      relayerFee,
      WASM_PATH,
      ZKEY_PATH
    );
    console.log('Proof generated ✓');
    console.log('Proof size:', proof.proofData.length, 'bytes');

    // 11. Execute withdrawal
    console.log('\n--- Executing Withdrawal ---');
    const result = await client.withdraw(proof, withdrawNote.pool);
    console.log('Withdrawal TX:', result.signature);
    console.log('Slot:', result.slot);

  } catch (error) {
    if (error instanceof PsolError) {
      switch (error.code) {
        case PsolErrorCode.InvalidProof:
          console.error('ZK proof verification failed');
          break;
        case PsolErrorCode.NullifierAlreadySpent:
          console.error('Note already withdrawn');
          break;
        case PsolErrorCode.InvalidMerkleRoot:
          console.error('Merkle root not in history');
          break;
        default:
          console.error('Withdrawal failed:', error.message);
      }
    } else {
      console.error('Withdrawal failed:', error);
    }
    return;
  }

  // 12. Verify note is now spent
  console.log('\n--- Verifying Note is Spent ---');
  const nullifierHash = await computeNullifierHash(withdrawNote);
  const isSpent = await isNullifierSpent(
    connection,
    withdrawNote.pool,
    nullifierHash,
    PROGRAM_ID
  );
  console.log('Nullifier spent:', isSpent);

  console.log('\n=== Demo Complete ===');
}

function sleep(ms: number): Promise<void> {
  return new Promise(resolve => setTimeout(resolve, ms));
}

main().catch(console.error);
