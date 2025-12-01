/**
 * Job Queue - BullMQ Worker with Persistent Store
 * 
 * Changes from original:
 * 1. Jobs are persisted in Redis (not in-memory Map)
 * 2. Proper status transitions with atomic updates
 * 3. Metrics recording for all state changes
 * 4. Idempotency checks via nullifier tracking
 */

import { Queue, Worker, Job } from 'bullmq';
import { Connection, Keypair, PublicKey } from '@solana/web3.js';
import { v4 as uuidv4 } from 'uuid';
import Redis from 'ioredis';
import { logger, logJobEvent, truncateHash } from './logger.js';
import { validateWithdrawalRequest } from './validation.js';
import { validateFee } from './fees.js';
import { buildWithdrawTransaction, submitTransaction, checkRelayerBalance } from './transaction.js';
import {
  createJob,
  getJob,
  updateJobStatus,
  completeJob,
  failJob,
  isNullifierPending,
  getPendingCountByIp,
  getTotalPendingCount,
} from './jobStore.js';
import {
  recordJobSubmitted,
  recordJobSucceeded,
  recordJobFailed,
  recordJobRejected,
  updatePendingJobs,
} from './metrics.js';
import type {
  RelayerConfig,
  WithdrawalRequestValidated,
  PersistedJob,
  JobStatus,
  RelayerErrorCode,
} from './types.js';

const QUEUE_NAME = 'psol-withdrawals';

// =============================================================================
// Redis Connection
// =============================================================================

function createRedisConnection(config: RelayerConfig): Redis {
  return new Redis({
    host: config.redisHost,
    port: config.redisPort,
    password: config.redisPassword,
    db: config.redisDb,
    maxRetriesPerRequest: null, // Required for BullMQ
    enableReadyCheck: false,
  });
}

// =============================================================================
// Queue Creation
// =============================================================================

/**
 * Initialize job queue
 */
export function createQueue(config: RelayerConfig): Queue {
  const connection = createRedisConnection(config);

  const queue = new Queue(QUEUE_NAME, {
    connection,
    defaultJobOptions: {
      attempts: 3,
      backoff: {
        type: 'exponential',
        delay: 2000,
      },
      removeOnComplete: {
        count: 1000,
      },
      removeOnFail: {
        count: 500,
      },
    },
  });

  queue.on('error', (error) => {
    logger.error({ err: error }, 'Queue error');
  });

  return queue;
}

// =============================================================================
// Worker Creation
// =============================================================================

/**
 * Job data passed to worker
 */
interface WorkerJobData {
  jobId: string;
  request: {
    poolAddress: string;
    tokenMint: string;
    proofData: string;      // Hex encoded
    merkleRoot: string;     // Hex encoded
    nullifierHash: string;  // Hex encoded
    recipient: string;
    amount: string;
    relayerFee: string;
  };
}

/**
 * Create job worker
 */
export function createWorker(
  config: RelayerConfig,
  connection: Connection
): Worker {
  const relayerKeypair = Keypair.fromSecretKey(config.relayerKeypair);
  const redisConnection = createRedisConnection(config);

  const worker = new Worker<WorkerJobData>(
    QUEUE_NAME,
    async (job: Job<WorkerJobData>) => {
      const { jobId, request } = job.data;
      const startTime = Date.now();

      logJobEvent(jobId, 'processing', {
        pool: request.poolAddress.slice(0, 8),
        attempt: job.attemptsMade + 1,
      });

      try {
        // =======================================================================
        // Step 1: Check relayer balance
        // =======================================================================
        await updateJobStatus(jobId, 'processing', { attempts: job.attemptsMade + 1 });

        const balance = await checkRelayerBalance(connection, relayerKeypair.publicKey);
        if (!balance.sufficient) {
          throw new WorkerError('INSUFFICIENT_BALANCE', 'Relayer has insufficient balance');
        }

        // =======================================================================
        // Step 2: Parse and validate request
        // =======================================================================
        const validatedRequest: WithdrawalRequestValidated = {
          poolAddress: new PublicKey(request.poolAddress),
          tokenMint: new PublicKey(request.tokenMint),
          proofData: Buffer.from(request.proofData, 'hex'),
          merkleRoot: Buffer.from(request.merkleRoot, 'hex'),
          nullifierHash: Buffer.from(request.nullifierHash, 'hex'),
          recipient: new PublicKey(request.recipient),
          amount: new (await import('bn.js')).default(request.amount),
          relayerFee: new (await import('bn.js')).default(request.relayerFee),
        };

        const validation = await validateWithdrawalRequest(connection, config, validatedRequest);
        if (!validation.valid) {
          throw new WorkerError('VALIDATION_FAILED', validation.error || 'Validation failed');
        }

        // =======================================================================
        // Step 3: Validate fee
        // =======================================================================
        const feeValidation = await validateFee(
          connection,
          config,
          validatedRequest.amount,
          validatedRequest.relayerFee
        );
        if (!feeValidation.valid) {
          throw new WorkerError('INSUFFICIENT_FEE', feeValidation.error || 'Insufficient fee');
        }

        // =======================================================================
        // Step 4: Build transaction
        // =======================================================================
        await updateJobStatus(jobId, 'submitting');

        const tx = await buildWithdrawTransaction(
          connection,
          config,
          relayerKeypair,
          validatedRequest,
          validatedRequest.tokenMint
        );

        // =======================================================================
        // Step 5: Submit transaction
        // =======================================================================
        await updateJobStatus(jobId, 'confirming');

        const signature = await submitTransaction(connection, tx, [relayerKeypair]);

        // =======================================================================
        // Step 6: Record success
        // =======================================================================
        await completeJob(jobId, signature);

        const processingTime = Date.now() - startTime;
        recordJobSucceeded(
          request.poolAddress,
          request.amount,
          request.relayerFee,
          processingTime
        );

        logJobEvent(jobId, 'succeeded', {
          signature: signature.slice(0, 16),
          processingTimeMs: processingTime,
        });

        // Update pending count
        const pendingCount = await getTotalPendingCount();
        updatePendingJobs(pendingCount);

        return { signature };
      } catch (error) {
        const errorMessage = error instanceof Error ? error.message : 'Unknown error';
        const errorCode = error instanceof WorkerError ? error.code : 'INTERNAL_ERROR';

        await failJob(jobId, errorCode, errorMessage);
        recordJobFailed(request.poolAddress);

        logJobEvent(jobId, 'failed', {
          error: errorMessage,
          code: errorCode,
          attempt: job.attemptsMade + 1,
        });

        // Update pending count
        const pendingCount = await getTotalPendingCount();
        updatePendingJobs(pendingCount);

        throw error;
      }
    },
    {
      connection: redisConnection,
      concurrency: 5,
      limiter: {
        max: 10,
        duration: 1000,
      },
    }
  );

  worker.on('completed', (job) => {
    logger.debug({ jobId: job.data.jobId.slice(0, 8) }, 'Worker job completed');
  });

  worker.on('failed', (job, error) => {
    logger.warn({
      jobId: job?.data.jobId.slice(0, 8),
      error: error.message,
    }, 'Worker job failed');
  });

  worker.on('error', (error) => {
    logger.error({ err: error }, 'Worker error');
  });

  return worker;
}

