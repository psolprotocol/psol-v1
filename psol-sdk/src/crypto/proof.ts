/**
 * ZK Proof Generation
 * Interface with snarkjs for Groth16 proof generation
 */

import * as snarkjs from 'snarkjs';
import BN from 'bn.js';
import { PublicKey } from '@solana/web3.js';
import { WithdrawProof, MerkleProof, PSOL_CONSTANTS } from '../types';
import { bnToBytes32, bytes32ToBN } from './poseidon';

/**
 * Circuit input for withdraw proof
 */
export interface WithdrawCircuitInput {
  [key: string]: string | string[] | number[];
  // Private inputs
  secret: string;
  nullifier: string;
  pathElements: string[];
  pathIndices: number[];
  
  // Public inputs
  root: string;
  nullifierHash: string;
  recipient: string;
  amount: string;
  relayer: string;
  relayerFee: string;
}

/**
 * Groth16 proof structure from snarkjs
 */
export interface Groth16Proof {
  pi_a: [string, string, string];
  pi_b: [[string, string], [string, string], [string, string]];
  pi_c: [string, string, string];
  protocol: string;
  curve: string;
}

/**
 * Generate withdraw proof using snarkjs
 * Requires circuit WASM and proving key files
 */
export async function generateWithdrawProof(
  secret: Uint8Array,
  nullifier: Uint8Array,
  merkleProof: MerkleProof,
  recipient: PublicKey,
  amount: BN,
  relayer: PublicKey,
  relayerFee: BN,
  nullifierHash: Uint8Array,
  wasmPath: string,
  zkeyPath: string
): Promise<WithdrawProof> {
  // Prepare circuit inputs
  const input: WithdrawCircuitInput = {
    // Private inputs
    secret: bytes32ToBN(secret).toString(),
    nullifier: bytes32ToBN(nullifier).toString(),
    pathElements: merkleProof.pathElements.map((e) => bytes32ToBN(e).toString()),
    pathIndices: merkleProof.pathIndices,
    
    // Public inputs
    root: bytes32ToBN(merkleProof.root).toString(),
    nullifierHash: bytes32ToBN(nullifierHash).toString(),
    recipient: new BN(recipient.toBuffer()).toString(),
    amount: amount.toString(),
    relayer: new BN(relayer.toBuffer()).toString(),
    relayerFee: relayerFee.toString(),
  };

  // Generate proof using snarkjs
  const { proof, publicSignals } = await snarkjs.groth16.fullProve(
    input,
    wasmPath,
    zkeyPath
  );

  // Convert proof to on-chain format (256 bytes)
  const proofData = serializeProof(proof as Groth16Proof);

  return {
    proofData,
    publicInputs: {
      merkleRoot: merkleProof.root,
      nullifierHash,
      recipient,
      amount,
      relayer,
      relayerFee,
    },
  };
}

/**
 * Serialize Groth16 proof to 256 bytes for on-chain verification
 * Format: [pi_a (64), pi_b (128), pi_c (64)]
 */
export function serializeProof(proof: Groth16Proof): Uint8Array {
  const result = new Uint8Array(256);
  
  // pi_a: 2 field elements (each 32 bytes)
  const piA = serializeG1Point(proof.pi_a);
  result.set(piA, 0);
  
  // pi_b: 2x2 field elements (128 bytes total)
  const piB = serializeG2Point(proof.pi_b);
  result.set(piB, 64);
  
  // pi_c: 2 field elements (each 32 bytes)
  const piC = serializeG1Point(proof.pi_c);
  result.set(piC, 192);
  
  return result;
}

/**
 * Deserialize proof from 256 bytes
 */
export function deserializeProof(data: Uint8Array): Groth16Proof {
  if (data.length !== 256) {
    throw new Error('Invalid proof data length');
  }

  return {
    pi_a: deserializeG1Point(data.slice(0, 64)),
    pi_b: deserializeG2Point(data.slice(64, 192)),
    pi_c: deserializeG1Point(data.slice(192, 256)),
    protocol: 'groth16',
    curve: 'bn128',
  };
}

/**
 * Serialize G1 point (64 bytes)
 */
function serializeG1Point(point: [string, string, string]): Uint8Array {
  const result = new Uint8Array(64);
  
  const x = new BN(point[0]);
  const y = new BN(point[1]);
  
  result.set(bnToBytes32(x), 0);
  result.set(bnToBytes32(y), 32);
  
  return result;
}

