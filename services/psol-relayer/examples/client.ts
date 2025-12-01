/**
 * Example Relayer Client
 * 
 * Demonstrates how to interact with the pSol relayer API:
 * 1. Get fee quote
 * 2. Submit withdrawal
 * 3. Poll for completion
 */

// Configuration
const RELAYER_URL = process.env.RELAYER_URL || 'http://localhost:3000';
const API_KEY = process.env.RELAYER_API_KEY;

interface FeeQuoteResponse {
  success: boolean;
  data?: {
    baseFee: string;
    dynamicFee: string;
    totalFee: string;
    feeBps: number;
    validUntil: number;
  };
  error?: {
    code: string;
    message: string;
  };
}

interface SubmitResponse {
  success: boolean;
  data?: {
    jobId: string;
    status: string;
    estimatedTime: number;
  };
  error?: {
    code: string;
    message: string;
    details?: unknown;
  };
}

interface JobStatusResponse {
  success: boolean;
  data?: {
    jobId: string;
    status: string;
    txSignature?: string;
    errorCode?: string;
    errorMessage?: string;
    createdAt: number;
    updatedAt: number;
  };
  error?: {
    code: string;
    message: string;
  };
}

/**
 * Get headers for API requests
 */
function getHeaders(): Record<string, string> {
  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
  };
  
  if (API_KEY) {
    headers['Authorization'] = `Bearer ${API_KEY}`;
  }
  
  return headers;
}

/**
 * Check relayer health
 */
async function checkHealth(): Promise<void> {
  console.log('\n=== Checking Relayer Health ===');
  
  const response = await fetch(`${RELAYER_URL}/health`);
  const data = await response.json();
  
  console.log('Status:', response.status);
  console.log('Data:', JSON.stringify(data, null, 2));
}

/**
 * Get relayer info
 */
async function getRelayerInfo(): Promise<void> {
  console.log('\n=== Getting Relayer Info ===');
  
  const response = await fetch(`${RELAYER_URL}/info`);
  const data = await response.json();
  
  console.log('Status:', response.status);
  console.log('Data:', JSON.stringify(data, null, 2));
}

/**
 * Get fee quote for a withdrawal
 */
async function getFeeQuote(poolAddress: string, amount: string): Promise<FeeQuoteResponse> {
  console.log('\n=== Getting Fee Quote ===');
  console.log('Pool:', poolAddress);
  console.log('Amount:', amount);
  
  const response = await fetch(`${RELAYER_URL}/fee/quote`, {
    method: 'POST',
    headers: getHeaders(),
    body: JSON.stringify({ poolAddress, amount }),
  });
  
  const data: FeeQuoteResponse = await response.json();
  
  console.log('Status:', response.status);
  console.log('Response:', JSON.stringify(data, null, 2));
  
  return data;
}

/**
 * Check if nullifier is spent
 */
async function checkNullifier(poolAddress: string, nullifierHash: string): Promise<boolean> {
  console.log('\n=== Checking Nullifier ===');
  console.log('Nullifier:', nullifierHash.slice(0, 16) + '...');
  
  const response = await fetch(`${RELAYER_URL}/validate/nullifier`, {
    method: 'POST',
    headers: getHeaders(),
    body: JSON.stringify({ poolAddress, nullifierHash }),
  });
  
  const data = await response.json();
  
  console.log('Status:', response.status);
  console.log('Spent:', data.data?.spent);
  
  return data.data?.spent ?? false;
}

/**
 * Submit withdrawal request
 */
async function submitWithdrawal(request: {
  poolAddress: string;
  tokenMint: string;
  proofData: string;
  merkleRoot: string;
  nullifierHash: string;
  recipient: string;
  amount: string;
  relayerFee: string;
}): Promise<SubmitResponse> {
  console.log('\n=== Submitting Withdrawal ===');
  console.log('Pool:', request.poolAddress);
  console.log('Recipient:', request.recipient);
  console.log('Amount:', request.amount);
  console.log('Fee:', request.relayerFee);
  
  const response = await fetch(`${RELAYER_URL}/withdraw`, {
    method: 'POST',
    headers: getHeaders(),
    body: JSON.stringify(request),
  });
  
  const data: SubmitResponse = await response.json();
  
  console.log('Status:', response.status);
  console.log('Response:', JSON.stringify(data, null, 2));
  
  return data;
}

