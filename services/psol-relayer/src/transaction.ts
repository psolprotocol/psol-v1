/**
 * Transaction Builder
 * Builds and submits withdrawal transactions
 * 
 * Based on pSol program withdraw instruction:
 * - Account order matches Withdraw struct in withdraw.rs
 * - Instruction data uses Anchor's borsh serialization
 */

import {
  Connection,
  PublicKey,
  Transaction,
  TransactionInstruction,
  Keypair,
  sendAndConfirmTransaction,
  ComputeBudgetProgram,
  LAMPORTS_PER_SOL,
  SystemProgram,
} from '@solana/web3.js';
import {
  TOKEN_PROGRAM_ID,
  getAssociatedTokenAddress,
  createAssociatedTokenAccountInstruction,
  getAccount,
} from '@solana/spl-token';
import BN from 'bn.js';
import { createHash } from 'crypto';
import { logger, logTransaction, truncateHash } from './logger.js';
import type { WithdrawalRequestValidated, RelayerConfig } from './types.js';

/**
 * Anchor instruction discriminator: first 8 bytes of sha256("global:<instruction_name>")
 */
function getInstructionDiscriminator(name: string): Buffer {
  const hash = createHash('sha256').update(`global:${name}`).digest();
  return hash.slice(0, 8);
}

const WITHDRAW_DISCRIMINATOR = getInstructionDiscriminator('withdraw');

/**
 * Derive all PDAs needed for withdrawal
 * Seeds match the pSol program exactly:
 * - pool_config: ["pool", token_mint]
 * - merkle_tree: ["merkle_tree", pool_config]
 * - verification_key: ["verification_key", pool_config]
 * - spent_nullifier: ["nullifier", pool_config, nullifier_hash]
 * - vault: ["vault", pool_config]
 */
function deriveWithdrawPDAs(
  poolAddress: PublicKey,
  nullifierHash: Uint8Array,
  programId: PublicKey
): {
  vault: PublicKey;
  merkleTree: PublicKey;
  verificationKey: PublicKey;
  spentNullifier: PublicKey;
} {
  const [merkleTree] = PublicKey.findProgramAddressSync(
    [Buffer.from('merkle_tree'), poolAddress.toBuffer()],
    programId
  );

  const [verificationKey] = PublicKey.findProgramAddressSync(
    [Buffer.from('verification_key'), poolAddress.toBuffer()],
    programId
  );

  const [spentNullifier] = PublicKey.findProgramAddressSync(
    [
      Buffer.from('nullifier'),
      poolAddress.toBuffer(),
      Buffer.from(nullifierHash),
    ],
    programId
  );

  const [vault] = PublicKey.findProgramAddressSync(
    [Buffer.from('vault'), poolAddress.toBuffer()],
    programId
  );

  return { vault, merkleTree, verificationKey, spentNullifier };
}

/**
 * Build withdraw instruction data using Anchor's borsh serialization
 * 
 * Format:
 * - 8 bytes: discriminator (sha256("global:withdraw")[0..8])
 * - 4 bytes: proof_data length (u32 LE)
 * - N bytes: proof_data
 * - 32 bytes: merkle_root
 * - 32 bytes: nullifier_hash
 * - 32 bytes: recipient (Pubkey)
 * - 8 bytes: amount (u64 LE)
 * - 32 bytes: relayer (Pubkey)
 * - 8 bytes: relayer_fee (u64 LE)
 */
function buildWithdrawInstructionData(
  request: WithdrawalRequestValidated
): Buffer {
  const proofLength = request.proofData.length;
  
  // Total size: 8 + 4 + proofLength + 32 + 32 + 32 + 8 + 32 + 8
  const totalSize = 8 + 4 + proofLength + 32 + 32 + 32 + 8 + 32 + 8;
  const buffer = Buffer.alloc(totalSize);
  let offset = 0;

  // 1. Discriminator (8 bytes)
  WITHDRAW_DISCRIMINATOR.copy(buffer, offset);
  offset += 8;

  // 2. proof_data as Vec<u8>: 4-byte length prefix + data
  buffer.writeUInt32LE(proofLength, offset);
  offset += 4;
  Buffer.from(request.proofData).copy(buffer, offset);
  offset += proofLength;

  // 3. merkle_root: [u8; 32]
  Buffer.from(request.merkleRoot).copy(buffer, offset);
  offset += 32;

  // 4. nullifier_hash: [u8; 32]
  Buffer.from(request.nullifierHash).copy(buffer, offset);
  offset += 32;

  // 5. recipient: Pubkey (32 bytes)
  request.recipient.toBuffer().copy(buffer, offset);
  offset += 32;

  // 6. amount: u64 (8 bytes LE)
  buffer.writeBigUInt64LE(BigInt(request.amount.toString()), offset);
  offset += 8;

  // 7. relayer: Pubkey (32 bytes) - this is the relayer address from proof
  // Note: In pSol, this comes from the proof public inputs
  // The relayer submitting the TX is the payer, but this field is the
  // relayer address that was committed to in the ZK proof
  request.recipient.toBuffer().copy(buffer, offset); // Will be overwritten below
  offset += 32;

  // 8. relayer_fee: u64 (8 bytes LE)
  buffer.writeBigUInt64LE(BigInt(request.relayerFee.toString()), offset);

  return buffer;
}

