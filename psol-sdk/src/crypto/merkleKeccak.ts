/**
 * Keccak256 Merkle Tree Implementation
 * 
 * IMPORTANT: This matches the ON-CHAIN Merkle tree implementation
 * which uses Keccak256 for internal node hashing.
 * 
 * The on-chain program uses:
 *   hash = keccak256(left || right)
 * 
 * Your ZK circuit MUST be configured to use the same hash function
 * for Merkle path verification.
 */

import { keccak_256 } from '@noble/hashes/sha3';
import BN from 'bn.js';
import { MerkleProof, PSOL_CONSTANTS } from '../types';
import { bnToBytes32, bytes32ToBN } from './poseidon';

/**
 * Keccak256 hash of two 32-byte values
 * Matches on-chain: keccak::hash(&combined).to_bytes()
 */
export function keccak256Hash2(left: Uint8Array, right: Uint8Array): Uint8Array {
  if (left.length !== 32 || right.length !== 32) {
    throw new Error('Both inputs must be 32 bytes');
  }
  
  const combined = new Uint8Array(64);
  combined.set(left, 0);
  combined.set(right, 32);
  
  return keccak_256(combined);
}

/**
 * Compute zero values for each level using Keccak256
 * zeros[0] = all zeros (empty leaf)
 * zeros[i] = keccak256(zeros[i-1], zeros[i-1])
 */
export function computeZeroValuesKeccak(depth: number): Uint8Array[] {
  const zeros: Uint8Array[] = new Array(depth + 1);
  zeros[0] = new Uint8Array(32); // All zeros for empty leaf
  
  for (let i = 1; i <= depth; i++) {
    zeros[i] = keccak256Hash2(zeros[i - 1], zeros[i - 1]);
  }
  
  return zeros;
}

/**
 * Keccak256-based Merkle tree for proof generation
 * This implementation matches the on-chain Solana program
 */
export class KeccakMerkleTree {
  readonly depth: number;
  readonly capacity: number;
  
  private leaves: Map<number, Uint8Array> = new Map();
  private nodes: Map<string, Uint8Array> = new Map();
  private zeroValues: Uint8Array[] = [];
  private _root: Uint8Array;
  private nextIndex: number = 0;

  constructor(depth: number = PSOL_CONSTANTS.DEFAULT_TREE_DEPTH) {
    if (depth < PSOL_CONSTANTS.MIN_TREE_DEPTH || depth > PSOL_CONSTANTS.MAX_TREE_DEPTH) {
      throw new Error(`Tree depth must be between ${PSOL_CONSTANTS.MIN_TREE_DEPTH} and ${PSOL_CONSTANTS.MAX_TREE_DEPTH}`);
    }
    this.depth = depth;
    this.capacity = 2 ** depth;
    
    // Initialize zero values (synchronous with Keccak)
    this.zeroValues = computeZeroValuesKeccak(depth);
    this._root = this.zeroValues[depth];
  }

  /**
   * Get current root
   */
  get root(): Uint8Array {
    return this._root;
  }

  /**
   * Get current number of leaves
   */
  get size(): number {
    return this.nextIndex;
  }

  /**
   * Check if tree is full
   */
  get isFull(): boolean {
    return this.nextIndex >= this.capacity;
  }

  /**
   * Insert a new leaf and return its index
   * This matches the on-chain insert_leaf function
   */
  insert(leaf: Uint8Array): number {
    if (leaf.length !== 32) {
      throw new Error('Leaf must be 32 bytes');
    }
    
    if (this.isFull) {
      throw new Error('Merkle tree is full');
    }

    const index = this.nextIndex;
    this.leaves.set(index, leaf);
    
    // Update path from leaf to root (matches on-chain logic)
    let currentHash = leaf;
    let currentIndex = index;
    
    for (let level = 0; level < this.depth; level++) {
      const isRight = (currentIndex & 1) === 1;
      
      // Store current node
      this.setNode(level, currentIndex, currentHash);
      
      if (isRight) {
        // Right child: hash with left sibling from filled_subtrees
        const leftSibling = this.getNode(level, currentIndex - 1);
        currentHash = keccak256Hash2(leftSibling, currentHash);
      } else {
        // Left child: hash with zero value for right sibling
        currentHash = keccak256Hash2(currentHash, this.zeroValues[level]);
      }
      
      currentIndex = currentIndex >> 1;
    }
    
    // Update root
    this._root = currentHash;
    this.setNode(this.depth, 0, currentHash);
    
    this.nextIndex++;
    return index;
  }

