import bs58 from 'bs58';
/**
 * Configuration Loader - Hardened for Mainnet
 * 
 * SECURITY:
 * 1. All config is validated via Zod
 * 2. Sensitive values (RELAYER_PRIVATE_KEY) are loaded once and never re-read
 * 3. Config object is frozen after creation
 * 4. API keys are validated for minimum length
 */

import { PublicKey, Keypair } from '@solana/web3.js';
import { config as dotenvConfig } from 'dotenv';
import { z } from 'zod';
import { createHash } from 'crypto';
import type { RelayerConfig } from './types.js';

// Load .env file
dotenvConfig();

// =============================================================================
// Environment Schema
// =============================================================================

const envSchema = z.object({
  // ==========================================================================
  // Solana Connection
  // ==========================================================================
  SOLANA_RPC_URL: z.string().url().describe('Solana RPC endpoint URL'),
  SOLANA_WS_URL: z.string().url().optional().describe('WebSocket URL (derived from RPC if not set)'),
  SOLANA_NETWORK: z.enum(['devnet', 'testnet', 'mainnet-beta']).default('devnet'),
  SOLANA_COMMITMENT: z.enum(['processed', 'confirmed', 'finalized']).default('confirmed'),

  // ==========================================================================
  // Program Configuration
  // ==========================================================================
  PSOL_PROGRAM_ID: z.string().min(32).max(44).describe('pSol program public key'),
  VERIFICATION_KEY_PATH: z.string().default('./keys/verification_key.json'),

  // ==========================================================================
  // Relayer Wallet - SECURITY CRITICAL
  // ==========================================================================
  RELAYER_PRIVATE_KEY: z.string().min(64).describe('Base58 encoded private key - NEVER LOG THIS'),
  RELAYER_MAX_BALANCE: z.coerce.number().positive().default(10).describe('Max SOL balance before warning'),
  RELAYER_MIN_BALANCE: z.coerce.number().positive().default(0.5).describe('Min SOL balance for operation'),

  // ==========================================================================
  // Fee Configuration
  // ==========================================================================
  BASE_FEE_BPS: z.coerce.number().int().min(0).max(1000).default(50).describe('Base fee in basis points'),
  MIN_FEE_LAMPORTS: z.coerce.number().int().min(0).default(10000).describe('Minimum fee in lamports'),
  MAX_FEE_BPS: z.coerce.number().int().min(0).max(1000).default(500).describe('Maximum fee cap in BPS'),
  ENABLE_DYNAMIC_FEES: z.coerce.boolean().default(true),

  // ==========================================================================
  // Server Configuration
  // ==========================================================================
  PORT: z.coerce.number().int().min(1).max(65535).default(3000),
  HOST: z.string().default('0.0.0.0'),
  CORS_ORIGINS: z.string().default('*').describe('Comma-separated origins or * for all'),

  // ==========================================================================
  // Redis Configuration
  // ==========================================================================
  REDIS_HOST: z.string().default('localhost'),
  REDIS_PORT: z.coerce.number().int().min(1).max(65535).default(6379),
  REDIS_PASSWORD: z.string().optional(),
  REDIS_DB: z.coerce.number().int().min(0).max(15).default(0),

  // ==========================================================================
  // Rate Limiting
  // ==========================================================================
  RATE_LIMIT_WINDOW_MS: z.coerce.number().int().positive().default(60000).describe('Rate limit window in ms'),
  RATE_LIMIT_MAX_REQUESTS: z.coerce.number().int().positive().default(60).describe('Max requests per window'),
  MAX_PENDING_PER_IP: z.coerce.number().int().positive().default(5).describe('Max pending jobs per IP'),
  MAX_PENDING_GLOBAL: z.coerce.number().int().positive().default(1000).describe('Max total pending jobs'),

  // ==========================================================================
  // Authentication
  // ==========================================================================
  // Comma-separated list of API keys (min 32 chars each for security)
  RELAYER_API_KEYS: z.string().optional().describe('Comma-separated API keys for write access'),
  ADMIN_API_KEY: z.string().min(32).optional().describe('Admin API key for management endpoints'),
  ALLOWED_IPS: z.string().optional().describe('Comma-separated allowed IPs (empty = all allowed)'),
  REQUIRE_AUTH_FOR_WRITE: z.coerce.boolean().default(true).describe('Require API key for POST endpoints'),
  ALLOW_UNAUTHENTICATED_READ: z.coerce.boolean().default(true).describe('Allow GET without auth'),

  // ==========================================================================
  // Observability
  // ==========================================================================
  LOG_LEVEL: z.enum(['trace', 'debug', 'info', 'warn', 'error', 'fatal']).default('info'),
  ENABLE_METRICS: z.coerce.boolean().default(true),
  METRICS_PATH: z.string().default('/metrics'),

  // ==========================================================================
  // Node Environment
  // ==========================================================================
  NODE_ENV: z.enum(['development', 'test', 'production']).default('development'),
});

