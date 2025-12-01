/**
 * Proof Validation
 * Validates ZK proofs before submitting to chain
 */

import * as snarkjs from 'snarkjs';
import { readFile } from 'fs/promises';
import { existsSync } from 'fs';
import { PublicKey, Connection } from '@solana/web3.js';
import BN from 'bn.js';
import { logger, truncateHash } from './logger.js';
import type { WithdrawalRequestValidated, RelayerConfig } from './types.js';

/**
 * Verification key structure
 */
interface VerificationKey {
  protocol: string;
  curve: string;
  nPublic: number;
  vk_alpha_1: string[];
  vk_beta_2: string[][];
  vk_gamma_2: string[][];
  vk_delta_2: string[][];
  IC: string[][];
}

let verificationKey: VerificationKey | null = null;

/**
 * Load verification key from file
 */
export async function loadVerificationKey(path: string): Promise<void> {
  if (!existsSync(path)) {
    logger.warn({ path }, 'Verification key not found - local proof validation disabled');
    return;
  }

  try {
    const content = await readFile(path, 'utf-8');
    verificationKey = JSON.parse(content);
    logger.info({ path }, 'Verification key loaded');
  } catch (error) {
    logger.error({ error, path }, 'Failed to load verification key');
    throw error;
  }
}

/**
 * Convert proof data (256 bytes) to snarkjs format
 */
function deserializeProof(data: Uint8Array): {
  pi_a: [string, string, string];
  pi_b: [[string, string], [string, string], [string, string]];
  pi_c: [string, string, string];
  protocol: string;
  curve: string;
} {
  if (data.length !== 256) {
    throw new Error(`Invalid proof length: ${data.length}, expected 256`);
  }

  // Helper to convert 32 bytes to decimal string
  const toDecimal = (bytes: Uint8Array): string => {
    return new BN(Buffer.from(bytes)).toString(10);
  };

  // pi_a: 64 bytes (2 x 32)
  const pi_a_x = toDecimal(data.slice(0, 32));
  const pi_a_y = toDecimal(data.slice(32, 64));

  // pi_b: 128 bytes (4 x 32) - note: coordinates are swapped in Groth16
  const pi_b_x0 = toDecimal(data.slice(64, 96));
  const pi_b_x1 = toDecimal(data.slice(96, 128));
  const pi_b_y0 = toDecimal(data.slice(128, 160));
  const pi_b_y1 = toDecimal(data.slice(160, 192));

  // pi_c: 64 bytes (2 x 32)
  const pi_c_x = toDecimal(data.slice(192, 224));
  const pi_c_y = toDecimal(data.slice(224, 256));

  return {
    pi_a: [pi_a_x, pi_a_y, '1'],
    pi_b: [
      [pi_b_x0, pi_b_x1],
      [pi_b_y0, pi_b_y1],
      ['1', '0'],
    ],
    pi_c: [pi_c_x, pi_c_y, '1'],
    protocol: 'groth16',
    curve: 'bn128',
  };
}

/**
 * Generate public signals array from request
 */
function generatePublicSignals(request: WithdrawalRequestValidated): string[] {
  const bytes32ToDecimal = (bytes: Uint8Array): string => {
    return new BN(Buffer.from(bytes)).toString(10);
  };

  const pubkeyToDecimal = (pubkey: PublicKey): string => {
    return new BN(pubkey.toBuffer()).toString(10);
  };

  return [
    bytes32ToDecimal(request.merkleRoot),
    bytes32ToDecimal(request.nullifierHash),
    pubkeyToDecimal(request.recipient),
    request.amount.toString(10),
    pubkeyToDecimal(new PublicKey(request.poolAddress.toString())), // relayer address from pool
    request.relayerFee.toString(10),
  ];
}

/**
 * Validate proof locally using snarkjs
 */
export async function validateProofLocally(
  request: WithdrawalRequestValidated
): Promise<{ valid: boolean; error?: string }> {
  if (!verificationKey) {
    logger.debug('Skipping local proof validation - no verification key');
    return { valid: true };
  }

  try {
    const proof = deserializeProof(request.proofData);
    const publicSignals = generatePublicSignals(request);

    logger.debug({ publicSignals }, 'Validating proof with public signals');

    const isValid = await snarkjs.groth16.verify(
      verificationKey,
      publicSignals,
      proof
    );

    if (!isValid) {
      return { valid: false, error: 'Proof verification failed' };
    }

    return { valid: true };
  } catch (error) {
    logger.error({ error }, 'Proof validation error');
    return {
      valid: false,
      error: error instanceof Error ? error.message : 'Unknown validation error',
    };
  }
}

/**
 * Check if nullifier has been spent on-chain
 */
export async function isNullifierSpent(
  connection: Connection,
  programId: PublicKey,
  poolAddress: PublicKey,
  nullifierHash: Uint8Array
): Promise<boolean> {
  // Derive nullifier PDA
  const [nullifierPda] = PublicKey.findProgramAddressSync(
    [
      Buffer.from('nullifier'),
      poolAddress.toBuffer(),
      Buffer.from(nullifierHash),
    ],
    programId
  );

  // Check if account exists
  const account = await connection.getAccountInfo(nullifierPda);
  return account !== null;
}