/**
 * Build withdraw instruction data with explicit relayer address
 */
function buildWithdrawInstructionDataFull(
  proofData: Uint8Array,
  merkleRoot: Uint8Array,
  nullifierHash: Uint8Array,
  recipient: PublicKey,
  amount: BN,
  relayer: PublicKey,
  relayerFee: BN
): Buffer {
  const proofLength = proofData.length;
  const totalSize = 8 + 4 + proofLength + 32 + 32 + 32 + 8 + 32 + 8;
  const buffer = Buffer.alloc(totalSize);
  let offset = 0;

  // 1. Discriminator
  WITHDRAW_DISCRIMINATOR.copy(buffer, offset);
  offset += 8;

  // 2. proof_data Vec<u8>
  buffer.writeUInt32LE(proofLength, offset);
  offset += 4;
  Buffer.from(proofData).copy(buffer, offset);
  offset += proofLength;

  // 3. merkle_root [u8; 32]
  Buffer.from(merkleRoot).copy(buffer, offset);
  offset += 32;

  // 4. nullifier_hash [u8; 32]
  Buffer.from(nullifierHash).copy(buffer, offset);
  offset += 32;

  // 5. recipient Pubkey
  recipient.toBuffer().copy(buffer, offset);
  offset += 32;

  // 6. amount u64
  buffer.writeBigUInt64LE(BigInt(amount.toString()), offset);
  offset += 8;

  // 7. relayer Pubkey
  relayer.toBuffer().copy(buffer, offset);
  offset += 32;

  // 8. relayer_fee u64
  buffer.writeBigUInt64LE(BigInt(relayerFee.toString()), offset);

  return buffer;
}

/**
 * Check if token account exists, create if needed
 */
async function ensureTokenAccount(
  connection: Connection,
  payer: Keypair,
  mint: PublicKey,
  owner: PublicKey
): Promise<{ address: PublicKey; instruction?: TransactionInstruction }> {
  const ata = await getAssociatedTokenAddress(mint, owner);

  try {
    await getAccount(connection, ata);
    return { address: ata };
  } catch {
    // Account doesn't exist, need to create
    logger.debug({ owner: owner.toString(), mint: mint.toString() }, 'Creating token account');
    
    const instruction = createAssociatedTokenAccountInstruction(
      payer.publicKey,
      ata,
      owner,
      mint
    );

    return { address: ata, instruction };
  }
}

/**
 * Build complete withdrawal transaction
 * 
 * Account order matches Withdraw struct in pSol program (withdraw.rs):
 * 0. pool_config (mut)
 * 1. merkle_tree
 * 2. verification_key
 * 3. spent_nullifier (init, mut)
 * 4. vault (mut)
 * 5. recipient_token_account (mut)
 * 6. relayer_token_account (mut)
 * 7. payer (signer, mut)
 * 8. token_program
 * 9. system_program
 */
