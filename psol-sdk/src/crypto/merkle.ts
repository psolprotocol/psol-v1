/**
 * Merkle Tree Implementation
 * Sparse Merkle tree with Poseidon hash for privacy pool
 */


import { poseidonHash2, bnToBytes32 } from './poseidon';
import { MerkleProof, PSOL_CONSTANTS } from '../types';

/**
 * Compute zero values for each level of the tree
 * Zero value at level 0 is a constant, each subsequent level is hash(zero[i-1], zero[i-1])
 */
export async function computeZeroValues(depth: number): Promise<Uint8Array[]> {
  const zeros: Uint8Array[] = new Array(depth + 1);
  zeros[0] = bnToBytes32(PSOL_CONSTANTS.ZERO_VALUE);
  
  for (let i = 1; i <= depth; i++) {
    zeros[i] = await poseidonHash2(zeros[i - 1], zeros[i - 1]);
  }
  
  return zeros;
}

/**
 * Off-chain Merkle tree for proof generation
 */
export class MerkleTree {
  readonly depth: number;
  readonly capacity: number;
  
  private leaves: Map<number, Uint8Array> = new Map();
  private nodes: Map<string, Uint8Array> = new Map();
  private zeroValues: Uint8Array[] = [];
  private _root: Uint8Array | null = null;
  private nextIndex: number = 0;
  private initialized: boolean = false;

  constructor(depth: number = PSOL_CONSTANTS.DEFAULT_TREE_DEPTH) {
    if (depth < PSOL_CONSTANTS.MIN_TREE_DEPTH || depth > PSOL_CONSTANTS.MAX_TREE_DEPTH) {
      throw new Error(`Tree depth must be between ${PSOL_CONSTANTS.MIN_TREE_DEPTH} and ${PSOL_CONSTANTS.MAX_TREE_DEPTH}`);
    }
    this.depth = depth;
    this.capacity = 2 ** depth;
  }

  /**
   * Initialize the tree (must be called before use)
   */
  async initialize(): Promise<void> {
    if (this.initialized) return;
    
    this.zeroValues = await computeZeroValues(this.depth);
    this._root = this.zeroValues[this.depth];
    this.initialized = true;
  }

  /**
   * Ensure tree is initialized
   */
  private ensureInitialized(): void {
    if (!this.initialized) {
      throw new Error('MerkleTree not initialized. Call initialize() first.');
    }
  }

  /**
   * Get current root
   */
  get root(): Uint8Array {
    this.ensureInitialized();
    return this._root!;
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
   */
  async insert(leaf: Uint8Array): Promise<number> {
    this.ensureInitialized();
    
    if (this.isFull) {
      throw new Error('Merkle tree is full');
    }

    const index = this.nextIndex;
    this.leaves.set(index, leaf);
    
    // Update path from leaf to root
    let currentHash = leaf;
    let currentIndex = index;
    
    for (let level = 0; level < this.depth; level++) {
      const isRight = currentIndex % 2 === 1;
      const siblingIndex = isRight ? currentIndex - 1 : currentIndex + 1;
      
      // Get sibling (either existing node or zero value)
      const sibling = this.getNode(level, siblingIndex);
      
      // Store current node
      this.setNode(level, currentIndex, currentHash);
      
      // Hash to get parent
      if (isRight) {
        currentHash = await poseidonHash2(sibling, currentHash);
      } else {
        currentHash = await poseidonHash2(currentHash, sibling);
      }
      
      currentIndex = Math.floor(currentIndex / 2);
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
  async generateProof(leafIndex: number): Promise<MerkleProof> {
    this.ensureInitialized();
    
    if (leafIndex < 0 || leafIndex >= this.nextIndex) {
      throw new Error(`Invalid leaf index: ${leafIndex}`);
    }

    const pathElements: Uint8Array[] = [];
    const pathIndices: number[] = [];
    
    let currentIndex = leafIndex;
    
    for (let level = 0; level < this.depth; level++) {
      const isRight = currentIndex % 2 === 1;
      const siblingIndex = isRight ? currentIndex - 1 : currentIndex + 1;
      
      pathElements.push(this.getNode(level, siblingIndex));
      pathIndices.push(isRight ? 1 : 0);
      
      currentIndex = Math.floor(currentIndex / 2);
    }

    return {
      pathElements,
      pathIndices,
      root: this._root!,
      leafIndex,
    };
  }

  /**
   * Verify a Merkle proof
   */
  async verifyProof(leaf: Uint8Array, proof: MerkleProof): Promise<boolean> {
    this.ensureInitialized();
    
    if (proof.pathElements.length !== this.depth) {
      return false;
    }

    let currentHash = leaf;
    
    for (let i = 0; i < this.depth; i++) {
      const sibling = proof.pathElements[i];
      
      if (proof.pathIndices[i] === 1) {
        currentHash = await poseidonHash2(sibling, currentHash);
      } else {
        currentHash = await poseidonHash2(currentHash, sibling);
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
  static async fromLeaves(leaves: Uint8Array[], depth?: number): Promise<MerkleTree> {
    const treeDepth = depth || Math.max(
      PSOL_CONSTANTS.MIN_TREE_DEPTH,
      Math.ceil(Math.log2(leaves.length || 1))
    );
    
    const tree = new MerkleTree(treeDepth);
    await tree.initialize();
    
    for (const leaf of leaves) {
      await tree.insert(leaf);
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
  } {
    this.ensureInitialized();
    
    const leaves: Array<{ index: number; value: string }> = [];
    this.leaves.forEach((value, index) => {
      leaves.push({ index, value: Buffer.from(value).toString('hex') });
    });
    
    return {
      depth: this.depth,
      nextIndex: this.nextIndex,
      leaves,
    };
  }

  /**
   * Import tree state from export
   */
  static async importState(state: {
    depth: number;
    nextIndex: number;
    leaves: Array<{ index: number; value: string }>;
  }): Promise<MerkleTree> {
    const leaves = state.leaves
      .sort((a, b) => a.index - b.index)
      .map((l) => Buffer.from(l.value, 'hex'));
    
    return MerkleTree.fromLeaves(leaves.map(l => new Uint8Array(l)), state.depth);
  }
}

/**
 * Compute the root from a leaf and its proof
 */
export async function computeRootFromProof(
  leaf: Uint8Array,
  proof: MerkleProof
): Promise<Uint8Array> {
  let currentHash = leaf;
  
  for (let i = 0; i < proof.pathElements.length; i++) {
    const sibling = proof.pathElements[i];
    
    if (proof.pathIndices[i] === 1) {
      currentHash = await poseidonHash2(sibling, currentHash);
    } else {
      currentHash = await poseidonHash2(currentHash, sibling);
    }
  }
  
  return currentHash;
}
