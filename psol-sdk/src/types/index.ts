/**
 * pSol SDK Types
 * Type definitions matching on-chain structures
 */

import { PublicKey } from '@solana/web3.js';
import BN from 'bn.js';

// ============================================================================
// Account Types (matching on-chain state)
// ============================================================================

/**
 * Pool configuration account structure
 * Matches: programs/psol-privacy/src/state/pool_config.rs
 */
export interface PoolConfig {
  authority: PublicKey;
  pendingAuthority: PublicKey;
  tokenMint: PublicKey;
  vault: PublicKey;
  merkleTree: PublicKey;
  verificationKey: PublicKey;
  treeDepth: number;
  bump: number;
  isPaused: boolean;
  vkConfigured: boolean;
  vkLocked: boolean; // Phase 4
  totalDeposits: BN;
  totalWithdrawals: BN;
  totalValueDeposited: BN; // Phase 4
  totalValueWithdrawn: BN; // Phase 4
  version: number;
}

/**
 * Merkle tree account structure
 */
export interface MerkleTreeAccount {
  pool: PublicKey;
  treeDepth: number;
  rootHistorySize: number;
  nextLeafIndex: number;
  currentRootIndex: number;
  roots: Uint8Array[];
  filledSubtrees: Uint8Array[];
  bump: number;
}

/**
 * Verification key account structure
 */
export interface VerificationKeyAccount {
  pool: PublicKey;
  isInitialized: boolean;
  bump: number;
  alphaG1: Uint8Array; // 64 bytes
  betaG2: Uint8Array; // 128 bytes
  gammaG2: Uint8Array; // 128 bytes
  deltaG2: Uint8Array; // 128 bytes
  icLength: number;
  ic: Uint8Array[]; // Array of 64-byte G1 points
}

/**
 * Spent nullifier PDA account
 */
export interface SpentNullifier {
  pool: PublicKey;
  nullifierHash: Uint8Array;
  spentAt: BN;
  spentSlot: BN;
  bump: number;
}

// ============================================================================
// Instruction Parameter Types
// ============================================================================

/**
 * Parameters for initializing a new pool
 */
export interface InitializePoolParams {
  treeDepth?: number; // Default: 20
  rootHistorySize?: number; // Default: 100
}

/**
 * Parameters for setting verification key
 */
export interface SetVerificationKeyParams {
  alphaG1: Uint8Array;
  betaG2: Uint8Array;
  gammaG2: Uint8Array;
  deltaG2: Uint8Array;
  ic: Uint8Array[];
}

/**
 * Parameters for deposit instruction
 */
export interface DepositParams {
  amount: BN;
  commitment: Uint8Array;
}

/**
 * Parameters for withdraw instruction
 */
export interface WithdrawParams {
  proofData: Uint8Array;
  merkleRoot: Uint8Array;
  nullifierHash: Uint8Array;
  recipient: PublicKey;
  amount: BN;
  relayer: PublicKey;
  relayerFee: BN;
}

// ============================================================================
// Deposit Note Types
// ============================================================================

/**
 * Secret data for a deposit note
 * KEEP THIS SECURE - losing this means losing access to funds
 */
export interface DepositNote {
  /** Random secret (32 bytes) */
  secret: Uint8Array;
  /** Random nullifier (32 bytes) */
  nullifier: Uint8Array;
  /** Poseidon hash of (secret, nullifier) */
  commitment: Uint8Array;
  /** Token amount deposited */
  amount: BN;
  /** Pool address */
  pool: PublicKey;
  /** Token mint */
  tokenMint: PublicKey;
  /** Leaf index in Merkle tree (set after deposit) */
  leafIndex?: number;
  /** Deposit timestamp */
  depositedAt?: number;
  /** Transaction signature */
  txSignature?: string;
}

/**
 * Serialized note format (base64 encoded)
 * Format: psol:v1:<base64_data>
 */
export type SerializedNote = string;

/**
 * Parsed note from serialized format
 */
export interface ParsedNote {
  version: number;
  secret: Uint8Array;
  nullifier: Uint8Array;
  amount: BN;
  pool: PublicKey;
  tokenMint: PublicKey;
  leafIndex?: number;
}

// ============================================================================
// Proof Types
// ============================================================================

/**
 * ZK proof for withdrawal
 */
export interface WithdrawProof {
  /** Groth16 proof (256 bytes) */
  proofData: Uint8Array;
  /** Public inputs */
  publicInputs: WithdrawPublicInputs;
}

/**
 * Public inputs for withdraw proof verification
 */
export interface WithdrawPublicInputs {
  merkleRoot: Uint8Array;
  nullifierHash: Uint8Array;
  recipient: PublicKey;
  amount: BN;
  relayer: PublicKey;
  relayerFee: BN;
}

/**
 * Merkle proof for a leaf
 */
export interface MerkleProof {
  /** Sibling hashes from leaf to root */
  pathElements: Uint8Array[];
  /** Path indices (0 = left, 1 = right) */
  pathIndices: number[];
  /** Root hash */
  root: Uint8Array;
  /** Leaf index */
  leafIndex: number;
}

// ============================================================================
// Event Types
// ============================================================================

/**
 * Deposit event emitted by program
 */
export interface DepositEvent {
  pool: PublicKey;
  commitment: Uint8Array;
  leafIndex: number;
  amount: BN;
  timestamp: BN;
}