export async function buildWithdrawTransaction(
  connection: Connection,
  config: RelayerConfig,
  relayerKeypair: Keypair,
  request: WithdrawalRequestValidated,
  tokenMint: PublicKey
): Promise<Transaction> {
  const tx = new Transaction();

  // Add compute budget instructions for complex ZK verification
  // Groth16 verification on Solana needs ~300-400k CU
  tx.add(
    ComputeBudgetProgram.setComputeUnitLimit({
      units: 500_000,
    })
  );

  // Add priority fee based on network conditions
  const recentFees = await connection.getRecentPrioritizationFees();
  const avgPriorityFee = recentFees.length > 0
    ? Math.ceil(recentFees.reduce((sum, f) => sum + f.prioritizationFee, 0) / recentFees.length)
    : 1000;

  tx.add(
    ComputeBudgetProgram.setComputeUnitPrice({
      microLamports: Math.max(avgPriorityFee, 1000),
    })
  );

  // Derive PDAs
  const pdas = deriveWithdrawPDAs(
    request.poolAddress,
    request.nullifierHash,
    config.programId
  );

  // Ensure recipient token account exists
  const recipientAta = await ensureTokenAccount(
    connection,
    relayerKeypair,
    tokenMint,
    request.recipient
  );

  if (recipientAta.instruction) {
    tx.add(recipientAta.instruction);
  }

  // Ensure relayer token account exists (for fee)
  const relayerAta = await ensureTokenAccount(
    connection,
    relayerKeypair,
    tokenMint,
    relayerKeypair.publicKey
  );

  if (relayerAta.instruction) {
    tx.add(relayerAta.instruction);
  }

  // Build withdraw instruction with EXACT account order from pSol program
  const withdrawIx = new TransactionInstruction({
    programId: config.programId,
    keys: [
      // 0. pool_config (mut) - PDA ["pool", token_mint]
      { pubkey: request.poolAddress, isSigner: false, isWritable: true },
      
      // 1. merkle_tree - PDA ["merkle_tree", pool_config]
      { pubkey: pdas.merkleTree, isSigner: false, isWritable: false },
      
      // 2. verification_key - PDA ["verification_key", pool_config]
      { pubkey: pdas.verificationKey, isSigner: false, isWritable: false },
      
      // 3. spent_nullifier (init, mut) - PDA ["nullifier", pool_config, nullifier_hash]
      { pubkey: pdas.spentNullifier, isSigner: false, isWritable: true },
      
      // 4. vault (mut) - PDA ["vault", pool_config]
      { pubkey: pdas.vault, isSigner: false, isWritable: true },
      
      // 5. recipient_token_account (mut)
      { pubkey: recipientAta.address, isSigner: false, isWritable: true },
      
      // 6. relayer_token_account (mut)
      { pubkey: relayerAta.address, isSigner: false, isWritable: true },
      
      // 7. payer (signer, mut) - relayer pays for tx and nullifier account
      { pubkey: relayerKeypair.publicKey, isSigner: true, isWritable: true },
      
      // 8. token_program
      { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      
      // 9. system_program
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data: buildWithdrawInstructionDataFull(
      request.proofData,
      request.merkleRoot,
      request.nullifierHash,
      request.recipient,
      request.amount,
      relayerKeypair.publicKey, // relayer address for the proof
      request.relayerFee
    ),
  });

  tx.add(withdrawIx);

  // Set recent blockhash and fee payer
  const { blockhash, lastValidBlockHeight } = await connection.getLatestBlockhash();
  tx.recentBlockhash = blockhash;
  tx.lastValidBlockHeight = lastValidBlockHeight;
  tx.feePayer = relayerKeypair.publicKey;

  return tx;
}

/**
 * Submit transaction with retries
 */
export async function submitTransaction(
  connection: Connection,
  tx: Transaction,
  signers: Keypair[],
  maxRetries: number = 3
): Promise<string> {
  let lastError: Error | null = null;

  for (let attempt = 1; attempt <= maxRetries; attempt++) {
    try {
      logTransaction('submitting', undefined, { attempt, maxRetries });

      const signature = await sendAndConfirmTransaction(
        connection,
        tx,
        signers,
        {
          commitment: 'confirmed',
          maxRetries: 3,
        }
      );

      logTransaction('confirmed', signature);
      return signature;
    } catch (error) {
      lastError = error as Error;
      logger.warn({ error, attempt }, 'Transaction submission failed');

      if (attempt < maxRetries) {
        // Update blockhash for retry
        const { blockhash, lastValidBlockHeight } = await connection.getLatestBlockhash();
        tx.recentBlockhash = blockhash;
        tx.lastValidBlockHeight = lastValidBlockHeight;

        // Wait before retry
        await new Promise((resolve) => setTimeout(resolve, 1000 * attempt));
      }
    }
  }

  throw lastError || new Error('Transaction submission failed');
}

/**
 * Check relayer balance
 */
export async function checkRelayerBalance(
  connection: Connection,
  relayerPublicKey: PublicKey
): Promise<{
  balance: number;
  sufficient: boolean;
  warning: boolean;
}> {
  const balance = await connection.getBalance(relayerPublicKey);
  const balanceSol = balance / LAMPORTS_PER_SOL;

  return {
    balance: balanceSol,
    sufficient: balanceSol >= 0.1, // Minimum 0.1 SOL
    warning: balanceSol < 0.5, // Warning below 0.5 SOL
  };
}

/**
 * Estimate transaction cost
 */
export async function estimateTransactionCost(
  connection: Connection,
  tx: Transaction
): Promise<{
  baseFee: number;
  priorityFee: number;
  total: number;
}> {
  // Get fee for transaction
  const message = tx.compileMessage();
  const feeCalc = await connection.getFeeForMessage(message);
  const baseFee = feeCalc.value || 5000;

  // Estimate priority fee
  const recentFees = await connection.getRecentPrioritizationFees();
  const avgPriorityFee = recentFees.length > 0
    ? Math.ceil(recentFees.reduce((sum, f) => sum + f.prioritizationFee, 0) / recentFees.length)
    : 1000;

  const priorityFee = avgPriorityFee * 400_000 / 1_000_000; // CU * price / 1M

  return {
    baseFee,
    priorityFee,
    total: baseFee + priorityFee,
  };
}
