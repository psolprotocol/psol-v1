/**
 * Poseidon Hash Implementation
 * Compatible with circomlib's Poseidon implementation
 */

import { buildPoseidon, Poseidon } from 'circomlibjs';
import BN from 'bn.js';
import { PSOL_CONSTANTS } from '../types';

let poseidonInstance: Poseidon | null = null;

/**
 * Initialize Poseidon hasher (lazy singleton)
 */
export async function getPoseidon(): Promise<Poseidon> {
  if (!poseidonInstance) {
    poseidonInstance = await buildPoseidon();
  }
  return poseidonInstance;
}

/**
 * Poseidon hash of arbitrary number of inputs
 * @param inputs - Array of BN or Uint8Array inputs
 * @returns 32-byte hash as Uint8Array
 */
export async function poseidonHash(
  inputs: (BN | Uint8Array | bigint | number)[]
): Promise<Uint8Array> {
  const poseidon = await getPoseidon();
  
  // Convert inputs to field elements
  const fieldInputs = inputs.map((input) => {
    if (input instanceof BN) {
      return BigInt(input.toString());
    } else if (input instanceof Uint8Array) {
      return BigInt('0x' + Buffer.from(input).toString('hex'));
    } else if (typeof input === 'bigint') {
      return input;
    } else {
      return BigInt(input);
    }
  });

  // Hash using circomlib's Poseidon
  const hash = poseidon(fieldInputs);
  
  // Convert to Uint8Array (32 bytes, big-endian)
  const hashBN = new BN(poseidon.F.toString(hash));
  return bnToBytes32(hashBN);
}

/**
 * Poseidon hash of two inputs (most common case)
 */
export async function poseidonHash2(
  left: BN | Uint8Array,
  right: BN | Uint8Array
): Promise<Uint8Array> {
  return poseidonHash([left, right]);
}

/**
 * Generate commitment from secret and nullifier
 * commitment = Poseidon(secret, nullifier)
 */
export async function generateCommitment(
  secret: Uint8Array,
  nullifier: Uint8Array
): Promise<Uint8Array> {
  return poseidonHash2(secret, nullifier);
}

/**
 * Generate nullifier hash from nullifier and leaf index
 * nullifierHash = Poseidon(nullifier, leafIndex)
 */
export async function generateNullifierHash(
  nullifier: Uint8Array,
  leafIndex: number
): Promise<Uint8Array> {
  return poseidonHash([nullifier, leafIndex]);
}

/**
 * Convert BN to 32-byte Uint8Array (big-endian)
 */
export function bnToBytes32(bn: BN): Uint8Array {
  const hex = bn.toString(16).padStart(64, '0');
  const bytes = new Uint8Array(32);
  for (let i = 0; i < 32; i++) {
    bytes[i] = parseInt(hex.slice(i * 2, i * 2 + 2), 16);
  }
  return bytes;
}

/**
 * Convert Uint8Array to BN (big-endian)
 */
export function bytes32ToBN(bytes: Uint8Array): BN {
  return new BN(Buffer.from(bytes).toString('hex'), 16);
}

/**
 * Generate cryptographically secure random bytes
 */
export function randomBytes(length: number): Uint8Array {
  if (typeof window !== 'undefined' && window.crypto) {
    // Browser environment
    const bytes = new Uint8Array(length);
    window.crypto.getRandomValues(bytes);
    return bytes;
  } else {
    // Node.js environment
    const crypto = require('crypto');
    return new Uint8Array(crypto.randomBytes(length));
  }
}

/**
 * Generate a random field element (< FIELD_SIZE)
 */
export function randomFieldElement(): Uint8Array {
  let bytes: Uint8Array;
  let bn: BN;
  
  // Keep generating until we get a value less than field size
  do {
    bytes = randomBytes(32);
    bn = bytes32ToBN(bytes);
  } while (bn.gte(PSOL_CONSTANTS.FIELD_SIZE));
  
  return bytes;
}

/**
 * Check if a value is a valid field element
 */
export function isValidFieldElement(value: BN | Uint8Array): boolean {
  const bn = value instanceof Uint8Array ? bytes32ToBN(value) : value;
  return bn.lt(PSOL_CONSTANTS.FIELD_SIZE) && bn.gtn(0);
}

/**
 * Hash public inputs for proof verification
 * This creates the exact format expected by the on-chain verifier
 */
export async function hashPublicInputs(
  merkleRoot: Uint8Array,
  nullifierHash: Uint8Array,
  recipient: Uint8Array,
  amount: BN,
  relayer: Uint8Array,
  relayerFee: BN
): Promise<Uint8Array[]> {
  // Return as array of field elements for circuit input
  return [
    merkleRoot,
    nullifierHash,
    recipient.slice(0, 32), // First 32 bytes of pubkey
    bnToBytes32(amount),
    relayer.slice(0, 32),
    bnToBytes32(relayerFee),
  ];
}
