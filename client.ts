import * as anchor from "@coral-xyz/anchor";
import axios, { AxiosInstance } from 'axios';
import { RelayerConfig, RelayerRequest, RelayerResponse } from './types';
import type { Crypto } from "../target/types/crypto";

// Configure the client to use the local cluster
anchor.setProvider(anchor.AnchorProvider.env());

const program = anchor.workspace.Crypto as anchor.Program<Crypto>;


export class RelayerClient {
  private client: AxiosInstance;
  private config: RelayerConfig;

  constructor(config: RelayerConfig) {
    this.config = config;
    this.client = axios.create({
      baseURL: config.endpoint,
      timeout: config.timeout || 30000, // 30 second default timeout
      headers: {
        'Content-Type': 'application/json',
        ...(config.apiKey && { 'X-API-Key': config.apiKey }),
      },
    });
  }

  async submitTransaction(request: RelayerRequest): Promise<RelayerResponse> {
    try {
      const response = await this.client.post('/api/v1/submit', request);
      return response.data;
    } catch (error: any) {
      console.error('Relayer submission error:', error);
      return {
        success: false,
        error: error.response?.data?.error || error.message || 'Unknown error',
      };
    }
  }

  async getTransactionStatus(transactionId: string): Promise<RelayerResponse> {
    try {
      const response = await this.client.get(`/api/v1/status/${transactionId}`);
      return response.data;
    } catch (error: any) {
      console.error('Relayer status check error:', error);
      return {
        success: false,
        error: error.response?.data?.error || error.message || 'Unknown error',
      };
    }
  }

  async getRelayerInfo(): Promise<any> {
    try {
      const response = await this.client.get('/api/v1/info');
      return response.data;
    } catch (error: any) {
      console.error('Relayer info error:', error);
      throw new Error(`Failed to get relayer info: ${error.message}`);
    }
  }

  async getFeeEstimate(request: Partial<RelayerRequest>): Promise<number> {
    try {
      const response = await this.client.post('/api/v1/fee', request);
      return response.data.estimatedFee;
    } catch (error: any) {
      console.error('Fee estimation error:', error);
      // Return default fee estimate
      return 0.001; // 0.001 SOL default
    }
  }

  async getHealthStatus(): Promise<{ healthy: boolean; timestamp: number }> {
    try {
      const response = await this.client.get('/health');
      return {
        healthy: response.status === 200,
        timestamp: Date.now(),
      };
    } catch (error: any) {
      console.error('Health check error:', error);
      return {
        healthy: false,
        timestamp: Date.now(),
      };
    }
  }

  // Batch transaction submission for improved privacy
  async submitBatch(requests: RelayerRequest[]): Promise<RelayerResponse[]> {
    try {
      const response = await this.client.post('/api/v1/batch', { requests });
      return response.data.results;
    } catch (error: any) {
      console.error('Batch submission error:', error);
      return requests.map(() => ({
        success: false,
        error: error.response?.data?.error || error.message || 'Unknown error',
      }));
    }
  }

  // Cancel pending transaction
  async cancelTransaction(transactionId: string): Promise<RelayerResponse> {
    try {
      const response = await this.client.delete(`/api/v1/cancel/${transactionId}`);
      return response.data;
    } catch (error: any) {
      console.error('Transaction cancellation error:', error);
      return {
        success: false,
        error: error.response?.data?.error || error.message || 'Unknown error',
      };
    }
  }

  // Get transaction history
  async getTransactionHistory(limit: number = 10, offset: number = 0): Promise<any[]> {
    try {
      const response = await this.client.get(`/api/v1/history?limit=${limit}&offset=${offset}`);
      return response.data.transactions;
    } catch (error: any) {
      console.error('Transaction history error:', error);
      return [];
    }
  }

  // WebSocket support for real-time updates
  subscribeToEvents(eventTypes: string[], callback: (event: any) => void): () => void {
    // In a real implementation, this would establish a WebSocket connection
    // For now, we'll simulate with polling
    const interval = setInterval(async () => {
      try {
        const response = await this.client.get('/api/v1/events');
        response.data.events.forEach(callback);
      } catch (error) {
        console.error('Event polling error:', error);
      }
    }, 5000);

    // Return unsubscribe function
    return () => clearInterval(interval);
  }
}