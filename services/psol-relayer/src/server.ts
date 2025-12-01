/**
 * API Server - Hardened for Mainnet
 * 
 * Security features:
 * 1. API key authentication for write endpoints
 * 2. IP allowlist support
 * 3. Redis-based rate limiting
 * 4. Standardized error responses
 * 5. Request correlation IDs
 * 6. Privacy-safe logging
 */

import express, { Request, Response, NextFunction } from 'express';
import helmet from 'helmet';
import { Connection, PublicKey, Keypair, LAMPORTS_PER_SOL } from '@solana/web3.js';
import { Queue } from 'bullmq';
import BN from 'bn.js';
import { v4 as uuidv4 } from 'uuid';
import { z } from 'zod';
import { logger, logRequest, truncateHash } from './logger.js';
import { createAuthMiddleware, getClientIp, requireAdmin } from './auth.js';
import { createRateLimitMiddleware, createEndpointRateLimiter } from './rateLimit.js';
import { getFeeQuote, validateFee } from './fees.js';
import { validateWithdrawalRequest, isNullifierSpent, validatePool } from './validation.js';
import { submitJob, getJob, getTotalPendingCount } from './queue.js';
import { isJobStoreConnected, getJobByNullifier, getRecentJobs } from './jobStore.js';
import { getMetrics, getPrometheusMetrics, recordRequest, updateRelayerBalance } from './metrics.js';
import { getRelayerPublicKey } from './config.js';
import type {
  RelayerConfig,
  WithdrawalRequest,
  WithdrawalRequestValidated,
  ApiResponse,
  SubmitResponseData,
  JobStatusResponseData,
  FeeQuoteResponseData,
  RelayerInfoResponseData,
  HealthResponseData,
  RelayerErrorCode,
  ErrorCodeToHttpStatus,
} from './types.js';

// =============================================================================
// Request Schemas
// =============================================================================

const withdrawalRequestSchema = z.object({
  poolAddress: z.string().min(32).max(44),
  tokenMint: z.string().min(32).max(44),
  proofData: z.string().regex(/^[0-9a-fA-F]+$/).min(256), // At least 128 bytes hex
  merkleRoot: z.string().regex(/^[0-9a-fA-F]{64}$/),
  nullifierHash: z.string().regex(/^[0-9a-fA-F]{64}$/),
  recipient: z.string().min(32).max(44),
  amount: z.string().regex(/^\d+$/),
  relayerFee: z.string().regex(/^\d+$/),
});

const feeQuoteSchema = z.object({
  poolAddress: z.string().min(32).max(44),
  amount: z.string().regex(/^\d+$/),
});

const nullifierCheckSchema = z.object({
  poolAddress: z.string().min(32).max(44),
  nullifierHash: z.string().regex(/^[0-9a-fA-F]{64}$/),
});

// =============================================================================
// Response Helpers
// =============================================================================

function sendSuccess<T>(res: Response, data: T, requestId: string, statusCode: number = 200): void {
  const response: ApiResponse<T> = {
    success: true,
    data,
    meta: {
      requestId,
      timestamp: Date.now(),
    },
  };
  res.status(statusCode).json(response);
}

function sendError(
  res: Response,
  code: RelayerErrorCode,
  message: string,
  requestId: string,
  details?: unknown
): void {
  const statusMap: Record<string, number> = {
    INVALID_REQUEST: 400,
    INVALID_PROOF: 400,
    INVALID_POOL: 400,
    INVALID_AMOUNT: 400,
    NULLIFIER_SPENT: 400,
    NULLIFIER_PENDING: 409,
    INSUFFICIENT_FEE: 400,
    POOL_PAUSED: 400,
    ROOT_NOT_FOUND: 400,
    UNAUTHORIZED: 401,
    FORBIDDEN: 403,
    INVALID_API_KEY: 401,
    IP_NOT_ALLOWED: 403,
    RATE_LIMITED: 429,
    TOO_MANY_PENDING: 429,
    QUEUE_FULL: 503,
    INTERNAL_ERROR: 500,
    RELAYER_PAUSED: 503,
    LOW_BALANCE: 503,
    SUBMISSION_FAILED: 500,
    RPC_ERROR: 502,
    REDIS_ERROR: 503,
    TIMEOUT: 504,
    JOB_NOT_FOUND: 404,
    JOB_ALREADY_EXISTS: 409,
  };

  const status = statusMap[code] || 500;
  
  const response: ApiResponse = {
    success: false,
    error: {
      code: code as RelayerErrorCode,
      message,
      details,
    },
    meta: {
      requestId,
      timestamp: Date.now(),
    },
  };
  
  res.status(status).json(response);
}

