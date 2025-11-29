/**
 * PDA (Program Derived Address) Utilities
 * Derive addresses for pSol accounts
 */

import { PublicKey } from '@solana/web3.js';
import { PROGRAM_ID } from '../types';

/**
 * Derive pool config PDA
 * Seeds: ["pool", token_mint]
 */
export function derivePoolConfig(
  tokenMint: PublicKey,
  programId: PublicKey = PROGRAM_ID
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from('pool'), tokenMint.toBuffer()],
    programId
  );
}

/**
 * Derive vault PDA
 * Seeds: ["vault", pool_config]
 */
export function deriveVault(
  poolConfig: PublicKey,
  programId: PublicKey = PROGRAM_ID
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from('vault'), poolConfig.toBuffer()],
    programId
  );
}

/**
 * Derive Merkle tree PDA
 * Seeds: ["merkle_tree", pool_config]
 */
export function deriveMerkleTree(
  poolConfig: PublicKey,
  programId: PublicKey = PROGRAM_ID
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from('merkle_tree'), poolConfig.toBuffer()],
    programId
  );
}

/**
 * Derive verification key PDA
 * Seeds: ["verification_key", pool_config]
 */
export function deriveVerificationKey(
  poolConfig: PublicKey,
  programId: PublicKey = PROGRAM_ID
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from('verification_key'), poolConfig.toBuffer()],
    programId
  );
}

/**
 * Derive spent nullifier PDA
 * Seeds: ["nullifier", pool_config, nullifier_hash]
 */
export function deriveSpentNullifier(
  poolConfig: PublicKey,
  nullifierHash: Uint8Array,
  programId: PublicKey = PROGRAM_ID
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [
      Buffer.from('nullifier'),
      poolConfig.toBuffer(),
      Buffer.from(nullifierHash),
    ],
    programId
  );
}

/**
 * Derive all pool-related PDAs at once
 */
export function deriveAllPoolPDAs(
  tokenMint: PublicKey,
  programId: PublicKey = PROGRAM_ID
): {
  poolConfig: [PublicKey, number];
  vault: [PublicKey, number];
  merkleTree: [PublicKey, number];
  verificationKey: [PublicKey, number];
} {
  const poolConfig = derivePoolConfig(tokenMint, programId);
  
  return {
    poolConfig,
    vault: deriveVault(poolConfig[0], programId),
    merkleTree: deriveMerkleTree(poolConfig[0], programId),
    verificationKey: deriveVerificationKey(poolConfig[0], programId),
  };
}

/**
 * Check if a nullifier has been spent (PDA exists)
 */
export async function isNullifierSpent(
  connection: { getAccountInfo: (address: PublicKey) => Promise<any> },
  poolConfig: PublicKey,
  nullifierHash: Uint8Array,
  programId: PublicKey = PROGRAM_ID
): Promise<boolean> {
  const [nullifierPDA] = deriveSpentNullifier(poolConfig, nullifierHash, programId);
  const account = await connection.getAccountInfo(nullifierPDA);
  return account !== null;
}

/**
 * Get pool address for a token mint
 */
export function getPoolAddress(
  tokenMint: PublicKey,
  programId: PublicKey = PROGRAM_ID
): PublicKey {
  const [poolConfig] = derivePoolConfig(tokenMint, programId);
  return poolConfig;
}
