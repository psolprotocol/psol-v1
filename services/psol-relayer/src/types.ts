/**
 * pSol Relayer Types - Hardened for Mainnet
 * 
 * SECURITY: This file defines all data structures.
 * No sensitive data (keys, proofs) should be logged from these types.
 */

import { PublicKey } from '@solana/web3.js';
import BN from 'bn.js';

// =============================================================================
// Configuration
// =============================================================================

export interface RelayerConfig {
  // Solana
  rpcUrl: string;
  wsUrl: string;
  network: 'devnet' | 'testnet' | 'mainnet-beta';
  commitment: 'processed' | 'confirmed' | 'finalized';

  // Program
  programId: PublicKey;
  verificationKeyPath: string;

  // Wallet - SECURITY: Never log this
  relayerKeypair: Uint8Array;
  maxBalance: number;
  minBalance: number;

  // Fees
  baseFeeBps: number;
  minFeeLamports: number;
  maxFeeBps: number;
  enableDynamicFees: boolean;

  // Server
  port: number;
  host: string;
  corsOrigins: string[];

  // Redis
  redisHost: string;
  redisPort: number;
  redisPassword?: string;
  redisDb: number;

  // Rate Limiting
  rateLimitWindowMs: number;
  rateLimitMaxRequests: number;
  maxPendingPerIp: number;
  maxPendingGlobal: number;

  // Authentication
  apiKeys: string[];
  adminApiKey?: string;
  allowedIps: string[];
  requireAuthForWrite: boolean;
  allowUnauthenticatedRead: boolean;

  // Metrics
  enableMetrics: boolean;
  metricsPath: string;
}

// =============================================================================
// Withdrawal Request
// =============================================================================

export interface WithdrawalRequest {
  poolAddress: string;
  tokenMint: string;
  proofData: string;      // Hex encoded - SECURITY: Never log full value
  merkleRoot: string;     // Hex encoded
  nullifierHash: string;  // Hex encoded - use for dedup, log only truncated
  recipient: string;
  amount: string;
  relayerFee: string;
}

export interface WithdrawalRequestValidated {
  poolAddress: PublicKey;
  tokenMint: PublicKey;
  proofData: Uint8Array;      // SECURITY: Never log
  merkleRoot: Uint8Array;
  nullifierHash: Uint8Array;  // SECURITY: Log only truncated hash
  recipient: PublicKey;
  amount: BN;
  relayerFee: BN;
}

// =============================================================================
// Job Status & Persistence
// =============================================================================

export type JobStatus =
  | 'queued'
  | 'processing'
  | 'submitting'
  | 'confirming'
  | 'succeeded'
  | 'failed';

/**
 * Persistent job record - stored in Redis
 * SECURITY: proofData is NOT stored, only nullifierHash for dedup
 */
export interface PersistedJob {
  id: string;
  status: JobStatus;
  
  // Request metadata (no proof data)
  poolAddress: string;
  tokenMint: string;
  nullifierHash: string;  // Hex - used for deduplication
  recipient: string;
  amount: string;
  relayerFee: string;
  
  // Timestamps
  createdAt: number;
  updatedAt: number;
  completedAt?: number;
  
  // Processing info
  attempts: number;
  maxAttempts: number;
  
  // Results
  txSignature?: string;
  errorCode?: string;
  errorMessage?: string;
  
  // Client tracking (privacy-safe)
  clientIp?: string;      // Hashed in production
  clientId?: string;      // API key identifier (not the key itself)
}

// =============================================================================
// API Responses - Standardized Format
// =============================================================================

export interface ApiResponse<T = unknown> {
  success: boolean;
  data?: T;
  error?: {
    code: RelayerErrorCode;
    message: string;
    details?: unknown;
  };
  meta?: {
    requestId: string;
    timestamp: number;
  };
}

export interface SubmitResponseData {
  jobId: string;
  status: JobStatus;
  estimatedTime: number;
}

export interface JobStatusResponseData {
  jobId: string;
  status: JobStatus;
  txSignature?: string;
  errorCode?: string;
  errorMessage?: string;
  createdAt: number;
  updatedAt: number;
}

export interface FeeQuoteResponseData {
  baseFee: string;
  dynamicFee: string;
  totalFee: string;
  feeBps: number;
  validUntil: number;
}

export interface RelayerInfoResponseData {
  address: string;
  balance: string;
  network: string;
  feeBps: number;
  minFee: string;
  status: 'active' | 'paused' | 'low_balance';
  pendingJobs: number;
  totalProcessed: number;
  successRate: number;
}

export interface HealthResponseData {
  status: 'healthy' | 'degraded' | 'unhealthy';
  slot: number;
  relayerBalance: number;
  redisConnected: boolean;
  pendingJobs: number;
  timestamp: number;
}

// =============================================================================
// Error Codes - Exhaustive Enum
// =============================================================================

