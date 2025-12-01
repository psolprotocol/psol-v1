/**
 * Authentication Middleware - API Key & IP Allowlist
 * 
 * SECURITY:
 * 1. API keys are compared using timing-safe comparison
 * 2. API keys are never logged - only key IDs (hash prefix)
 * 3. Failed auth attempts are logged for monitoring
 * 4. IP allowlist checked before API key validation
 */

import { Request, Response, NextFunction } from 'express';
import { createHash, timingSafeEqual } from 'crypto';
import { RelayerConfig, AuthContext, RelayerErrorCode, ErrorCodeToHttpStatus, ApiResponse } from './types.js';
import { logger, logSecurityEvent } from './logger.js';
import { getApiKeyId } from './config.js';

// Extend Express Request to include auth context
declare global {
  namespace Express {
    interface Request {
      auth?: AuthContext;
      requestId?: string;
    }
  }
}

// =============================================================================
// Helper Functions
// =============================================================================

/**
 * Extract client IP from request
 * Handles X-Forwarded-For header from reverse proxies
 */
export function getClientIp(req: Request): string {
  const forwarded = req.headers['x-forwarded-for'];
  if (typeof forwarded === 'string') {
    // Take first IP in chain (original client)
    return forwarded.split(',')[0].trim();
  }
  if (Array.isArray(forwarded) && forwarded.length > 0) {
    return forwarded[0].split(',')[0].trim();
  }
  return req.socket.remoteAddress || 'unknown';
}

/**
 * Extract API key from request
 * Supports Authorization header (Bearer) and X-API-Key header
 */
function extractApiKey(req: Request): string | null {
  // Check Authorization header first
  const authHeader = req.headers.authorization;
  if (authHeader) {
    const parts = authHeader.split(' ');
    if (parts.length === 2 && parts[0].toLowerCase() === 'bearer') {
      return parts[1];
    }
  }
  
  // Check X-API-Key header
  const apiKeyHeader = req.headers['x-api-key'];
  if (typeof apiKeyHeader === 'string' && apiKeyHeader.length > 0) {
    return apiKeyHeader;
  }
  
  return null;
}

/**
 * Timing-safe API key comparison
 * Prevents timing attacks by ensuring constant-time comparison
 */
function isValidApiKey(providedKey: string, validKeys: string[]): { valid: boolean; keyId?: string } {
  const providedBuffer = Buffer.from(providedKey);
  
  for (const validKey of validKeys) {
    const validBuffer = Buffer.from(validKey);
    
    // Keys must be same length for timing-safe comparison
    if (providedBuffer.length === validBuffer.length) {
      if (timingSafeEqual(providedBuffer, validBuffer)) {
        return { valid: true, keyId: getApiKeyId(validKey) };
      }
    }
  }
  
  return { valid: false };
}

/**
 * Check if IP is in allowlist
 */
function isIpAllowed(clientIp: string, allowedIps: string[]): boolean {
  // Empty allowlist means all IPs allowed
  if (allowedIps.length === 0) return true;
  
  // Check for exact match
  if (allowedIps.includes(clientIp)) return true;
  
  // Check for CIDR ranges (simplified - just /24 support)
  for (const allowed of allowedIps) {
    if (allowed.endsWith('/24')) {
      const prefix = allowed.slice(0, -3);
      const clientPrefix = clientIp.split('.').slice(0, 3).join('.');
      const allowedPrefix = prefix.split('.').slice(0, 3).join('.');
      if (clientPrefix === allowedPrefix) return true;
    }
  }
  
  return false;
}

/**
 * Send standardized error response
 */
function sendAuthError(
  res: Response,
  errorCode: RelayerErrorCode,
  message: string,
  requestId?: string
): void {
  const status = ErrorCodeToHttpStatus[errorCode];
  const response: ApiResponse = {
    success: false,
    error: {
      code: errorCode,
      message,
    },
    meta: {
      requestId: requestId || 'unknown',
      timestamp: Date.now(),
    },
  };
  res.status(status).json(response);
}

