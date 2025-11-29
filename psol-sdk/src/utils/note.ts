/**
 * Deposit Note Management
 * Create, serialize, and parse deposit notes
 */

import { PublicKey } from '@solana/web3.js';
import BN from 'bn.js';
import {
  DepositNote,
  SerializedNote,
  ParsedNote,
  PSOL_CONSTANTS,
  PsolError,
  PsolErrorCode,
} from '../types';
import {
  randomFieldElement,
  generateCommitment,
  generateNullifierHash,
  bnToBytes32,
  bytes32ToBN,
} from '../crypto/poseidon';

const NOTE_VERSION = 1;
const NOTE_PREFIX = 'psol';

/**
 * Generate a new deposit note with random secrets
 */
export async function createDepositNote(
  pool: PublicKey,
  tokenMint: PublicKey,
  amount: BN
): Promise<DepositNote> {
  // Validate amount
  if (amount.lten(0)) {
    throw new PsolError(PsolErrorCode.InvalidAmount, 'Amount must be positive');
  }
  if (amount.gt(PSOL_CONSTANTS.MAX_DEPOSIT_AMOUNT)) {
    throw new PsolError(
      PsolErrorCode.LimitExceeded,
      `Amount exceeds maximum: ${PSOL_CONSTANTS.MAX_DEPOSIT_AMOUNT.toString()}`
    );
  }

  // Generate random secret and nullifier
  const secret = randomFieldElement();
  const nullifier = randomFieldElement();

  // Compute commitment
  const commitment = await generateCommitment(secret, nullifier);

  return {
    secret,
    nullifier,
    commitment,
    amount,
    pool,
    tokenMint,
  };
}

/**
 * Serialize a deposit note to a string for storage/sharing
 * Format: psol:v1:<base64_data>
 * 
 * Data structure:
 * - secret: 32 bytes
 * - nullifier: 32 bytes
 * - amount: 8 bytes (u64 LE)
 * - pool: 32 bytes
 * - tokenMint: 32 bytes
 * - leafIndex: 4 bytes (u32 LE, optional, 0xFFFFFFFF if not set)
 */
export function serializeNote(note: DepositNote): SerializedNote {
  const buffer = Buffer.alloc(140); // 32 + 32 + 8 + 32 + 32 + 4
  let offset = 0;

  // Secret
  buffer.set(note.secret, offset);
  offset += 32;

  // Nullifier
  buffer.set(note.nullifier, offset);
  offset += 32;

  // Amount (8 bytes, little-endian)
  const amountBuf = note.amount.toArrayLike(Buffer, 'le', 8);
  buffer.set(amountBuf, offset);
  offset += 8;

  // Pool pubkey
  buffer.set(note.pool.toBytes(), offset);
  offset += 32;

  // Token mint pubkey
  buffer.set(note.tokenMint.toBytes(), offset);
  offset += 32;

  // Leaf index (optional)
  const leafIndex = note.leafIndex ?? 0xffffffff;
  buffer.writeUInt32LE(leafIndex, offset);

  const base64Data = buffer.toString('base64');
  return `${NOTE_PREFIX}:v${NOTE_VERSION}:${base64Data}`;
}

/**
 * Parse a serialized note back to a DepositNote
 */
export async function parseNote(serialized: SerializedNote): Promise<ParsedNote> {
  // Validate format
  const parts = serialized.split(':');
  if (parts.length !== 3 || parts[0] !== NOTE_PREFIX) {
    throw new PsolError(
      PsolErrorCode.CorruptedData,
      'Invalid note format: must be psol:v<version>:<data>'
    );
  }

  // Parse version
  const versionMatch = parts[1].match(/^v(\d+)$/);
  if (!versionMatch) {
    throw new PsolError(PsolErrorCode.CorruptedData, 'Invalid note version format');
  }
  const version = parseInt(versionMatch[1], 10);

  if (version !== NOTE_VERSION) {
    throw new PsolError(
      PsolErrorCode.CorruptedData,
      `Unsupported note version: ${version}`
    );
  }

  // Decode base64 data
  let buffer: Buffer;
  try {
    buffer = Buffer.from(parts[2], 'base64');
  } catch {
    throw new PsolError(PsolErrorCode.CorruptedData, 'Invalid base64 data');
  }

  if (buffer.length !== 140) {
    throw new PsolError(
      PsolErrorCode.CorruptedData,
      `Invalid note data length: expected 140, got ${buffer.length}`
    );
  }

  let offset = 0;

  // Secret
  const secret = new Uint8Array(buffer.slice(offset, offset + 32));
  offset += 32;

  // Nullifier
  const nullifier = new Uint8Array(buffer.slice(offset, offset + 32));
  offset += 32;

  // Amount
  const amount = new BN(buffer.slice(offset, offset + 8), 'le');
  offset += 8;

  // Pool
  const pool = new PublicKey(buffer.slice(offset, offset + 32));
  offset += 32;

  // Token mint
  const tokenMint = new PublicKey(buffer.slice(offset, offset + 32));
  offset += 32;

  // Leaf index
  const leafIndexRaw = buffer.readUInt32LE(offset);
  const leafIndex = leafIndexRaw === 0xffffffff ? undefined : leafIndexRaw;

  return {
    version,
    secret,
    nullifier,
    amount,
    pool,
    tokenMint,
    leafIndex,
  };
}