/**
 * Get job status
 */
async function getJobStatus(jobId: string): Promise<JobStatusResponse> {
  const response = await fetch(`${RELAYER_URL}/withdraw/${jobId}`, {
    headers: getHeaders(),
  });
  
  return response.json();
}

/**
 * Poll for job completion
 */
async function waitForCompletion(jobId: string, maxWaitMs: number = 120_000): Promise<JobStatusResponse> {
  console.log('\n=== Waiting for Job Completion ===');
  console.log('Job ID:', jobId);
  
  const startTime = Date.now();
  const pollIntervalMs = 2000;
  
  while (Date.now() - startTime < maxWaitMs) {
    const status = await getJobStatus(jobId);
    
    console.log(`Status: ${status.data?.status} (${Math.round((Date.now() - startTime) / 1000)}s)`);
    
    if (status.data?.status === 'succeeded') {
      console.log('\n✅ Withdrawal succeeded!');
      console.log('Transaction:', status.data.txSignature);
      return status;
    }
    
    if (status.data?.status === 'failed') {
      console.log('\n❌ Withdrawal failed!');
      console.log('Error:', status.data.errorCode, '-', status.data.errorMessage);
      return status;
    }
    
    await new Promise(resolve => setTimeout(resolve, pollIntervalMs));
  }
  
  throw new Error(`Job ${jobId} did not complete within ${maxWaitMs}ms`);
}

/**
 * Full withdrawal flow example
 */
async function exampleWithdrawalFlow(): Promise<void> {
  console.log('========================================');
  console.log('pSol Relayer Client Example');
  console.log('========================================');
  console.log('Relayer URL:', RELAYER_URL);
  console.log('API Key:', API_KEY ? 'Configured' : 'Not set');
  
  // Step 1: Check health
  await checkHealth();
  
  // Step 2: Get relayer info
  await getRelayerInfo();
  
  // Example withdrawal parameters (replace with real values)
  const poolAddress = 'POOL_ADDRESS_HERE';
  const tokenMint = 'TOKEN_MINT_HERE';
  const amount = '1000000000'; // 1 token with 9 decimals
  
  // Step 3: Get fee quote
  const feeQuote = await getFeeQuote(poolAddress, amount);
  
  if (!feeQuote.success || !feeQuote.data) {
    console.error('Failed to get fee quote:', feeQuote.error);
    return;
  }
  
  console.log('\nFee breakdown:');
  console.log('  Base fee:', feeQuote.data.baseFee);
  console.log('  Dynamic fee:', feeQuote.data.dynamicFee);
  console.log('  Total fee:', feeQuote.data.totalFee);
  
  // Step 4: Submit withdrawal (with actual proof data from SDK)
  // In a real scenario, you would:
  // 1. Generate the ZK proof using the pSol SDK
  // 2. Get the merkle root, nullifier hash from the proof
  // 3. Submit with the relayer fee from the quote
  
  /*
  const withdrawal = await submitWithdrawal({
    poolAddress,
    tokenMint,
    proofData: 'PROOF_DATA_HEX',
    merkleRoot: 'MERKLE_ROOT_HEX',
    nullifierHash: 'NULLIFIER_HASH_HEX',
    recipient: 'RECIPIENT_ADDRESS',
    amount,
    relayerFee: feeQuote.data.totalFee,
  });
  
  if (withdrawal.success && withdrawal.data) {
    await waitForCompletion(withdrawal.data.jobId);
  }
  */
  
  console.log('\n========================================');
  console.log('Example complete!');
  console.log('========================================');
}

// Run example
exampleWithdrawalFlow().catch(console.error);
