/**
 * pSol Privacy Pool SDK
 * TypeScript SDK for interacting with pSol on Solana
 *
 * @packageDocumentation
 */

// Re-export types
export * from './types';

// Client exports
export { PsolClient } from './client';
export { PsolProgram } from './client/program';

// Crypto exports
export { MerkleTree } from './crypto/merkle';
export {
  poseidonHash,
  poseidonHash2,
  generateCommitment,
  generateNullifierHash,
  randomBytes,
  randomFieldElement,
  bnToBytes32,
  bytes32ToBN,
} from './crypto/poseidon';
export {
  generateWithdrawProof,
  serializeProof,
  deserializeProof,
  vkeyJsonToOnChain,
} from './crypto/proof';

// Utils exports
export {
  createDepositNote,
  serializeNote,
  parseNote,
  computeNullifierHash,
  noteFromParsed,
  validateNote,
  noteToSummary,
} from './utils/note';
export {
  derivePoolConfig,
  deriveVault,
  deriveMerkleTree,
  deriveVerificationKey,
  deriveSpentNullifier,
  deriveAllPoolPDAs,
  isNullifierSpent,
  getPoolAddress,
} from './utils/pda';

// Version
export const VERSION = '0.4.0';
