/**
 * Logger - Security-Hardened for Mainnet
 * 
 * SECURITY RULES:
 * 1. NEVER log private keys, seeds, or full keypairs
 * 2. NEVER log full proof data (proofData field)
 * 3. NEVER log full nullifier hashes - truncate to first 8 chars
 * 4. NEVER log API keys - only key identifiers
 * 5. Include correlation IDs (requestId) in all request logs
 * 6. Hash client IPs in production for privacy
 */

import pino, { Logger } from 'pino';
import { createHash } from 'crypto';

// =============================================================================
// Configuration
// =============================================================================

const LOG_LEVEL = process.env.LOG_LEVEL || 'info';
const IS_PRODUCTION = process.env.NODE_ENV === 'production';

// =============================================================================
// Sensitive Data Redaction
// =============================================================================

/**
 * Truncate sensitive hashes for logging
 * Shows first 8 chars only: "abc123..." 
 */
export function truncateHash(hash: string | Uint8Array): string {
  const str = typeof hash === 'string' ? hash : Buffer.from(hash).toString('hex');
  if (str.length <= 8) return str;
  return `${str.slice(0, 8)}...`;
}

/**
 * Hash IP address for privacy in production
 */
export function hashIp(ip: string): string {
  if (!IS_PRODUCTION) return ip;
  return createHash('sha256').update(ip).digest('hex').slice(0, 12);
}

/**
 * Redact sensitive fields from an object before logging
 */
export function redactSensitive<T extends Record<string, unknown>>(obj: T): Record<string, unknown> {
  const redacted: Record<string, unknown> = {};
  
  for (const [key, value] of Object.entries(obj)) {
    const lowerKey = key.toLowerCase();
    
    // Completely redact these fields
    if (
      lowerKey.includes('private') ||
      lowerKey.includes('secret') ||
      lowerKey.includes('password') ||
      lowerKey.includes('keypair') ||
      lowerKey.includes('seed') ||
      lowerKey === 'proofdata' ||
      lowerKey === 'proof_data' ||
      lowerKey === 'apikey' ||
      lowerKey === 'api_key'
    ) {
      redacted[key] = '[REDACTED]';
      continue;
    }
    
    // Truncate hash fields
    if (
      lowerKey.includes('nullifier') ||
      lowerKey.includes('commitment') ||
      lowerKey.includes('root')
    ) {
      if (typeof value === 'string') {
        redacted[key] = truncateHash(value);
      } else if (value instanceof Uint8Array) {
        redacted[key] = truncateHash(value);
      } else {
        redacted[key] = value;
      }
      continue;
    }
    
    // Hash IPs in production
    if (lowerKey === 'clientip' || lowerKey === 'client_ip' || lowerKey === 'ip') {
      if (typeof value === 'string') {
        redacted[key] = hashIp(value);
      } else {
        redacted[key] = value;
      }
      continue;
    }
    
    // Recursively handle nested objects
    if (value && typeof value === 'object' && !Array.isArray(value) && !(value instanceof Uint8Array)) {
      redacted[key] = redactSensitive(value as Record<string, unknown>);
      continue;
    }
    
    redacted[key] = value;
  }
  
  return redacted;
}

// =============================================================================
// Logger Instance
// =============================================================================

const baseLogger = pino({
  level: LOG_LEVEL,
  transport: !IS_PRODUCTION
    ? {
        target: 'pino-pretty',
        options: {
          colorize: true,
          translateTime: 'SYS:standard',
          ignore: 'pid,hostname',
        },
      }
    : undefined,
  base: {
    service: 'psol-relayer',
    env: process.env.NODE_ENV || 'development',
  },
  formatters: {
    level: (label) => ({ level: label }),
  },
  // Redact sensitive fields at serialization
  redact: {
    paths: [
      'relayerKeypair',
      'privateKey',
      'secretKey',
      'proofData',
      'proof_data',
      'apiKey',
      'api_key',
      '*.relayerKeypair',
      '*.privateKey',
      '*.proofData',
    ],
    censor: '[REDACTED]',
  },
});

export const logger = baseLogger;

// =============================================================================
// Specialized Loggers
// =============================================================================

/**
 * Create a child logger with request context
 */
export function createRequestLogger(requestId: string, clientIp?: string): Logger {
  return logger.child({
    requestId,
    clientIp: clientIp ? hashIp(clientIp) : undefined,
  });
}

/**
 * Create a child logger for job processing
 */
export function createJobLogger(jobId: string): Logger {
  return logger.child({
    jobId: jobId.slice(0, 8), // Truncate job ID for brevity
  });
}

// =============================================================================
// Structured Log Events
// =============================================================================

/**
 * Log a job lifecycle event
 * SECURITY: Automatically redacts sensitive fields
 */
export function logJobEvent(
  jobId: string,
  event: 'created' | 'processing' | 'submitting' | 'succeeded' | 'failed' | 'retrying',
  data?: Record<string, unknown>
): void {
  const safeData = data ? redactSensitive(data) : {};
  logger.info({
    jobId: jobId.slice(0, 8),
    event,
    ...safeData,
  }, `Job ${event}`);
}

/**
 * Log a transaction event
 */
export function logTransaction(
  action: 'building' | 'submitting' | 'confirmed' | 'failed',
  signature?: string,
  data?: Record<string, unknown>
): void {
  const safeData = data ? redactSensitive(data) : {};
  logger.info({
    action,
    signature: signature ? `${signature.slice(0, 16)}...` : undefined,
    ...safeData,
  }, `Transaction: ${action}`);
}

/**
 * Log an API request
 */
export function logRequest(
  requestId: string,
  method: string,
  path: string,
  statusCode: number,
  durationMs: number,
  clientIp?: string
): void {
  const level = statusCode >= 500 ? 'error' : statusCode >= 400 ? 'warn' : 'info';
  logger[level]({
    requestId,
    method,
    path,
    statusCode,
    durationMs,
    clientIp: clientIp ? hashIp(clientIp) : undefined,
  }, `${method} ${path} ${statusCode}`);
}

/**
 * Log an error with context
 * SECURITY: Redacts sensitive data in error context
 */
export function logError(
  error: Error,
  context?: Record<string, unknown>
): void {
  const safeContext = context ? redactSensitive(context) : {};
  logger.error({
    err: {
      message: error.message,
      name: error.name,
      // Don't log full stack in production
      stack: IS_PRODUCTION ? undefined : error.stack,
    },
    ...safeContext,
  }, error.message);
}

/**
 * Log a security event (auth failures, rate limits, etc.)
 */
export function logSecurityEvent(
  event: 'auth_failed' | 'rate_limited' | 'ip_blocked' | 'invalid_api_key' | 'suspicious_activity',
  clientIp: string,
  details?: Record<string, unknown>
): void {
  logger.warn({
    securityEvent: event,
    clientIp: hashIp(clientIp),
    ...details,
  }, `Security: ${event}`);
}

/**
 * Log startup information
 * SECURITY: Never log private key, only public address
 */
export function logStartup(config: {
  network: string;
  rpcUrl: string;
  relayerAddress: string;
  port: number;
}): void {
  logger.info({
    network: config.network,
    // Truncate RPC URL to hide API keys that might be in it
    rpcUrl: config.rpcUrl.split('?')[0],
    relayerAddress: config.relayerAddress,
    port: config.port,
  }, 'Relayer starting');
}

export default logger;