/**
 * Verify merkle root exists in pool's root history
 * 
 * MerkleTree layout (from merkle_tree.rs):
 * Offset  Field                   Size
 * 0       discriminator           8
 * 8       pool                    32
 * 40      depth                   1
 * 41      next_leaf_index         4 (u32)
 * 45      current_root            32
 * 77      root_history            4 + (32 * history_size) - Vec header then data
 * ...     root_history_index      2 (u16)
 * ...     root_history_size       2 (u16)
 * ...     filled_subtrees         4 + (32 * depth)
 * ...     zeros                   4 + (32 * (depth + 1))
 */
export async function verifyMerkleRoot(
  connection: Connection,
  programId: PublicKey,
  poolAddress: PublicKey,
  merkleRoot: Uint8Array
): Promise<boolean> {
  // Derive merkle tree PDA
  const [merkleTreePda] = PublicKey.findProgramAddressSync(
    [Buffer.from('merkle_tree'), poolAddress.toBuffer()],
    programId
  );

  // Fetch merkle tree account
  const account = await connection.getAccountInfo(merkleTreePda);
  if (!account) {
    logger.error({ poolAddress: poolAddress.toString() }, 'Merkle tree account not found');
    return false;
  }

  const data = account.data;
  const targetRoot = Buffer.from(merkleRoot);

  // Parse fixed fields
  let offset = 8; // Skip discriminator
  
  // Skip pool (32 bytes)
  offset += 32;
  
  // Read depth (1 byte)
  const depth = data.readUInt8(offset);
  offset += 1;
  
  // Skip next_leaf_index (4 bytes)
  offset += 4;
  
  // Read current_root (32 bytes)
  const currentRoot = data.slice(offset, offset + 32);
  offset += 32;
  
  // Check current root first (most common case)
  if (currentRoot.equals(targetRoot)) {
    logger.debug('Merkle root matches current root');
    return true;
  }
  
  // Parse root_history Vec
  // Vec layout: 4 bytes length (u32) + data
  const historyLength = data.readUInt32LE(offset);
  offset += 4;
  
  // Check each root in history
  for (let i = 0; i < historyLength; i++) {
    const rootStart = offset + i * 32;
    const root = data.slice(rootStart, rootStart + 32);
    
    if (root.equals(targetRoot)) {
      logger.debug({ rootIndex: i }, 'Merkle root found in history');
      return true;
    }
  }

  logger.warn('Merkle root not found in current root or history');
  return false;
}

/**
 * Validate pool is active and not paused
 * 
 * PoolConfig layout (from pool_config.rs):
 * Offset  Field                   Size
 * 0       discriminator           8
 * 8       authority               32
 * 40      pending_authority       32
 * 72      token_mint              32
 * 104     vault                   32
 * 136     merkle_tree             32
 * 168     verification_key        32
 * 200     tree_depth              1
 * 201     bump                    1
 * 202     is_paused               1
 * 203     vk_configured           1
 * 204     vk_locked               1
 * 205-207 (padding)               3
 * 208     total_deposits          8
 * 216     total_withdrawals       8
 * 224     total_value_deposited   8
 * 232     total_value_withdrawn   8
 * 240     version                 1
 * 241     _reserved               64
 * TOTAL: 305 bytes
 */
export async function validatePool(
  connection: Connection,
  poolAddress: PublicKey
): Promise<{ valid: boolean; error?: string; tokenMint?: PublicKey }> {
  const account = await connection.getAccountInfo(poolAddress);
  
  if (!account) {
    return { valid: false, error: 'Pool not found' };
  }

  const data = account.data;
  
  // Minimum size check
  if (data.length < 241) {
    return { valid: false, error: 'Invalid pool account data' };
  }

  // Parse token_mint at offset 72
  const tokenMint = new PublicKey(data.slice(72, 104));
  
  // Parse is_paused at offset 202
  const isPaused = data.readUInt8(202) === 1;

  if (isPaused) {
    return { valid: false, error: 'Pool is paused' };
  }

  // Check if VK is configured at offset 203
  const vkConfigured = data.readUInt8(203) === 1;

  if (!vkConfigured) {
    return { valid: false, error: 'Pool verification key not configured' };
  }

  return { valid: true, tokenMint };
}

/**
 * Full request validation
 */
export async function validateWithdrawalRequest(
  connection: Connection,
  config: RelayerConfig,
  request: WithdrawalRequestValidated
): Promise<{ valid: boolean; error?: string }> {
  // 1. Validate pool
  const poolValidation = await validatePool(connection, request.poolAddress);
  if (!poolValidation.valid) {
    return poolValidation;
  }

  // 2. Check nullifier not spent
  const nullifierSpent = await isNullifierSpent(
    connection,
    config.programId,
    request.poolAddress,
    request.nullifierHash
  );
  if (nullifierSpent) {
    return { valid: false, error: 'Nullifier already spent' };
  }

  // 3. Verify merkle root
  const rootValid = await verifyMerkleRoot(
    connection,
    config.programId,
    request.poolAddress,
    request.merkleRoot
  );
  if (!rootValid) {
    return { valid: false, error: 'Invalid merkle root' };
  }

  // 4. Validate proof locally (if verification key available)
  const proofValidation = await validateProofLocally(request);
  if (!proofValidation.valid) {
    return proofValidation;
  }

  return { valid: true };
}