// =============================================================================
// Configuration Loading
// =============================================================================

let cachedConfig: RelayerConfig | null = null;

/**
 * Load and validate configuration from environment
 * 
 * SECURITY: 
 * - Private key is decoded once and stored in memory only
 * - Config is frozen to prevent modification
 * - Validation errors reveal which field failed but not the value
 */
export function loadConfig(): RelayerConfig {
  // Return cached config if already loaded (singleton pattern)
  if (cachedConfig) {
    return cachedConfig;
  }

  // Validate environment
  const result = envSchema.safeParse(process.env);
  
  if (!result.success) {
    const errors = result.error.errors.map(e => `${e.path.join('.')}: ${e.message}`);
    throw new Error(`Configuration validation failed:\n${errors.join('\n')}`);
  }

  const env = result.data;

  // ==========================================================================
  // Parse Relayer Keypair - SECURITY CRITICAL
  // ==========================================================================
  const relayerKeypair = bs58.decode(env.RELAYER_PRIVATE_KEY);

  if (relayerKeypair.length !== 64) {
    throw new Error(`Invalid RELAYER_PRIVATE_KEY: decoded length ${relayerKeypair.length}, expected 64 bytes`);
  }

  try {
    Keypair.fromSecretKey(relayerKeypair);
  } catch (e) {
    throw new Error(`Invalid RELAYER_PRIVATE_KEY: Keypair.fromSecretKey failed: ${(e as Error).message}`);
  }

  // ==========================================================================
  // Parse Program ID
  // ==========================================================================
  let programId: PublicKey;
  try {
    programId = new PublicKey(env.PSOL_PROGRAM_ID);
  } catch {
    throw new Error('Invalid PSOL_PROGRAM_ID: not a valid Solana public key');
  }

  // ==========================================================================
  // Parse API Keys
  // ==========================================================================
  const apiKeys: string[] = [];
  if (env.RELAYER_API_KEYS) {
    const keys = env.RELAYER_API_KEYS.split(',').map(k => k.trim()).filter(k => k.length > 0);
    for (const key of keys) {
      if (key.length < 32) {
        throw new Error('API keys must be at least 32 characters for security');
      }
      apiKeys.push(key);
    }
  }

  // Warn if no API keys but auth is required
  if (env.REQUIRE_AUTH_FOR_WRITE && apiKeys.length === 0) {
    console.warn('WARNING: REQUIRE_AUTH_FOR_WRITE is true but no API keys configured');
  }

  // ==========================================================================
  // Parse Allowed IPs
  // ==========================================================================
  const allowedIps: string[] = [];
  if (env.ALLOWED_IPS) {
    const ips = env.ALLOWED_IPS.split(',').map(ip => ip.trim()).filter(ip => ip.length > 0);
    allowedIps.push(...ips);
  }

  // ==========================================================================
  // Parse CORS Origins
  // ==========================================================================
  const corsOrigins = env.CORS_ORIGINS === '*'
    ? ['*']
    : env.CORS_ORIGINS.split(',').map(o => o.trim()).filter(o => o.length > 0);

  // ==========================================================================
  // Generate WebSocket URL if not provided
  // ==========================================================================
  const wsUrl = env.SOLANA_WS_URL || env.SOLANA_RPC_URL
    .replace('https://', 'wss://')
    .replace('http://', 'ws://');

  // ==========================================================================
  // Build Config Object
  // ==========================================================================
  const config: RelayerConfig = {
    // Solana
    rpcUrl: env.SOLANA_RPC_URL,
    wsUrl,
    network: env.SOLANA_NETWORK,
    commitment: env.SOLANA_COMMITMENT,

    // Program
    programId,
    verificationKeyPath: env.VERIFICATION_KEY_PATH,

    // Wallet
    relayerKeypair,
    maxBalance: env.RELAYER_MAX_BALANCE,
    minBalance: env.RELAYER_MIN_BALANCE,

    // Fees
    baseFeeBps: env.BASE_FEE_BPS,
    minFeeLamports: env.MIN_FEE_LAMPORTS,
    maxFeeBps: env.MAX_FEE_BPS,
    enableDynamicFees: env.ENABLE_DYNAMIC_FEES,

    // Server
    port: env.PORT,
    host: env.HOST,
    corsOrigins,

    // Redis
    redisHost: env.REDIS_HOST,
    redisPort: env.REDIS_PORT,
    redisPassword: env.REDIS_PASSWORD,
    redisDb: env.REDIS_DB,

    // Rate Limiting
    rateLimitWindowMs: env.RATE_LIMIT_WINDOW_MS,
    rateLimitMaxRequests: env.RATE_LIMIT_MAX_REQUESTS,
    maxPendingPerIp: env.MAX_PENDING_PER_IP,
    maxPendingGlobal: env.MAX_PENDING_GLOBAL,

    // Authentication
    apiKeys,
    adminApiKey: env.ADMIN_API_KEY,
    allowedIps,
    requireAuthForWrite: env.REQUIRE_AUTH_FOR_WRITE,
    allowUnauthenticatedRead: env.ALLOW_UNAUTHENTICATED_READ,

    // Metrics
    enableMetrics: env.ENABLE_METRICS,
    metricsPath: env.METRICS_PATH,
  };

  // Freeze config to prevent accidental modification
  cachedConfig = Object.freeze(config) as RelayerConfig;
  
  return cachedConfig;
}