// =============================================================================
// Job Submission
// =============================================================================

/**
 * Custom error class for worker errors
 */
class WorkerError extends Error {
  constructor(
    public code: string,
    message: string
  ) {
    super(message);
    this.name = 'WorkerError';
  }
}

/**
 * Submit a new withdrawal job
 * 
 * Returns:
 * - job: The created job if successful
 * - error: Error details if submission failed
 */
export async function submitJob(
  queue: Queue,
  config: RelayerConfig,
  request: WithdrawalRequestValidated,
  metadata?: { clientIp?: string; clientId?: string }
): Promise<{ job?: PersistedJob; error?: { code: RelayerErrorCode; message: string } }> {
  const nullifierHex = Buffer.from(request.nullifierHash).toString('hex');

  // =======================================================================
  // Check 1: Is this nullifier already being processed?
  // =======================================================================
  const isPending = await isNullifierPending(nullifierHex);
  if (isPending) {
    recordJobRejected();
    return {
      error: {
        code: 'NULLIFIER_PENDING' as RelayerErrorCode,
        message: 'A withdrawal for this nullifier is already being processed',
      },
    };
  }

  // =======================================================================
  // Check 2: Per-IP pending limit
  // =======================================================================
  if (metadata?.clientIp) {
    const ipPendingCount = await getPendingCountByIp(metadata.clientIp);
    if (ipPendingCount >= config.maxPendingPerIp) {
      recordJobRejected();
      return {
        error: {
          code: 'TOO_MANY_PENDING' as RelayerErrorCode,
          message: `Too many pending withdrawals (${ipPendingCount}/${config.maxPendingPerIp})`,
        },
      };
    }
  }

  // =======================================================================
  // Check 3: Global pending limit
  // =======================================================================
  const totalPending = await getTotalPendingCount();
  if (totalPending >= config.maxPendingGlobal) {
    recordJobRejected();
    return {
      error: {
        code: 'QUEUE_FULL' as RelayerErrorCode,
        message: 'Queue is full, please try again later',
      },
    };
  }

  // =======================================================================
  // Create job in persistent store
  // =======================================================================
  const jobId = uuidv4();

  const persistedJob: PersistedJob = {
    id: jobId,
    status: 'queued',
    poolAddress: request.poolAddress.toString(),
    tokenMint: request.tokenMint.toString(),
    nullifierHash: nullifierHex,
    recipient: request.recipient.toString(),
    amount: request.amount.toString(),
    relayerFee: request.relayerFee.toString(),
    createdAt: Date.now(),
    updatedAt: Date.now(),
    attempts: 0,
    maxAttempts: 3,
    clientIp: metadata?.clientIp,
    clientId: metadata?.clientId,
  };

  const createdJob = await createJob(persistedJob);
  if (!createdJob) {
    // This means nullifier was already in store (race condition)
    recordJobRejected();
    return {
      error: {
        code: 'NULLIFIER_PENDING' as RelayerErrorCode,
        message: 'A withdrawal for this nullifier already exists',
      },
    };
  }

  // =======================================================================
  // Add to BullMQ queue
  // =======================================================================
  const jobData: WorkerJobData = {
    jobId,
    request: {
      poolAddress: request.poolAddress.toString(),
      tokenMint: request.tokenMint.toString(),
      proofData: Buffer.from(request.proofData).toString('hex'),
      merkleRoot: Buffer.from(request.merkleRoot).toString('hex'),
      nullifierHash: nullifierHex,
      recipient: request.recipient.toString(),
      amount: request.amount.toString(),
      relayerFee: request.relayerFee.toString(),
    },
  };

  await queue.add(jobId, jobData, { jobId });

  recordJobSubmitted(request.poolAddress.toString());

  logJobEvent(jobId, 'created', {
    pool: request.poolAddress.toString().slice(0, 8),
    recipient: request.recipient.toString().slice(0, 8),
    amount: request.amount.toString(),
    nullifier: truncateHash(nullifierHex),
  });

  // Update pending count
  const pendingCount = await getTotalPendingCount();
  updatePendingJobs(pendingCount);

  return { job: createdJob };
}

// =============================================================================
// Job Queries (re-exported from jobStore)
// =============================================================================

export { getJob, getPendingCountByIp, getTotalPendingCount };