/**
 * Withdraw event emitted by program
 */
export interface WithdrawEvent {
  pool: PublicKey;
  nullifierHash: Uint8Array;
  recipient: PublicKey;
  amount: BN;
  relayer: PublicKey;
  relayerFee: BN;
  timestamp: BN;
}

/**
 * Pool initialized event
 */
export interface PoolInitializedEvent {
  pool: PublicKey;
  authority: PublicKey;
  tokenMint: PublicKey;
  treeDepth: number;
  rootHistorySize: number;
  timestamp: BN;
}

/**
 * Authority transfer events (Phase 4)
 */
export interface AuthorityTransferInitiatedEvent {
  pool: PublicKey;
  currentAuthority: PublicKey;
  pendingAuthority: PublicKey;
  timestamp: BN;
}

export interface AuthorityTransferCompletedEvent {
  pool: PublicKey;
  oldAuthority: PublicKey;
  newAuthority: PublicKey;
  timestamp: BN;
}

/**
 * VK locked event (Phase 4)
 */
export interface VerificationKeyLockedEvent {
  pool: PublicKey;
  authority: PublicKey;
  timestamp: BN;
}

// ============================================================================
// Configuration Types
// ============================================================================

/**
 * SDK configuration options
 */
export interface PsolClientConfig {
  /** Program ID */
  programId?: PublicKey;
  /** Commitment level */
  commitment?: 'processed' | 'confirmed' | 'finalized';
  /** Skip preflight checks */
  skipPreflight?: boolean;
  /** Custom RPC endpoint */
  rpcEndpoint?: string;
}

/**
 * Transaction result
 */
export interface TransactionResult {
  signature: string;
  slot: number;
  confirmations: number | null;
}

/**
 * Pool status for UI display
 */
export interface PoolStatus {
  address: PublicKey;
  tokenMint: PublicKey;
  tokenSymbol?: string;
  isPaused: boolean;
  vkConfigured: boolean;
  vkLocked: boolean;
  totalDeposits: number;
  totalWithdrawals: number;
  totalValueLocked: BN;
  treeCapacity: number;
  treeUtilization: number; // 0-1
  authority: PublicKey;
  hasPendingAuthorityTransfer: boolean;
}

// ============================================================================
// Error Types
// ============================================================================

/**
 * SDK error codes (matching on-chain errors)
 */
export enum PsolErrorCode {
  InvalidProof = 6000,
  InvalidProofFormat = 6001,
  InvalidPublicInputs = 6002,
  VerificationKeyNotSet = 6003,
  InvalidMerkleRoot = 6004,
  MerkleTreeFull = 6005,
  InvalidTreeDepth = 6006,
  InvalidRootHistorySize = 6007,
  NullifierAlreadySpent = 6008,
  InvalidNullifier = 6009,
  InvalidAmount = 6010,
  InsufficientBalance = 6011,
  InvalidMint = 6012,
  RelayerFeeExceedsAmount = 6013,
  InvalidCommitment = 6014,
  DuplicateCommitment = 6015,
  InvalidSecret = 6016,
  Unauthorized = 6017,
  PoolPaused = 6018,
  RecipientMismatch = 6019,
  ArithmeticOverflow = 6020,
  NotImplemented = 6021,
  CryptoNotImplemented = 6022,
  // Phase 4 errors
  VerificationKeyLocked = 6023,
  InvalidAuthority = 6024,
  NoPendingAuthority = 6025,
  AlreadyInitialized = 6026,
  InputTooLarge = 6027,
  PoolHasDeposits = 6028,
  InvalidOwner = 6029,
  CorruptedData = 6030,
  LimitExceeded = 6031,
  InvalidTimestamp = 6032,
}

/**
 * Custom SDK error
 */
export class PsolError extends Error {
  constructor(
    public code: PsolErrorCode | number,
    message: string,
    public cause?: Error
  ) {
    super(message);
    this.name = 'PsolError';
  }
}

// ============================================================================
// Constants
// ============================================================================

export const PSOL_CONSTANTS = {
  /** Default Merkle tree depth */
  DEFAULT_TREE_DEPTH: 20,
  /** Maximum Merkle tree depth */
  MAX_TREE_DEPTH: 24,
  /** Minimum Merkle tree depth */
  MIN_TREE_DEPTH: 4,
  /** Default root history size */
  DEFAULT_ROOT_HISTORY: 100,
  /** Maximum root history size */
  MAX_ROOT_HISTORY: 1000,
  /** Minimum root history size */
  MIN_ROOT_HISTORY: 30,
  /** Maximum deposit amount */
  MAX_DEPOSIT_AMOUNT: new BN('1000000000000000'), // 1M tokens with 9 decimals
  /** Maximum relayer fee basis points */
  MAX_RELAYER_FEE_BPS: 1000, // 10%
  /** Field size for BN254 curve */
  FIELD_SIZE: new BN(
    '21888242871839275222246405745257275088548364400416034343698204186575808495617'
  ),
  /** Zero value for empty Merkle leaves */
  ZERO_VALUE: new BN(
    '21663839004416932945382355908790599225266501822907911457504978515578255421292'
  ),
} as const;

/**
 * Program ID for mainnet/devnet
 */
export const PROGRAM_ID = new PublicKey(
  'Ddokrq1M6hT9Vu63k4JWqVRSecyLeotNf8xKknKfRwvZ'
);