// =============================================================================
// Middleware Factory
// =============================================================================

/**
 * Create authentication middleware
 */
export function createAuthMiddleware(config: RelayerConfig) {
  return function authMiddleware(req: Request, res: Response, next: NextFunction): void {
    const clientIp = getClientIp(req);
    const requestId = req.requestId || 'unknown';
    
    // Initialize auth context
    const authContext: AuthContext = {
      authenticated: false,
      clientIp,
      isAdmin: false,
    };
    
    // =======================================================================
    // Step 1: IP Allowlist Check (if configured)
    // =======================================================================
    if (config.allowedIps.length > 0 && !isIpAllowed(clientIp, config.allowedIps)) {
      logSecurityEvent('ip_blocked', clientIp, { path: req.path });
      sendAuthError(res, RelayerErrorCode.IP_NOT_ALLOWED, 'IP not in allowlist', requestId);
      return;
    }
    
    // =======================================================================
    // Step 2: Extract and Validate API Key
    // =======================================================================
    const providedKey = extractApiKey(req);
    
    if (providedKey) {
      // Check admin key first
      if (config.adminApiKey) {
        const adminBuffer = Buffer.from(config.adminApiKey);
        const providedBuffer = Buffer.from(providedKey);
        
        if (providedBuffer.length === adminBuffer.length && 
            timingSafeEqual(providedBuffer, adminBuffer)) {
          authContext.authenticated = true;
          authContext.isAdmin = true;
          authContext.apiKeyId = 'admin';
          req.auth = authContext;
          next();
          return;
        }
      }
      
      // Check regular API keys
      const result = isValidApiKey(providedKey, config.apiKeys);
      if (result.valid) {
        authContext.authenticated = true;
        authContext.apiKeyId = result.keyId;
        req.auth = authContext;
        next();
        return;
      }
      
      // Invalid key provided
      logSecurityEvent('invalid_api_key', clientIp, { 
        path: req.path,
        keyPrefix: providedKey.slice(0, 4) + '...',
      });
      sendAuthError(res, RelayerErrorCode.INVALID_API_KEY, 'Invalid API key', requestId);
      return;
    }
    
    // =======================================================================
    // Step 3: Handle Unauthenticated Requests
    // =======================================================================
    const isWriteMethod = ['POST', 'PUT', 'PATCH', 'DELETE'].includes(req.method);
    
    // Check if auth is required for this request
    if (isWriteMethod && config.requireAuthForWrite) {
      logSecurityEvent('auth_failed', clientIp, { 
        path: req.path,
        method: req.method,
        reason: 'no_api_key',
      });
      sendAuthError(res, RelayerErrorCode.UNAUTHORIZED, 'API key required', requestId);
      return;
    }
    
    if (!isWriteMethod && !config.allowUnauthenticatedRead) {
      logSecurityEvent('auth_failed', clientIp, { 
        path: req.path,
        method: req.method,
        reason: 'read_auth_required',
      });
      sendAuthError(res, RelayerErrorCode.UNAUTHORIZED, 'Authentication required', requestId);
      return;
    }
    
    // Unauthenticated request allowed
    req.auth = authContext;
    next();
  };
}

/**
 * Middleware to require admin access
 */
export function requireAdmin(req: Request, res: Response, next: NextFunction): void {
  if (!req.auth?.isAdmin) {
    const requestId = req.requestId || 'unknown';
    sendAuthError(res, RelayerErrorCode.FORBIDDEN, 'Admin access required', requestId);
    return;
  }
  next();
}

/**
 * Middleware to require authentication (any valid API key)
 */
export function requireAuth(req: Request, res: Response, next: NextFunction): void {
  if (!req.auth?.authenticated) {
    const requestId = req.requestId || 'unknown';
    sendAuthError(res, RelayerErrorCode.UNAUTHORIZED, 'Authentication required', requestId);
    return;
  }
  next();
}
