/**
 * Rate Limiting - Redis-based Token Bucket
 * 
 * Features:
 * 1. Per-IP rate limiting with sliding window
 * 2. Persistent across restarts (Redis-backed)
 * 3. Configurable per endpoint
 * 4. Graceful degradation if Redis unavailable
 */

import { Request, Response, NextFunction } from 'express';
import Redis from 'ioredis';
import { RelayerConfig, RelayerErrorCode, ErrorCodeToHttpStatus, ApiResponse } from './types.js';
import { logger, logSecurityEvent } from './logger.js';
import { getClientIp } from './auth.js';

// Redis key prefixes
const RATE_LIMIT_PREFIX = 'psol:relayer:ratelimit:';

let redis: Redis | null = null;

// =============================================================================
// Initialization
// =============================================================================

/**
 * Initialize rate limiter with Redis connection
 */
export function initRateLimiter(redisClient: Redis): void {
  redis = redisClient;
  logger.info('Rate limiter initialized');
}

// =============================================================================
// Rate Limiting Logic
// =============================================================================

interface RateLimitResult {
  allowed: boolean;
  remaining: number;
  resetAt: number;
  retryAfter?: number;
}

/**
 * Check rate limit using Redis sliding window
 * Uses sorted set with timestamp scores for accurate windowing
 */
async function checkRateLimit(
  identifier: string,
  windowMs: number,
  maxRequests: number
): Promise<RateLimitResult> {
  if (!redis || redis.status !== 'ready') {
    // Fail open if Redis unavailable (but log warning)
    logger.warn('Rate limiter: Redis unavailable, allowing request');
    return {
      allowed: true,
      remaining: maxRequests,
      resetAt: Date.now() + windowMs,
    };
  }

  const key = `${RATE_LIMIT_PREFIX}${identifier}`;
  const now = Date.now();
  const windowStart = now - windowMs;

  try {
    // Use Lua script for atomic operation
    const luaScript = `
      local key = KEYS[1]
      local now = tonumber(ARGV[1])
      local window_start = tonumber(ARGV[2])
      local max_requests = tonumber(ARGV[3])
      local window_ms = tonumber(ARGV[4])
      
      -- Remove old entries outside window
      redis.call('ZREMRANGEBYSCORE', key, '-inf', window_start)
      
      -- Count current requests in window
      local current_count = redis.call('ZCARD', key)
      
      if current_count < max_requests then
        -- Add new request
        redis.call('ZADD', key, now, now .. ':' .. math.random())
        -- Set expiry on key
        redis.call('PEXPIRE', key, window_ms)
        return {1, max_requests - current_count - 1, now + window_ms}
      else
        -- Get oldest entry for retry-after calculation
        local oldest = redis.call('ZRANGE', key, 0, 0, 'WITHSCORES')
        local retry_after = 0
        if #oldest >= 2 then
          retry_after = tonumber(oldest[2]) + window_ms - now
        end
        return {0, 0, now + window_ms, retry_after}
      end
    `;

    const result = await redis.eval(
      luaScript,
      1,
      key,
      now.toString(),
      windowStart.toString(),
      maxRequests.toString(),
      windowMs.toString()
    ) as number[];

    return {
      allowed: result[0] === 1,
      remaining: result[1],
      resetAt: result[2],
      retryAfter: result[3] ? Math.ceil(result[3] / 1000) : undefined,
    };
  } catch (error) {
    logger.error({ err: error }, 'Rate limit check failed');
    // Fail open on error
    return {
      allowed: true,
      remaining: maxRequests,
      resetAt: Date.now() + windowMs,
    };
  }
}

// =============================================================================
// Middleware
// =============================================================================

/**
 * Send rate limit error response
 */
function sendRateLimitError(
  res: Response,
  result: RateLimitResult,
  requestId?: string
): void {
  const response: ApiResponse = {
    success: false,
    error: {
      code: RelayerErrorCode.RATE_LIMITED,
      message: 'Too many requests',
      details: {
        retryAfter: result.retryAfter,
      },
    },
    meta: {
      requestId: requestId || 'unknown',
      timestamp: Date.now(),
    },
  };

  res.set({
    'X-RateLimit-Remaining': '0',
    'X-RateLimit-Reset': result.resetAt.toString(),
    'Retry-After': (result.retryAfter || 60).toString(),
  });

  res.status(429).json(response);
}

/**
 * Create rate limiting middleware
 */
export function createRateLimitMiddleware(config: RelayerConfig) {
  return async function rateLimitMiddleware(
    req: Request,
    res: Response,
    next: NextFunction
  ): Promise<void> {
    const clientIp = getClientIp(req);
    const requestId = req.requestId || 'unknown';

    // Skip rate limiting for health checks
    if (req.path === '/health') {
      next();
      return;
    }

    try {
      const result = await checkRateLimit(
        `ip:${clientIp}`,
        config.rateLimitWindowMs,
        config.rateLimitMaxRequests
      );

      // Set rate limit headers
      res.set({
        'X-RateLimit-Limit': config.rateLimitMaxRequests.toString(),
        'X-RateLimit-Remaining': result.remaining.toString(),
        'X-RateLimit-Reset': result.resetAt.toString(),
      });

      if (!result.allowed) {
        logSecurityEvent('rate_limited', clientIp, {
          path: req.path,
          method: req.method,
        });
        sendRateLimitError(res, result, requestId);
        return;
      }

      next();
    } catch (error) {
      // Log but don't block on rate limit errors
      logger.error({ err: error }, 'Rate limit middleware error');
      next();
    }
  };
}

/**
 * Create endpoint-specific rate limiter
 * For stricter limits on sensitive endpoints
 */
export function createEndpointRateLimiter(
  windowMs: number,
  maxRequests: number,
  keyPrefix: string = 'endpoint'
) {
  return async function endpointRateLimiter(
    req: Request,
    res: Response,
    next: NextFunction
  ): Promise<void> {
    const clientIp = getClientIp(req);
    const requestId = req.requestId || 'unknown';

    try {
      const result = await checkRateLimit(
        `${keyPrefix}:${clientIp}:${req.path}`,
        windowMs,
        maxRequests
      );

      if (!result.allowed) {
        logSecurityEvent('rate_limited', clientIp, {
          path: req.path,
          keyPrefix,
          windowMs,
          maxRequests,
        });
        sendRateLimitError(res, result, requestId);
        return;
      }

      next();
    } catch (error) {
      logger.error({ err: error }, 'Endpoint rate limit error');
      next();
    }
  };
}