// =============================================================================
// Server Factory
// =============================================================================

export function createServer(
  config: RelayerConfig,
  connection: Connection,
  queue: Queue
) {
  const app = express();
  const relayerKeypair = Keypair.fromSecretKey(config.relayerKeypair);
  const relayerPublicKey = getRelayerPublicKey(config);

  // =========================================================================
  // Middleware Stack
  // =========================================================================

  // Security headers
  app.use(helmet({
    contentSecurityPolicy: false, // API server, not serving HTML
  }));

  // Body parsing
  app.use(express.json({ limit: '100kb' }));

  // Request ID middleware
  app.use((req, res, next) => {
    req.requestId = req.headers['x-request-id'] as string || uuidv4();
    res.setHeader('X-Request-Id', req.requestId);
    next();
  });

  // CORS
  app.use((req, res, next) => {
    const origin = req.headers.origin || '*';
    if (config.corsOrigins.includes('*') || config.corsOrigins.includes(origin)) {
      res.header('Access-Control-Allow-Origin', origin);
    }
    res.header('Access-Control-Allow-Methods', 'GET, POST, OPTIONS');
    res.header('Access-Control-Allow-Headers', 'Content-Type, Authorization, X-API-Key, X-Request-Id');
    if (req.method === 'OPTIONS') {
      return res.sendStatus(200);
    }
    next();
  });

  // Request logging
  app.use((req, res, next) => {
    const start = Date.now();
    res.on('finish', () => {
      logRequest(
        req.requestId!,
        req.method,
        req.path,
        res.statusCode,
        Date.now() - start,
        getClientIp(req)
      );
      recordRequest(req.path);
    });
    next();
  });

  // Authentication
  app.use(createAuthMiddleware(config));

  // Rate limiting (global)
  app.use(createRateLimitMiddleware(config));

  // =========================================================================
  // Health & Info Endpoints (Read-only, optionally unauthenticated)
  // =========================================================================

  app.get('/health', async (req, res) => {
    const requestId = req.requestId!;
    
    try {
      const [slot, balance, pendingJobs] = await Promise.all([
        connection.getSlot().catch(() => 0),
        connection.getBalance(relayerPublicKey).catch(() => 0),
        getTotalPendingCount().catch(() => 0),
      ]);

      const redisConnected = isJobStoreConnected();
      const balanceSol = balance / LAMPORTS_PER_SOL;
      
      // Determine health status
      let status: 'healthy' | 'degraded' | 'unhealthy' = 'healthy';
      if (!redisConnected) status = 'unhealthy';
      else if (balanceSol < config.minBalance) status = 'degraded';
      else if (slot === 0) status = 'degraded';

      // Update balance metric
      updateRelayerBalance(balance);

      const data: HealthResponseData = {
        status,
        slot,
        relayerBalance: balanceSol,
        redisConnected,
        pendingJobs,
        timestamp: Date.now(),
      };

      const statusCode = status === 'healthy' ? 200 : status === 'degraded' ? 200 : 503;
      sendSuccess(res, data, requestId, statusCode);
    } catch (error) {
      sendError(res, 'INTERNAL_ERROR' as RelayerErrorCode, 'Health check failed', requestId);
    }
  });

  app.get('/info', async (req, res) => {
    const requestId = req.requestId!;
    
    try {
      const [balance, metrics] = await Promise.all([
        connection.getBalance(relayerPublicKey),
        Promise.resolve(getMetrics()),
      ]);
      
      const balanceSol = balance / LAMPORTS_PER_SOL;
      updateRelayerBalance(balance);

      let status: 'active' | 'paused' | 'low_balance' = 'active';
      if (balanceSol < config.minBalance) status = 'low_balance';

      const data: RelayerInfoResponseData = {
        address: relayerPublicKey.toString(),
        balance: balanceSol.toFixed(4),
        network: config.network,
        feeBps: config.baseFeeBps,
        minFee: config.minFeeLamports.toString(),
        status,
        pendingJobs: metrics.pendingJobs,
        totalProcessed: metrics.jobsSucceeded + metrics.jobsFailed,
        successRate: metrics.successRate,
      };

      sendSuccess(res, data, requestId);
    } catch (error) {
      sendError(res, 'INTERNAL_ERROR' as RelayerErrorCode, 'Failed to get relayer info', requestId);
    }
  });

  // =========================================================================
  // Metrics Endpoint
  // =========================================================================

  app.get(config.metricsPath, (req, res) => {
    const requestId = req.requestId!;
    const acceptHeader = req.headers.accept || '';

    // Return Prometheus format if requested
    if (acceptHeader.includes('text/plain') || req.query.format === 'prometheus') {
      res.setHeader('Content-Type', 'text/plain; charset=utf-8');
      res.send(getPrometheusMetrics());
      return;
    }

    // Default to JSON
    sendSuccess(res, getMetrics(), requestId);
  });

  // =========================================================================
  // Fee Endpoints
  // =========================================================================

  // Stricter rate limit for fee quotes
  const feeQuoteRateLimiter = createEndpointRateLimiter(60000, 30, 'fee');

  app.post('/fee/quote', feeQuoteRateLimiter, async (req, res) => {
    const requestId = req.requestId!;
    
    try {
      const parsed = feeQuoteSchema.safeParse(req.body);
      if (!parsed.success) {
        return sendError(
          res,
          'INVALID_REQUEST' as RelayerErrorCode,
          'Invalid request',
          requestId,
          parsed.error.errors
        );
      }

      const { poolAddress, amount } = parsed.data;
      const amountBn = new BN(amount);

      // Validate pool exists
      const poolPubkey = new PublicKey(poolAddress);
      const poolValidation = await validatePool(connection, poolPubkey);
      if (!poolValidation.valid) {
        return sendError(
          res,
          'INVALID_POOL' as RelayerErrorCode,
          poolValidation.error!,
          requestId
        );
      }

      const quote = await getFeeQuote(connection, config, amountBn);
      
      const data: FeeQuoteResponseData = quote;
      sendSuccess(res, data, requestId);
    } catch (error) {
      logger.error({ err: error, requestId }, 'Fee quote error');
      sendError(res, 'INTERNAL_ERROR' as RelayerErrorCode, 'Failed to get fee quote', requestId);
    }
  });

  // =========================================================================
  // Withdrawal Endpoints
  // =========================================================================

  // Stricter rate limit for withdrawals
  const withdrawRateLimiter = createEndpointRateLimiter(60000, 10, 'withdraw');

  app.post('/withdraw', withdrawRateLimiter, async (req, res) => {
    const requestId = req.requestId!;
    const clientIp = getClientIp(req);

    try {
      // Parse and validate request
      const parsed = withdrawalRequestSchema.safeParse(req.body);
      if (!parsed.success) {
        return sendError(
          res,
          'INVALID_REQUEST' as RelayerErrorCode,
          'Invalid request format',
          requestId,
          parsed.error.errors.map(e => ({ path: e.path.join('.'), message: e.message }))
        );
      }

      const body = parsed.data;

      // Check if nullifier is already spent on-chain
      const nullifierSpent = await isNullifierSpent(
        connection,
        config.programId,
        new PublicKey(body.poolAddress),
        Buffer.from(body.nullifierHash, 'hex')
      );
      if (nullifierSpent) {
        return sendError(
          res,
          'NULLIFIER_SPENT' as RelayerErrorCode,
          'Nullifier already spent',
          requestId
        );
      }

      // Parse request into validated format
      const request: WithdrawalRequestValidated = {
        poolAddress: new PublicKey(body.poolAddress),
        tokenMint: new PublicKey(body.tokenMint),
        proofData: Buffer.from(body.proofData, 'hex'),
        merkleRoot: Buffer.from(body.merkleRoot, 'hex'),
        nullifierHash: Buffer.from(body.nullifierHash, 'hex'),
        recipient: new PublicKey(body.recipient),
        amount: new BN(body.amount),
        relayerFee: new BN(body.relayerFee),
      };

      // Validate fee before queueing
      const feeValidation = await validateFee(connection, config, request.amount, request.relayerFee);
      if (!feeValidation.valid) {
        return sendError(
          res,
          'INSUFFICIENT_FEE' as RelayerErrorCode,
          feeValidation.error!,
          requestId,
          { minimumFee: feeValidation.minimumFee?.toString() }
        );
      }

      // Submit job
      const result = await submitJob(queue, config, request, {
        clientIp,
        clientId: req.auth?.apiKeyId,
      });

      if (result.error) {
        return sendError(res, result.error.code, result.error.message, requestId);
      }

      const data: SubmitResponseData = {
        jobId: result.job!.id,
        status: result.job!.status,
        estimatedTime: 30,
      };

      sendSuccess(res, data, requestId, 202);
    } catch (error) {
      logger.error({ err: error, requestId }, 'Withdrawal submission error');
      sendError(res, 'INTERNAL_ERROR' as RelayerErrorCode, 'Failed to process withdrawal', requestId);
    }
  });

  app.get('/withdraw/:jobId', async (req, res) => {
    const requestId = req.requestId!;
    const { jobId } = req.params;

    try {
      const job = await getJob(jobId);
      if (!job) {
        return sendError(res, 'JOB_NOT_FOUND' as RelayerErrorCode, 'Job not found', requestId);
      }

      const data: JobStatusResponseData = {
        jobId: job.id,
        status: job.status,
        txSignature: job.txSignature,
        errorCode: job.errorCode,
        errorMessage: job.errorMessage,
        createdAt: job.createdAt,
        updatedAt: job.updatedAt,
      };

      sendSuccess(res, data, requestId);
    } catch (error) {
      logger.error({ err: error, requestId }, 'Job status error');
      sendError(res, 'INTERNAL_ERROR' as RelayerErrorCode, 'Failed to get job status', requestId);
    }
  });

  // =========================================================================
  // Validation Endpoints
  // =========================================================================

  app.post('/validate/nullifier', async (req, res) => {
    const requestId = req.requestId!;

    try {
      const parsed = nullifierCheckSchema.safeParse(req.body);
      if (!parsed.success) {
        return sendError(
          res,
          'INVALID_REQUEST' as RelayerErrorCode,
          'Invalid request',
          requestId,
          parsed.error.errors
        );
      }

      const { poolAddress, nullifierHash } = parsed.data;

      const spent = await isNullifierSpent(
        connection,
        config.programId,
        new PublicKey(poolAddress),
        Buffer.from(nullifierHash, 'hex')
      );

      sendSuccess(res, { spent }, requestId);
    } catch (error) {
      sendError(res, 'INTERNAL_ERROR' as RelayerErrorCode, 'Validation failed', requestId);
    }
  });

  // =========================================================================
  // Admin Endpoints (require admin API key)
  // =========================================================================

  app.get('/admin/jobs', requireAdmin, async (req, res) => {
    const requestId = req.requestId!;

    try {
      const limit = Math.min(parseInt(req.query.limit as string) || 20, 100);
      const jobs = await getRecentJobs(limit);
      
      // Redact sensitive info
      const sanitizedJobs = jobs.map(job => ({
        ...job,
        nullifierHash: truncateHash(job.nullifierHash),
      }));

      sendSuccess(res, { jobs: sanitizedJobs, count: sanitizedJobs.length }, requestId);
    } catch (error) {
      sendError(res, 'INTERNAL_ERROR' as RelayerErrorCode, 'Failed to get jobs', requestId);
    }
  });

  // =========================================================================
  // Error Handler
  // =========================================================================

  app.use((err: Error, req: Request, res: Response, next: NextFunction) => {
    const requestId = req.requestId || 'unknown';
    logger.error({ err, path: req.path, requestId }, 'Unhandled error');
    sendError(res, 'INTERNAL_ERROR' as RelayerErrorCode, 'Internal server error', requestId);
  });

  // 404 handler
  app.use((req, res) => {
    const requestId = req.requestId || 'unknown';
    sendError(res, 'INVALID_REQUEST' as RelayerErrorCode, `Endpoint not found: ${req.method} ${req.path}`, requestId);
  });

  return app;
}
