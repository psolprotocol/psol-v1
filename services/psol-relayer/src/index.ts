/**
 * pSol Relayer - Main Entry Point
 * 
 * Startup sequence:
 * 1. Load and validate configuration
 * 2. Connect to Solana RPC
 * 3. Initialize Redis and job store
 * 4. Initialize rate limiter
 * 5. Load verification key (optional)
 * 6. Create queue and worker
 * 7. Start API server
 * 8. Setup graceful shutdown handlers
 */

import { Connection, Keypair, LAMPORTS_PER_SOL } from '@solana/web3.js';
import Redis from 'ioredis';
import { loadConfig, getRelayerPublicKey, isProduction } from './config.js';
import { logger, logStartup, logError } from './logger.js';
import { initJobStore, closeJobStore, isJobStoreConnected } from './jobStore.js';
import { initRateLimiter } from './rateLimit.js';
import { loadVerificationKey } from './validation.js';
import { createQueue, createWorker } from './queue.js';
import { createServer } from './server.js';
import { updateRelayerBalance, updatePendingJobs } from './metrics.js';
import { getTotalPendingCount } from './queue.js';

// =============================================================================
// Main Entry Point
// =============================================================================

async function main(): Promise<void> {
  logger.info('========================================');
  logger.info('pSol Relayer Starting...');
  logger.info('========================================');

  // =========================================================================
  // Step 1: Load Configuration
  // =========================================================================
  let config;
  try {
    config = loadConfig();
    logger.info({
      network: config.network,
      port: config.port,
      authRequired: config.requireAuthForWrite,
      apiKeysConfigured: config.apiKeys.length,
    }, 'Configuration loaded');
  } catch (error) {
    logger.fatal({ err: error }, 'Failed to load configuration');
    process.exit(1);
  }

  // =========================================================================
  // Step 2: Connect to Solana
  // =========================================================================
  const connection = new Connection(config.rpcUrl, {
    commitment: config.commitment,
    wsEndpoint: config.wsUrl,
  });

  try {
    const version = await connection.getVersion();
    logger.info({ version: version['solana-core'] }, 'Connected to Solana');
  } catch (error) {
    logger.fatal({ err: error }, 'Failed to connect to Solana RPC');
    process.exit(1);
  }

  // =========================================================================
  // Step 3: Check Relayer Balance
  // =========================================================================
  const relayerPublicKey = getRelayerPublicKey(config);
  
  try {
    const balance = await connection.getBalance(relayerPublicKey);
    const balanceSol = balance / LAMPORTS_PER_SOL;
    updateRelayerBalance(balance);

    if (balanceSol < config.minBalance) {
      logger.error({
        balance: balanceSol,
        minimum: config.minBalance,
        address: relayerPublicKey.toString(),
      }, 'Relayer balance below minimum');
      
      if (isProduction()) {
        logger.fatal('Cannot start in production with insufficient balance');
        process.exit(1);
      }
    }

    logger.info({
      address: relayerPublicKey.toString(),
      balance: balanceSol.toFixed(4),
    }, 'Relayer wallet ready');
  } catch (error) {
    logger.fatal({ err: error }, 'Failed to check relayer balance');
    process.exit(1);
  }

  // =========================================================================
  // Step 4: Initialize Redis and Job Store
  // =========================================================================
  let redis: Redis;
  try {
    redis = new Redis({
      host: config.redisHost,
      port: config.redisPort,
      password: config.redisPassword,
      db: config.redisDb,
      lazyConnect: true,
      maxRetriesPerRequest: null,
    });

    await redis.connect();
    await redis.ping();
    
    // Initialize job store
    await initJobStore(config);
    
    // Initialize rate limiter with same connection
    initRateLimiter(redis);

    logger.info({
      host: config.redisHost,
      port: config.redisPort,
    }, 'Redis connected');
  } catch (error) {
    logger.fatal({ err: error }, 'Failed to connect to Redis');
    process.exit(1);
  }

  // =========================================================================
  // Step 5: Load Verification Key (Optional)
  // =========================================================================
  try {
    await loadVerificationKey(config.verificationKeyPath);
  } catch (error) {
    logger.warn({ err: error }, 'Verification key not loaded - local proof validation disabled');
  }

  // =========================================================================
  // Step 6: Create Queue and Worker
  // =========================================================================
  const queue = createQueue(config);
  const worker = createWorker(config, connection);

  logger.info('Queue and worker initialized');

  // =========================================================================
  // Step 7: Start API Server
  // =========================================================================
  const app = createServer(config, connection, queue);

  const server = app.listen(config.port, config.host, () => {
    logStartup({
      network: config.network,
      rpcUrl: config.rpcUrl,
      relayerAddress: relayerPublicKey.toString(),
      port: config.port,
    });

    logger.info('========================================');
    logger.info(`pSol Relayer running on ${config.host}:${config.port}`);
    logger.info('========================================');
  });

  // =========================================================================
  // Step 8: Background Tasks
  // =========================================================================

  // Balance check interval
  const balanceCheckInterval = setInterval(async () => {
    try {
      const balance = await connection.getBalance(relayerPublicKey);
      const balanceSol = balance / LAMPORTS_PER_SOL;
      updateRelayerBalance(balance);

      if (balanceSol < config.minBalance) {
        logger.error({
          balance: balanceSol,
          minimum: config.minBalance,
        }, 'ALERT: Relayer balance low');
      } else if (balanceSol < config.minBalance * 2) {
        logger.warn({
          balance: balanceSol,
          minimum: config.minBalance,
        }, 'Warning: Relayer balance getting low');
      }
    } catch (error) {
      logger.error({ err: error }, 'Balance check failed');
    }
  }, 5 * 60 * 1000); // Every 5 minutes

  // Pending jobs update interval
  const pendingUpdateInterval = setInterval(async () => {
    try {
      const pending = await getTotalPendingCount();
      updatePendingJobs(pending);
    } catch (error) {
      logger.debug({ err: error }, 'Pending jobs update failed');
    }
  }, 30 * 1000); // Every 30 seconds

  // =========================================================================
  // Step 9: Graceful Shutdown
  // =========================================================================
  let isShuttingDown = false;

  async function shutdown(signal: string): Promise<void> {
    if (isShuttingDown) return;
    isShuttingDown = true;

    logger.info({ signal }, 'Shutdown initiated');

    // Stop accepting new requests
    server.close(() => {
      logger.info('HTTP server closed');
    });

    // Clear intervals
    clearInterval(balanceCheckInterval);
    clearInterval(pendingUpdateInterval);

    try {
      // Wait for worker to finish current jobs
      logger.info('Waiting for worker to finish...');
      await worker.close();
      logger.info('Worker closed');

      // Close queue
      await queue.close();
      logger.info('Queue closed');

      // Close Redis connections
      await closeJobStore();
      await redis.quit();
      logger.info('Redis connections closed');

      logger.info('Shutdown complete');
      process.exit(0);
    } catch (error) {
      logger.error({ err: error }, 'Error during shutdown');
      process.exit(1);
    }
  }

  // Signal handlers
  process.on('SIGTERM', () => shutdown('SIGTERM'));
  process.on('SIGINT', () => shutdown('SIGINT'));

  // Unhandled error handlers
  process.on('uncaughtException', (error) => {
    logger.fatal({ err: error }, 'Uncaught exception');
    shutdown('uncaughtException').catch(() => process.exit(1));
  });

  process.on('unhandledRejection', (reason) => {
    logger.error({ reason }, 'Unhandled rejection');
  });
}

// Run
main().catch((error) => {
  console.error('Fatal error:', error);
  process.exit(1);
});