/**
 * Deserialize G1 point from 64 bytes
 */
function deserializeG1Point(data: Uint8Array): [string, string, string] {
  const x = bytes32ToBN(data.slice(0, 32));
  const y = bytes32ToBN(data.slice(32, 64));
  
  return [x.toString(), y.toString(), '1'];
}

/**
 * Serialize G2 point (128 bytes)
 * G2 points have coordinates in extension field Fp2, so each coordinate has 2 components
 */
function serializeG2Point(
  point: [[string, string], [string, string], [string, string]]
): Uint8Array {
  const result = new Uint8Array(128);
  
  // x coordinate (2 field elements)
  const x0 = new BN(point[0][0]);
  const x1 = new BN(point[0][1]);
  result.set(bnToBytes32(x0), 0);
  result.set(bnToBytes32(x1), 32);
  
  // y coordinate (2 field elements)
  const y0 = new BN(point[1][0]);
  const y1 = new BN(point[1][1]);
  result.set(bnToBytes32(y0), 64);
  result.set(bnToBytes32(y1), 96);
  
  return result;
}

/**
 * Deserialize G2 point from 128 bytes
 */
function deserializeG2Point(
  data: Uint8Array
): [[string, string], [string, string], [string, string]] {
  const x0 = bytes32ToBN(data.slice(0, 32));
  const x1 = bytes32ToBN(data.slice(32, 64));
  const y0 = bytes32ToBN(data.slice(64, 96));
  const y1 = bytes32ToBN(data.slice(96, 128));
  
  return [
    [x0.toString(), x1.toString()],
    [y0.toString(), y1.toString()],
    ['1', '0'],
  ];
}

/**
 * Verify proof locally (for testing)
 */
export async function verifyProofLocally(
  proof: Groth16Proof,
  publicSignals: string[],
  vkeyPath: string
): Promise<boolean> {
  const vkey = await fetch(vkeyPath).then((r) => r.json());
  return snarkjs.groth16.verify(vkey, publicSignals, proof);
}

/**
 * Generate public signals array from inputs
 */
export function generatePublicSignals(
  merkleRoot: Uint8Array,
  nullifierHash: Uint8Array,
  recipient: PublicKey,
  amount: BN,
  relayer: PublicKey,
  relayerFee: BN
): string[] {
  return [
    bytes32ToBN(merkleRoot).toString(),
    bytes32ToBN(nullifierHash).toString(),
    new BN(recipient.toBuffer()).toString(),
    amount.toString(),
    new BN(relayer.toBuffer()).toString(),
    relayerFee.toString(),
  ];
}

/**
 * Parse verification key from JSON file
 */
export interface VerificationKeyJson {
  protocol: string;
  curve: string;
  nPublic: number;
  vk_alpha_1: string[];
  vk_beta_2: string[][];
  vk_gamma_2: string[][];
  vk_delta_2: string[][];
  vk_alphabeta_12: string[][][];
  IC: string[][];
}

/**
 * Convert verification key JSON to on-chain format
 */
export function vkeyJsonToOnChain(vkey: VerificationKeyJson): {
  alphaG1: Uint8Array;
  betaG2: Uint8Array;
  gammaG2: Uint8Array;
  deltaG2: Uint8Array;
  ic: Uint8Array[];
} {
  return {
    alphaG1: serializeG1Point([vkey.vk_alpha_1[0], vkey.vk_alpha_1[1], '1']),
    betaG2: serializeG2Point([
      [vkey.vk_beta_2[0][0], vkey.vk_beta_2[0][1]],
      [vkey.vk_beta_2[1][0], vkey.vk_beta_2[1][1]],
      ['1', '0'],
    ]),
    gammaG2: serializeG2Point([
      [vkey.vk_gamma_2[0][0], vkey.vk_gamma_2[0][1]],
      [vkey.vk_gamma_2[1][0], vkey.vk_gamma_2[1][1]],
      ['1', '0'],
    ]),
    deltaG2: serializeG2Point([
      [vkey.vk_delta_2[0][0], vkey.vk_delta_2[0][1]],
      [vkey.vk_delta_2[1][0], vkey.vk_delta_2[1][1]],
      ['1', '0'],
    ]),
    ic: vkey.IC.map((point) =>
      serializeG1Point([point[0], point[1], '1'])
    ),
  };
}
