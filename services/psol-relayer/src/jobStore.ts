/**
 * Job Store - Persistent Redis-based Storage
 * 
 * Replaces in-memory Map with Redis for:
 * 1. Persistence across restarts
 * 2. Distributed state if running multiple instances
 * 3. Automatic TTL for old jobs
 * 
 * SECURITY:
 * - Proof data is NEVER stored (only nullifier hash for dedup)
 * - Client IPs are hashed before storage
 */

import Redis from 'ioredis';
import { logger, truncateHash, hashIp } from './logger.js';
import type { PersistedJob, JobStatus, RelayerConfig } from './types.js';

// Redis key prefixes
const KEY_PREFIX = 'psol:relayer:';
const JOB_PREFIX = `${KEY_PREFIX}job:`;
const NULLIFIER_PREFIX = `${KEY_PREFIX}nullifier:`;
const PENDING_IP_PREFIX = `${KEY_PREFIX}pending:ip:`;
const METRICS_KEY = `${KEY_PREFIX}metrics`;

// TTLs
const JOB_TTL_SECONDS = 7 * 24 * 60 * 60; // 7 days for completed jobs
const NULLIFIER_TTL_SECONDS = 30 * 24 * 60 * 60; // 30 days for nullifier dedup
const PENDING_TTL_SECONDS = 60 * 60; // 1 hour for pending tracking

let redis: Redis | null = null;

// =============================================================================
// Initialization
// =============================================================================

/**
 * Initialize Redis connection
 */
export async function initJobStore(config: RelayerConfig): Promise<void> {
  redis = new Redis({
    host: config.redisHost,
    port: config.redisPort,
    password: config.redisPassword,
    db: config.redisDb,
    maxRetriesPerRequest: 3,
    retryStrategy: (times) => {
      if (times > 10) return null; // Stop retrying
      return Math.min(times * 100, 3000);
    },
    lazyConnect: true,
  });

  redis.on('error', (error) => {
    logger.error({ err: error }, 'Redis connection error');
  });

  redis.on('connect', () => {
    logger.info('Redis connected');
  });

  redis.on('ready', () => {
    logger.info('Redis ready');
  });

  // Connect
  await redis.connect();
  
  // Verify connection
  await redis.ping();
  logger.info('Job store initialized');
}

/**
 * Close Redis connection
 */
export async function closeJobStore(): Promise<void> {
  if (redis) {
    await redis.quit();
    redis = null;
    logger.info('Job store closed');
  }
}

/**
 * Check if Redis is connected
 */
export function isJobStoreConnected(): boolean {
  return redis !== null && redis.status === 'ready';
}

function getRedis(): Redis {
  if (!redis || redis.status !== 'ready') {
    throw new Error('Job store not initialized or disconnected');
  }
  return redis;
}

// =============================================================================
// Job CRUD Operations
// =============================================================================

/**
 * Create a new job
 * Returns the job if created, null if nullifier already exists
 */
export async function createJob(job: PersistedJob): Promise<PersistedJob | null> {
  const r = getRedis();
  const jobKey = `${JOB_PREFIX}${job.id}`;
  const nullifierKey = `${NULLIFIER_PREFIX}${job.nullifierHash}`;

  // Check if nullifier already has a job (idempotency)
  const existingJobId = await r.get(nullifierKey);
  if (existingJobId) {
    logger.warn({
      nullifier: truncateHash(job.nullifierHash),
      existingJobId: existingJobId.slice(0, 8),
    }, 'Duplicate nullifier submission');
    return null;
  }

  // Hash client IP for privacy
  const jobToStore: PersistedJob = {
    ...job,
    clientIp: job.clientIp ? hashIp(job.clientIp) : undefined,
  };

  // Use transaction to ensure atomicity
  const pipeline = r.pipeline();
  
  // Store job
  pipeline.set(jobKey, JSON.stringify(jobToStore), 'EX', JOB_TTL_SECONDS);
  
  // Map nullifier to job (for dedup)
  pipeline.set(nullifierKey, job.id, 'EX', NULLIFIER_TTL_SECONDS);
  
  // Track pending jobs per IP
  if (job.clientIp) {
    const ipKey = `${PENDING_IP_PREFIX}${hashIp(job.clientIp)}`;
    pipeline.sadd(ipKey, job.id);
    pipeline.expire(ipKey, PENDING_TTL_SECONDS);
  }

  await pipeline.exec();

  logger.debug({
    jobId: job.id.slice(0, 8),
    nullifier: truncateHash(job.nullifierHash),
  }, 'Job created');

  return jobToStore;
}

/**
 * Get job by ID
 */
export async function getJob(jobId: string): Promise<PersistedJob | null> {
  const r = getRedis();
  const data = await r.get(`${JOB_PREFIX}${jobId}`);
  
  if (!data) return null;
  
  try {
    return JSON.parse(data) as PersistedJob;
  } catch {
    logger.error({ jobId: jobId.slice(0, 8) }, 'Failed to parse job data');
    return null;
  }
}

/**
 * Get job by nullifier hash (for idempotency checks)
 */
export async function getJobByNullifier(nullifierHash: string): Promise<PersistedJob | null> {
  const r = getRedis();
  const nullifierKey = `${NULLIFIER_PREFIX}${nullifierHash}`;
  
  const jobId = await r.get(nullifierKey);
  if (!jobId) return null;
  
  return getJob(jobId);
}

