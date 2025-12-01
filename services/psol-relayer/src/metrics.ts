/**
 * Metrics Collection - Prometheus Compatible
 * 
 * Provides:
 * 1. Counter metrics (requests, jobs, errors)
 * 2. Gauge metrics (pending jobs, balance)
 * 3. Histogram approximations (processing time)
 * 4. Per-pool statistics
 * 5. Prometheus-format output
 */

import { logger } from './logger.js';
import type { RelayerMetrics } from './types.js';

// =============================================================================
// In-Memory Metrics Storage
// =============================================================================

const metrics: RelayerMetrics = {
  // Counters
  requestsTotal: 0,
  requestsByEndpoint: {},
  jobsSubmitted: 0,
  jobsSucceeded: 0,
  jobsFailed: 0,
  jobsRejected: 0,

  // Per-pool stats
  jobsByPool: {},

  // Gauges (updated externally)
  pendingJobs: 0,
  relayerBalanceLamports: 0,

  // Processing times (keep last 100)
  processingTimesMs: [],

  // Derived (calculated on read)
  avgProcessingTimeMs: 0,
  successRate: 100,

  // Timestamps
  startedAt: Date.now(),
  lastUpdatedAt: Date.now(),
};

// Max samples to keep for histograms
const MAX_PROCESSING_SAMPLES = 100;

// =============================================================================
// Metric Recording Functions
// =============================================================================

/**
 * Record an API request
 */
export function recordRequest(endpoint: string): void {
  metrics.requestsTotal++;
  metrics.requestsByEndpoint[endpoint] = (metrics.requestsByEndpoint[endpoint] || 0) + 1;
  metrics.lastUpdatedAt = Date.now();
}

/**
 * Record job submitted
 */
export function recordJobSubmitted(poolAddress?: string): void {
  metrics.jobsSubmitted++;
  
  if (poolAddress) {
    if (!metrics.jobsByPool[poolAddress]) {
      metrics.jobsByPool[poolAddress] = {
        submitted: 0,
        succeeded: 0,
        failed: 0,
        volumeTotal: '0',
        feesTotal: '0',
      };
    }
    metrics.jobsByPool[poolAddress].submitted++;
  }
  
  metrics.lastUpdatedAt = Date.now();
}

/**
 * Record job succeeded
 */
export function recordJobSucceeded(
  poolAddress?: string,
  amount?: string,
  fee?: string,
  processingTimeMs?: number
): void {
  metrics.jobsSucceeded++;
  
  if (poolAddress && metrics.jobsByPool[poolAddress]) {
    metrics.jobsByPool[poolAddress].succeeded++;
    
    if (amount) {
      const current = BigInt(metrics.jobsByPool[poolAddress].volumeTotal);
      metrics.jobsByPool[poolAddress].volumeTotal = (current + BigInt(amount)).toString();
    }
    if (fee) {
      const current = BigInt(metrics.jobsByPool[poolAddress].feesTotal);
      metrics.jobsByPool[poolAddress].feesTotal = (current + BigInt(fee)).toString();
    }
  }
  
  if (processingTimeMs !== undefined) {
    metrics.processingTimesMs.push(processingTimeMs);
    if (metrics.processingTimesMs.length > MAX_PROCESSING_SAMPLES) {
      metrics.processingTimesMs.shift();
    }
  }
  
  updateDerivedMetrics();
  metrics.lastUpdatedAt = Date.now();
}

/**
 * Record job failed
 */
export function recordJobFailed(poolAddress?: string): void {
  metrics.jobsFailed++;
  
  if (poolAddress && metrics.jobsByPool[poolAddress]) {
    metrics.jobsByPool[poolAddress].failed++;
  }
  
  updateDerivedMetrics();
  metrics.lastUpdatedAt = Date.now();
}

/**
 * Record job rejected (validation failed, rate limited, etc.)
 */
export function recordJobRejected(): void {
  metrics.jobsRejected++;
  metrics.lastUpdatedAt = Date.now();
}

/**
 * Update pending jobs gauge
 */
export function updatePendingJobs(count: number): void {
  metrics.pendingJobs = count;
  metrics.lastUpdatedAt = Date.now();
}

/**
 * Update relayer balance gauge
 */
export function updateRelayerBalance(lamports: number): void {
  metrics.relayerBalanceLamports = lamports;
  metrics.lastUpdatedAt = Date.now();
}

/**
 * Update derived metrics
 */
function updateDerivedMetrics(): void {
  // Calculate average processing time
  if (metrics.processingTimesMs.length > 0) {
    const sum = metrics.processingTimesMs.reduce((a, b) => a + b, 0);
    metrics.avgProcessingTimeMs = Math.round(sum / metrics.processingTimesMs.length);
  }
  
  // Calculate success rate
  const totalCompleted = metrics.jobsSucceeded + metrics.jobsFailed;
  if (totalCompleted > 0) {
    metrics.successRate = Math.round((metrics.jobsSucceeded / totalCompleted) * 100 * 100) / 100;
  }
}

