/**
 * Fee Calculation
 * Dynamic fee calculation based on network conditions
 */

import { Connection } from '@solana/web3.js';
import BN from 'bn.js';
import { logger } from './logger.js';
import type { RelayerConfig, FeeQuoteResponseData } from './types.js';

// Cache for recent priority fee data
interface FeeCache {
  baseFee: number;
  priorityFee: number;
  timestamp: number;
}

let feeCache: FeeCache | null = null;
const FEE_CACHE_TTL = 30_000; // 30 seconds

/**
 * Get current priority fees from network
 */
async function getNetworkFees(connection: Connection): Promise<{
  baseFee: number;
  priorityFee: number;
}> {
  // Check cache
  if (feeCache && Date.now() - feeCache.timestamp < FEE_CACHE_TTL) {
    return { baseFee: feeCache.baseFee, priorityFee: feeCache.priorityFee };
  }

  try {
    // Get recent prioritization fees
    const recentFees = await connection.getRecentPrioritizationFees();
    
    if (recentFees.length === 0) {
      return { baseFee: 5000, priorityFee: 0 }; // Default base fee
    }

    // Calculate median priority fee
    const priorityFees = recentFees
      .map(f => f.prioritizationFee)
      .sort((a, b) => a - b);
    
    const medianIndex = Math.floor(priorityFees.length / 2);
    const medianPriorityFee = priorityFees[medianIndex];

    // Base fee is constant on Solana (5000 lamports)
    const baseFee = 5000;

    // Update cache
    feeCache = {
      baseFee,
      priorityFee: medianPriorityFee,
      timestamp: Date.now(),
    };

    return { baseFee, priorityFee: medianPriorityFee };
  } catch (error) {
    logger.error({ error }, 'Failed to fetch network fees');
    return { baseFee: 5000, priorityFee: 1000 }; // Safe defaults
  }
}

/**
 * Calculate dynamic fee multiplier based on network congestion
 */
async function getDynamicMultiplier(connection: Connection): Promise<number> {
  try {
    // Get recent performance samples
    const samples = await connection.getRecentPerformanceSamples(5);
    
    if (samples.length === 0) {
      return 1.0;
    }

    // Calculate average TPS
    const avgTps = samples.reduce((sum, s) => sum + s.numTransactions / s.samplePeriodSecs, 0) / samples.length;

    // Solana can handle ~65k TPS theoretically, but practical limit is lower
    // If TPS > 2000, network is congested
    if (avgTps > 3000) {
      return 1.5; // High congestion
    } else if (avgTps > 2000) {
      return 1.25; // Medium congestion
    } else {
      return 1.0; // Normal
    }
  } catch (error) {
    logger.debug({ error }, 'Failed to get performance samples');
    return 1.0;
  }
}

/**
 * Calculate relayer fee for a withdrawal
 */
export async function calculateFee(
  connection: Connection,
  config: RelayerConfig,
  amount: BN
): Promise<{
  baseFee: BN;
  dynamicFee: BN;
  totalFee: BN;
  feeBps: number;
}> {
  // Get network fees
  const networkFees = await getNetworkFees(connection);
  
  // Calculate base relayer fee (percentage of amount)
  let feeBps = config.baseFeeBps;
  
  // Apply dynamic multiplier if enabled
  if (config.enableDynamicFees) {
    const multiplier = await getDynamicMultiplier(connection);
    feeBps = Math.floor(feeBps * multiplier);
  }

  // Cap fee at maximum
  feeBps = Math.min(feeBps, config.maxFeeBps);

  // Calculate fee amount
  let baseFee = amount.muln(feeBps).divn(10_000);
  
  // Ensure minimum fee
  const minFee = new BN(config.minFeeLamports);
  if (baseFee.lt(minFee)) {
    baseFee = minFee;
  }

  // Dynamic fee covers gas costs
  const gasCost = networkFees.baseFee + networkFees.priorityFee * 200_000; // Estimate CU
  const dynamicFee = new BN(Math.ceil(gasCost * 1.2)); // 20% buffer

  // Total fee
  const totalFee = baseFee.add(dynamicFee);

  return {
    baseFee,
    dynamicFee,
    totalFee,
    feeBps,
  };
}

/**
 * Generate fee quote response
 */
export async function getFeeQuote(
  connection: Connection,
  config: RelayerConfig,
  amount: BN
): Promise<FeeQuoteResponseData> {
  const fees = await calculateFee(connection, config, amount);

  return {
    baseFee: fees.baseFee.toString(),
    dynamicFee: fees.dynamicFee.toString(),
    totalFee: fees.totalFee.toString(),
    feeBps: fees.feeBps,
    validUntil: Date.now() + 60_000, // Valid for 1 minute
  };
}

/**
 * Validate that provided fee meets minimum requirements
 */
export async function validateFee(
  connection: Connection,
  config: RelayerConfig,
  amount: BN,
  providedFee: BN
): Promise<{ valid: boolean; minimumFee?: BN; error?: string }> {
  const required = await calculateFee(connection, config, amount);

  // Allow 5% tolerance for timing differences
  const minRequired = required.totalFee.muln(95).divn(100);

  if (providedFee.lt(minRequired)) {
    return {
      valid: false,
      minimumFee: required.totalFee,
      error: `Insufficient fee: provided ${providedFee.toString()}, minimum ${required.totalFee.toString()}`,
    };
  }

  return { valid: true };
}

/**
 * Calculate expected earnings from a withdrawal
 */
export function calculateRelayerProfit(
  relayerFee: BN,
  estimatedGasCost: BN
): {
  grossRevenue: BN;
  gasCost: BN;
  netProfit: BN;
  profitMargin: number;
} {
  const netProfit = relayerFee.sub(estimatedGasCost);
  const profitMargin = relayerFee.gtn(0)
    ? netProfit.muln(100).div(relayerFee).toNumber()
    : 0;

  return {
    grossRevenue: relayerFee,
    gasCost: estimatedGasCost,
    netProfit,
    profitMargin,
  };
}