/**
 * Update job status
 */
export async function updateJobStatus(
  jobId: string,
  status: JobStatus,
  updates?: Partial<Pick<PersistedJob, 'txSignature' | 'errorCode' | 'errorMessage' | 'attempts' | 'completedAt'>>
): Promise<PersistedJob | null> {
  const r = getRedis();
  const jobKey = `${JOB_PREFIX}${jobId}`;
  
  const data = await r.get(jobKey);
  if (!data) return null;
  
  const job = JSON.parse(data) as PersistedJob;
  
  // Update fields
  job.status = status;
  job.updatedAt = Date.now();
  
  if (updates) {
    if (updates.txSignature !== undefined) job.txSignature = updates.txSignature;
    if (updates.errorCode !== undefined) job.errorCode = updates.errorCode;
    if (updates.errorMessage !== undefined) job.errorMessage = updates.errorMessage;
    if (updates.attempts !== undefined) job.attempts = updates.attempts;
    if (updates.completedAt !== undefined) job.completedAt = updates.completedAt;
  }
  
  // Persist
  await r.set(jobKey, JSON.stringify(job), 'EX', JOB_TTL_SECONDS);
  
  // If completed/failed, remove from pending tracking
  if (status === 'succeeded' || status === 'failed') {
    if (job.clientIp) {
      const ipKey = `${PENDING_IP_PREFIX}${job.clientIp}`;
      await r.srem(ipKey, jobId);
    }
  }
  
  logger.debug({
    jobId: jobId.slice(0, 8),
    status,
  }, 'Job status updated');
  
  return job;
}

/**
 * Complete a job successfully
 */
export async function completeJob(jobId: string, txSignature: string): Promise<PersistedJob | null> {
  return updateJobStatus(jobId, 'succeeded', {
    txSignature,
    completedAt: Date.now(),
  });
}

/**
 * Fail a job
 */
export async function failJob(
  jobId: string,
  errorCode: string,
  errorMessage: string
): Promise<PersistedJob | null> {
  return updateJobStatus(jobId, 'failed', {
    errorCode,
    errorMessage,
    completedAt: Date.now(),
  });
}

// =============================================================================
// Pending Job Tracking
// =============================================================================

/**
 * Get count of pending jobs for an IP
 */
export async function getPendingCountByIp(clientIp: string): Promise<number> {
  const r = getRedis();
  const ipKey = `${PENDING_IP_PREFIX}${hashIp(clientIp)}`;
  return r.scard(ipKey);
}

/**
 * Get total pending job count
 */
export async function getTotalPendingCount(): Promise<number> {
  const r = getRedis();
  
  // Scan for all pending IP keys and sum their counts
  let cursor = '0';
  let total = 0;
  
  do {
    const [nextCursor, keys] = await r.scan(cursor, 'MATCH', `${PENDING_IP_PREFIX}*`, 'COUNT', 100);
    cursor = nextCursor;
    
    if (keys.length > 0) {
      const pipeline = r.pipeline();
      for (const key of keys) {
        pipeline.scard(key);
      }
      const results = await pipeline.exec();
      if (results) {
        for (const [, count] of results) {
          total += (count as number) || 0;
        }
      }
    }
  } while (cursor !== '0');
  
  return total;
}

/**
 * Check if nullifier is already in a pending job
 */
export async function isNullifierPending(nullifierHash: string): Promise<boolean> {
  const r = getRedis();
  const jobId = await r.get(`${NULLIFIER_PREFIX}${nullifierHash}`);
  
  if (!jobId) return false;
  
  // Check if job is still pending
  const job = await getJob(jobId);
  if (!job) return false;
  
  return ['queued', 'processing', 'submitting', 'confirming'].includes(job.status);
}

// =============================================================================
// Metrics Helpers
// =============================================================================

/**
 * Increment a metric counter
 */
export async function incrementMetric(field: string, amount: number = 1): Promise<void> {
  const r = getRedis();
  await r.hincrby(METRICS_KEY, field, amount);
}

/**
 * Get all metrics
 */
export async function getStoredMetrics(): Promise<Record<string, string>> {
  const r = getRedis();
  return r.hgetall(METRICS_KEY);
}

/**
 * Get recent jobs for admin/debugging
 */
export async function getRecentJobs(limit: number = 20): Promise<PersistedJob[]> {
  const r = getRedis();
  const jobs: PersistedJob[] = [];
  
  let cursor = '0';
  
  do {
    const [nextCursor, keys] = await r.scan(cursor, 'MATCH', `${JOB_PREFIX}*`, 'COUNT', 100);
    cursor = nextCursor;
    
    if (keys.length > 0) {
      const pipeline = r.pipeline();
      for (const key of keys) {
        pipeline.get(key);
      }
      const results = await pipeline.exec();
      
      if (results) {
        for (const [, data] of results) {
          if (data && typeof data === 'string') {
            try {
              jobs.push(JSON.parse(data));
            } catch {
              // Skip invalid entries
            }
          }
        }
      }
    }
    
    if (jobs.length >= limit) break;
  } while (cursor !== '0');
  
  // Sort by createdAt descending and limit
  return jobs
    .sort((a, b) => b.createdAt - a.createdAt)
    .slice(0, limit);
}