/**
 * Compute the nullifier hash for a note (needed for withdrawal)
 */
export async function computeNullifierHash(
  note: DepositNote | ParsedNote
): Promise<Uint8Array> {
  if (note.leafIndex === undefined) {
    throw new PsolError(
      PsolErrorCode.InvalidNullifier,
      'Cannot compute nullifier hash without leaf index'
    );
  }
  return generateNullifierHash(note.nullifier, note.leafIndex);
}

/**
 * Verify that a note's commitment is valid
 */
export async function verifyNoteCommitment(note: DepositNote): Promise<boolean> {
  const expectedCommitment = await generateCommitment(note.secret, note.nullifier);
  return Buffer.from(note.commitment).equals(Buffer.from(expectedCommitment));
}

/**
 * Create a note from parsed data (reconstitute full DepositNote)
 */
export async function noteFromParsed(parsed: ParsedNote): Promise<DepositNote> {
  const commitment = await generateCommitment(parsed.secret, parsed.nullifier);
  
  return {
    secret: parsed.secret,
    nullifier: parsed.nullifier,
    commitment,
    amount: parsed.amount,
    pool: parsed.pool,
    tokenMint: parsed.tokenMint,
    leafIndex: parsed.leafIndex,
  };
}

/**
 * Generate a human-readable note summary (for display, not security!)
 */
export function noteToSummary(note: DepositNote | ParsedNote): string {
  const amountStr = note.amount.toString();
  const poolShort = note.pool.toBase58().slice(0, 8);
  const hasLeaf = 'leafIndex' in note && note.leafIndex !== undefined;
  
  return `pSol Note: ${amountStr} tokens | Pool: ${poolShort}... | ${
    hasLeaf ? `Deposited (leaf #${note.leafIndex})` : 'Pending'
  }`;
}

/**
 * Validate a note's structure and values
 */
export function validateNote(note: DepositNote | ParsedNote): void {
  // Check secret
  if (note.secret.length !== 32) {
    throw new PsolError(PsolErrorCode.InvalidSecret, 'Secret must be 32 bytes');
  }
  if (note.secret.every((b) => b === 0)) {
    throw new PsolError(PsolErrorCode.InvalidSecret, 'Secret cannot be all zeros');
  }

  // Check nullifier
  if (note.nullifier.length !== 32) {
    throw new PsolError(PsolErrorCode.InvalidNullifier, 'Nullifier must be 32 bytes');
  }
  if (note.nullifier.every((b) => b === 0)) {
    throw new PsolError(PsolErrorCode.InvalidNullifier, 'Nullifier cannot be all zeros');
  }

  // Check amount
  if (note.amount.lten(0)) {
    throw new PsolError(PsolErrorCode.InvalidAmount, 'Amount must be positive');
  }
  if (note.amount.gt(PSOL_CONSTANTS.MAX_DEPOSIT_AMOUNT)) {
    throw new PsolError(PsolErrorCode.LimitExceeded, 'Amount exceeds maximum');
  }
}

/**
 * Encrypt a note for secure storage (using recipient pubkey)
 * Uses a simple XOR with derived key - in production, use proper encryption
 */
export function encryptNote(
  serialized: SerializedNote,
  password: string
): string {
  // Simple encryption for demo - use proper crypto in production!
  const encoder = new TextEncoder();
  const data = encoder.encode(serialized);
  const key = encoder.encode(password);
  
  const encrypted = new Uint8Array(data.length);
  for (let i = 0; i < data.length; i++) {
    encrypted[i] = data[i] ^ key[i % key.length];
  }
  
  return `enc:${Buffer.from(encrypted).toString('base64')}`;
}

/**
 * Decrypt an encrypted note
 */
export function decryptNote(encrypted: string, password: string): SerializedNote {
  if (!encrypted.startsWith('enc:')) {
    throw new PsolError(PsolErrorCode.CorruptedData, 'Invalid encrypted format');
  }
  
  const encoder = new TextEncoder();
  const decoder = new TextDecoder();
  const data = Buffer.from(encrypted.slice(4), 'base64');
  const key = encoder.encode(password);
  
  const decrypted = new Uint8Array(data.length);
  for (let i = 0; i < data.length; i++) {
    decrypted[i] = data[i] ^ key[i % key.length];
  }
  
  return decoder.decode(decrypted);
}