/**
 * Get the public key of the relayer (safe to log)
 */
export function getRelayerPublicKey(config: RelayerConfig): PublicKey {
  return Keypair.fromSecretKey(config.relayerKeypair).publicKey;
}

/**
 * Generate a key identifier from an API key (safe to log)
 * Returns first 8 chars of SHA256 hash
 */
export function getApiKeyId(apiKey: string): string {
  return createHash('sha256').update(apiKey).digest('hex').slice(0, 8);
}

/**
 * Check if running in production mode
 */
export function isProduction(): boolean {
  return process.env.NODE_ENV === 'production';
}

/**
 * Get network-specific defaults
 */
export function getNetworkDefaults(network: 'devnet' | 'testnet' | 'mainnet-beta'): {
  rpcUrl: string;
  programId: string;
  minBalance: number;
} {
  switch (network) {
    case 'mainnet-beta':
      return {
        rpcUrl: 'https://api.mainnet-beta.solana.com',
        programId: '', // Must be configured
        minBalance: 1.0, // Higher for mainnet
      };
    case 'testnet':
      return {
        rpcUrl: 'https://api.testnet.solana.com',
        programId: '',
        minBalance: 0.5,
      };
    case 'devnet':
    default:
      return {
        rpcUrl: 'https://api.devnet.solana.com',
        programId: 'DRDywerR6UCmvGCTTDf6aDefj7rLBKBQEp44DHrMeWkF',
        minBalance: 0.1,
      };
  }
}