// =============================================================================
// Metric Retrieval
// =============================================================================

/**
 * Get all metrics as JSON
 */
export function getMetrics(): RelayerMetrics {
  updateDerivedMetrics();
  return { ...metrics };
}

/**
 * Get metrics in Prometheus format
 */
export function getPrometheusMetrics(): string {
  updateDerivedMetrics();
  
  const lines: string[] = [];
  const prefix = 'psol_relayer_';
  
  // Helper to add metric
  const addMetric = (name: string, type: string, help: string, value: number | string, labels?: Record<string, string>) => {
    lines.push(`# HELP ${prefix}${name} ${help}`);
    lines.push(`# TYPE ${prefix}${name} ${type}`);
    
    let labelStr = '';
    if (labels && Object.keys(labels).length > 0) {
      const labelParts = Object.entries(labels).map(([k, v]) => `${k}="${v}"`);
      labelStr = `{${labelParts.join(',')}}`;
    }
    
    lines.push(`${prefix}${name}${labelStr} ${value}`);
  };
  
  // Counters
  addMetric('requests_total', 'counter', 'Total API requests', metrics.requestsTotal);
  addMetric('jobs_submitted_total', 'counter', 'Total jobs submitted', metrics.jobsSubmitted);
  addMetric('jobs_succeeded_total', 'counter', 'Total jobs succeeded', metrics.jobsSucceeded);
  addMetric('jobs_failed_total', 'counter', 'Total jobs failed', metrics.jobsFailed);
  addMetric('jobs_rejected_total', 'counter', 'Total jobs rejected', metrics.jobsRejected);
  
  // Per-endpoint request counts
  for (const [endpoint, count] of Object.entries(metrics.requestsByEndpoint)) {
    addMetric('requests_by_endpoint', 'counter', 'Requests by endpoint', count, { endpoint });
  }
  
  // Per-pool stats
  for (const [pool, stats] of Object.entries(metrics.jobsByPool)) {
    const poolShort = pool.slice(0, 8); // Truncate for label
    addMetric('pool_jobs_submitted', 'counter', 'Jobs submitted per pool', stats.submitted, { pool: poolShort });
    addMetric('pool_jobs_succeeded', 'counter', 'Jobs succeeded per pool', stats.succeeded, { pool: poolShort });
    addMetric('pool_jobs_failed', 'counter', 'Jobs failed per pool', stats.failed, { pool: poolShort });
  }
  
  // Gauges
  addMetric('pending_jobs', 'gauge', 'Current pending jobs', metrics.pendingJobs);
  addMetric('relayer_balance_lamports', 'gauge', 'Relayer SOL balance in lamports', metrics.relayerBalanceLamports);
  addMetric('relayer_balance_sol', 'gauge', 'Relayer SOL balance', metrics.relayerBalanceLamports / 1e9);
  
  // Histograms (simplified as gauges)
  addMetric('processing_time_avg_ms', 'gauge', 'Average job processing time in ms', metrics.avgProcessingTimeMs);
  addMetric('success_rate_percent', 'gauge', 'Job success rate percentage', metrics.successRate);
  
  // Processing time percentiles (if we have enough samples)
  if (metrics.processingTimesMs.length >= 10) {
    const sorted = [...metrics.processingTimesMs].sort((a, b) => a - b);
    const p50 = sorted[Math.floor(sorted.length * 0.5)];
    const p95 = sorted[Math.floor(sorted.length * 0.95)];
    const p99 = sorted[Math.floor(sorted.length * 0.99)];
    
    addMetric('processing_time_p50_ms', 'gauge', 'Job processing time 50th percentile', p50);
    addMetric('processing_time_p95_ms', 'gauge', 'Job processing time 95th percentile', p95);
    addMetric('processing_time_p99_ms', 'gauge', 'Job processing time 99th percentile', p99);
  }
  
  // Meta
  addMetric('uptime_seconds', 'gauge', 'Relayer uptime in seconds', Math.floor((Date.now() - metrics.startedAt) / 1000));
  addMetric('info', 'gauge', 'Relayer info', 1, { version: '1.0.0' });
  
  return lines.join('\n') + '\n';
}

/**
 * Reset all metrics (for testing)
 */
export function resetMetrics(): void {
  metrics.requestsTotal = 0;
  metrics.requestsByEndpoint = {};
  metrics.jobsSubmitted = 0;
  metrics.jobsSucceeded = 0;
  metrics.jobsFailed = 0;
  metrics.jobsRejected = 0;
  metrics.jobsByPool = {};
  metrics.pendingJobs = 0;
  metrics.relayerBalanceLamports = 0;
  metrics.processingTimesMs = [];
  metrics.avgProcessingTimeMs = 0;
  metrics.successRate = 100;
  metrics.startedAt = Date.now();
  metrics.lastUpdatedAt = Date.now();
}