  /**
   * Generate Merkle proof for a leaf
   */
  generateProof(leafIndex: number): MerkleProof {
    if (leafIndex < 0 || leafIndex >= this.nextIndex) {
      throw new Error(`Invalid leaf index: ${leafIndex}`);
    }

    const pathElements: Uint8Array[] = [];
    const pathIndices: number[] = [];
    
    let currentIndex = leafIndex;
    
    for (let level = 0; level < this.depth; level++) {
      const isRight = (currentIndex & 1) === 1;
      const siblingIndex = isRight ? currentIndex - 1 : currentIndex + 1;
      
      pathElements.push(this.getNode(level, siblingIndex));
      pathIndices.push(isRight ? 1 : 0);
      
      currentIndex = currentIndex >> 1;
    }

    return {
      pathElements,
      pathIndices,
      root: this._root,
      leafIndex,
    };
  }

  /**
   * Verify a Merkle proof
   */
  verifyProof(leaf: Uint8Array, proof: MerkleProof): boolean {
    if (proof.pathElements.length !== this.depth) {
      return false;
    }

    let currentHash = leaf;
    
    for (let i = 0; i < this.depth; i++) {
      const sibling = proof.pathElements[i];
      
      if (proof.pathIndices[i] === 1) {
        currentHash = keccak256Hash2(sibling, currentHash);
      } else {
        currentHash = keccak256Hash2(currentHash, sibling);
      }
    }
    
    return Buffer.from(currentHash).equals(Buffer.from(proof.root));
  }

  /**
   * Get a node at a specific level and index
   */
  private getNode(level: number, index: number): Uint8Array {
    const key = `${level}-${index}`;
    return this.nodes.get(key) || this.zeroValues[level];
  }

  /**
   * Set a node at a specific level and index
   */
  private setNode(level: number, index: number, value: Uint8Array): void {
    const key = `${level}-${index}`;
    this.nodes.set(key, value);
  }

  /**
   * Get leaf at index
   */
  getLeaf(index: number): Uint8Array | undefined {
    return this.leaves.get(index);
  }

  /**
   * Build tree from array of leaves
   */
  static fromLeaves(leaves: Uint8Array[], depth?: number): KeccakMerkleTree {
    const treeDepth = depth || Math.max(
      PSOL_CONSTANTS.MIN_TREE_DEPTH,
      Math.ceil(Math.log2(leaves.length || 1))
    );
    
    const tree = new KeccakMerkleTree(treeDepth);
    
    for (const leaf of leaves) {
      tree.insert(leaf);
    }
    
    return tree;
  }

  /**
   * Export tree state for persistence
   */
  exportState(): {
    depth: number;
    nextIndex: number;
    leaves: Array<{ index: number; value: string }>;
    root: string;
  } {
    const leaves: Array<{ index: number; value: string }> = [];
    this.leaves.forEach((value, index) => {
      leaves.push({ index, value: Buffer.from(value).toString('hex') });
    });
    
    return {
      depth: this.depth,
      nextIndex: this.nextIndex,
      leaves,
      root: Buffer.from(this._root).toString('hex'),
    };
  }

  /**
   * Import tree state from export
   */
  static importState(state: {
    depth: number;
    nextIndex: number;
    leaves: Array<{ index: number; value: string }>;
  }): KeccakMerkleTree {
    const leaves = state.leaves
      .sort((a, b) => a.index - b.index)
      .map((l) => new Uint8Array(Buffer.from(l.value, 'hex')));
    
    return KeccakMerkleTree.fromLeaves(leaves, state.depth);
  }
}

/**
 * Compute the root from a leaf and its proof using Keccak256
 */
export function computeKeccakRootFromProof(
  leaf: Uint8Array,
  proof: MerkleProof
): Uint8Array {
  let currentHash = leaf;
  
  for (let i = 0; i < proof.pathElements.length; i++) {
    const sibling = proof.pathElements[i];
    
    if (proof.pathIndices[i] === 1) {
      currentHash = keccak256Hash2(sibling, currentHash);
    } else {
      currentHash = keccak256Hash2(currentHash, sibling);
    }
  }
  
  return currentHash;
}

export { KeccakMerkleTree as OnChainMerkleTree };