export enum RelayerErrorCode {
  // Client errors (4xx)
  INVALID_REQUEST = 'INVALID_REQUEST',
  INVALID_PROOF = 'INVALID_PROOF',
  INVALID_POOL = 'INVALID_POOL',
  INVALID_AMOUNT = 'INVALID_AMOUNT',
  NULLIFIER_SPENT = 'NULLIFIER_SPENT',
  NULLIFIER_PENDING = 'NULLIFIER_PENDING',
  INSUFFICIENT_FEE = 'INSUFFICIENT_FEE',
  POOL_PAUSED = 'POOL_PAUSED',
  ROOT_NOT_FOUND = 'ROOT_NOT_FOUND',
  
  // Auth errors
  UNAUTHORIZED = 'UNAUTHORIZED',
  FORBIDDEN = 'FORBIDDEN',
  INVALID_API_KEY = 'INVALID_API_KEY',
  IP_NOT_ALLOWED = 'IP_NOT_ALLOWED',
  
  // Rate limiting
  RATE_LIMITED = 'RATE_LIMITED',
  TOO_MANY_PENDING = 'TOO_MANY_PENDING',
  QUEUE_FULL = 'QUEUE_FULL',
  
  // Server errors (5xx)
  INTERNAL_ERROR = 'INTERNAL_ERROR',
  RELAYER_PAUSED = 'RELAYER_PAUSED',
  LOW_BALANCE = 'LOW_BALANCE',
  SUBMISSION_FAILED = 'SUBMISSION_FAILED',
  RPC_ERROR = 'RPC_ERROR',
  REDIS_ERROR = 'REDIS_ERROR',
  TIMEOUT = 'TIMEOUT',
  
  // Job errors
  JOB_NOT_FOUND = 'JOB_NOT_FOUND',
  JOB_ALREADY_EXISTS = 'JOB_ALREADY_EXISTS',
}

// HTTP status code mapping
export const ErrorCodeToHttpStatus: Record<RelayerErrorCode, number> = {
  [RelayerErrorCode.INVALID_REQUEST]: 400,
  [RelayerErrorCode.INVALID_PROOF]: 400,
  [RelayerErrorCode.INVALID_POOL]: 400,
  [RelayerErrorCode.INVALID_AMOUNT]: 400,
  [RelayerErrorCode.NULLIFIER_SPENT]: 400,
  [RelayerErrorCode.NULLIFIER_PENDING]: 409,
  [RelayerErrorCode.INSUFFICIENT_FEE]: 400,
  [RelayerErrorCode.POOL_PAUSED]: 400,
  [RelayerErrorCode.ROOT_NOT_FOUND]: 400,
  [RelayerErrorCode.UNAUTHORIZED]: 401,
  [RelayerErrorCode.FORBIDDEN]: 403,
  [RelayerErrorCode.INVALID_API_KEY]: 401,
  [RelayerErrorCode.IP_NOT_ALLOWED]: 403,
  [RelayerErrorCode.RATE_LIMITED]: 429,
  [RelayerErrorCode.TOO_MANY_PENDING]: 429,
  [RelayerErrorCode.QUEUE_FULL]: 503,
  [RelayerErrorCode.INTERNAL_ERROR]: 500,
  [RelayerErrorCode.RELAYER_PAUSED]: 503,
  [RelayerErrorCode.LOW_BALANCE]: 503,
  [RelayerErrorCode.SUBMISSION_FAILED]: 500,
  [RelayerErrorCode.RPC_ERROR]: 502,
  [RelayerErrorCode.REDIS_ERROR]: 503,
  [RelayerErrorCode.TIMEOUT]: 504,
  [RelayerErrorCode.JOB_NOT_FOUND]: 404,
  [RelayerErrorCode.JOB_ALREADY_EXISTS]: 409,
};

// =============================================================================
// Metrics
// =============================================================================

export interface RelayerMetrics {
  // Counters
  requestsTotal: number;
  requestsByEndpoint: Record<string, number>;
  jobsSubmitted: number;
  jobsSucceeded: number;
  jobsFailed: number;
  jobsRejected: number;
  
  // Per-pool stats
  jobsByPool: Record<string, {
    submitted: number;
    succeeded: number;
    failed: number;
    volumeTotal: string;
    feesTotal: string;
  }>;
  
  // Gauges
  pendingJobs: number;
  relayerBalanceLamports: number;
  
  // Histograms (simplified - store last N values)
  processingTimesMs: number[];
  
  // Derived
  avgProcessingTimeMs: number;
  successRate: number;
  
  // Timestamps
  startedAt: number;
  lastUpdatedAt: number;
}

// =============================================================================
// Internal Types
// =============================================================================

export interface RateLimitEntry {
  count: number;
  windowStart: number;
}

export interface AuthContext {
  authenticated: boolean;
  apiKeyId?: string;
  clientIp: string;
  isAdmin: boolean;
}
