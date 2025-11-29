/**
 * pSol Privacy Pool Client
 * Main SDK entry point for interacting with the protocol
 */

import {
  Connection,
  PublicKey,
  Transaction,
  sendAndConfirmTransaction,
} from '@solana/web3.js';
import {
  getAssociatedTokenAddress,
  getAccount,
} from '@solana/spl-token';
import { AnchorProvider, Wallet, BN } from '@coral-xyz/anchor';

import {
  PsolClientConfig,
  PoolConfig,
  PoolStatus,
  DepositNote,
  WithdrawProof,
  TransactionResult,
  InitializePoolParams,
  SetVerificationKeyParams,
  PROGRAM_ID,
  PSOL_CONSTANTS,
  PsolError,
  PsolErrorCode,
} from '../types';
import {
  derivePoolConfig,
  deriveVault,
  deriveMerkleTree,
  deriveAllPoolPDAs,
  isNullifierSpent,
} from '../utils/pda';
import {
  createDepositNote,
  computeNullifierHash,
} from '../utils/note';
import { MerkleTree } from '../crypto/merkle';
import { generateWithdrawProof } from '../crypto/proof';

/**
 * Main client for interacting with pSol Privacy Pool
 */
export class PsolClient {
  readonly connection: Connection;
  readonly programId: PublicKey;
  private wallet: Wallet | null = null;
  public provider: AnchorProvider | null = null;
  private config: PsolClientConfig;

  constructor(connection: Connection, config: PsolClientConfig = {}) {
    this.connection = connection;
    this.programId = config.programId || PROGRAM_ID;
    this.config = {
      commitment: 'confirmed',
      skipPreflight: false,
      ...config,
    };
  }

  connect(wallet: Wallet): PsolClient {
    this.wallet = wallet;
    this.provider = new AnchorProvider(
      this.connection,
      wallet,
      { commitment: this.config.commitment }
    );
    return this;
  }

  get publicKey(): PublicKey {
    if (!this.wallet) {
      throw new PsolError(PsolErrorCode.Unauthorized, 'Wallet not connected');
    }
    return this.wallet.publicKey;
  }

  async getPool(tokenMint: PublicKey): Promise<PoolConfig | null> {
    const [poolAddress] = derivePoolConfig(tokenMint, this.programId);
    return this.getPoolByAddress(poolAddress);
  }

  async getPoolByAddress(poolAddress: PublicKey): Promise<PoolConfig | null> {
    const account = await this.connection.getAccountInfo(poolAddress);
    if (!account) return null;
    return this.decodePoolConfig(account.data);
  }

  async getPoolStatus(tokenMint: PublicKey): Promise<PoolStatus | null> {
    const pool = await this.getPool(tokenMint);
    if (!pool) return null;

    const [poolAddress] = derivePoolConfig(tokenMint, this.programId);
    const treeCapacity = 2 ** pool.treeDepth;
    const tvl = pool.totalValueDeposited.sub(pool.totalValueWithdrawn);

    return {
      address: poolAddress,
      tokenMint: pool.tokenMint,
      isPaused: pool.isPaused,
      vkConfigured: pool.vkConfigured,
      vkLocked: pool.vkLocked,
      totalDeposits: pool.totalDeposits.toNumber(),
      totalWithdrawals: pool.totalWithdrawals.toNumber(),
      totalValueLocked: tvl,
      treeCapacity,
      treeUtilization: pool.totalDeposits.toNumber() / treeCapacity,
      authority: pool.authority,
      hasPendingAuthorityTransfer: !pool.pendingAuthority.equals(PublicKey.default),
    };
  }

  async poolExists(tokenMint: PublicKey): Promise<boolean> {
    const pool = await this.getPool(tokenMint);
    return pool !== null;
  }

  async generateDepositNote(tokenMint: PublicKey, amount: BN): Promise<DepositNote> {
    const [poolAddress] = derivePoolConfig(tokenMint, this.programId);
    return createDepositNote(poolAddress, tokenMint, amount);
  }

  async deposit(note: DepositNote): Promise<{ signature: string; note: DepositNote }> {
    this.requireWallet();

    const pool = await this.getPoolByAddress(note.pool);
    if (!pool) throw new PsolError(PsolErrorCode.InvalidMint, 'Pool not found');
    if (pool.isPaused) throw new PsolError(PsolErrorCode.PoolPaused, 'Pool is paused');

    const [vaultAddress] = deriveVault(note.pool, this.programId);
    const [merkleTreeAddress] = deriveMerkleTree(note.pool, this.programId);

    const userTokenAccount = await getAssociatedTokenAddress(note.tokenMint, this.publicKey);
    const tokenAccountInfo = await getAccount(this.connection, userTokenAccount);
    
    if (new BN(tokenAccountInfo.amount.toString()).lt(note.amount)) {
      throw new PsolError(PsolErrorCode.InsufficientBalance, 'Insufficient token balance');
    }

    const tx = this.buildDepositTransaction(
      note.pool, merkleTreeAddress, vaultAddress, userTokenAccount, note.amount, note.commitment
    );

    const signature = await sendAndConfirmTransaction(
      this.connection, tx, [this.wallet!.payer], { commitment: this.config.commitment }
    );

    const leafIndex = await this.getLeafIndexFromTx(signature);

    return {
      signature,
      note: { ...note, leafIndex, depositedAt: Date.now(), txSignature: signature },
    };
  }

  private buildDepositTransaction(
    _poolConfig: PublicKey, _merkleTree: PublicKey, _vault: PublicKey,
    _depositorTokenAccount: PublicKey, _amount: BN, _commitment: Uint8Array
  ): Transaction {
    void [_poolConfig, _merkleTree, _vault, _depositorTokenAccount, _amount, _commitment];
    throw new Error('Use with Anchor program - see examples');
  }

  async generateWithdrawProof(
    note: DepositNote, recipient: PublicKey, relayer: PublicKey = this.publicKey,
    relayerFee: BN = new BN(0), wasmPath: string, zkeyPath: string
  ): Promise<WithdrawProof> {
    if (note.leafIndex === undefined) {
      throw new PsolError(PsolErrorCode.InvalidNullifier, 'Note has not been deposited yet');
    }

    const merkleTree = await this.buildMerkleTreeFromChain(note.pool);
    const merkleProof = await merkleTree.generateProof(note.leafIndex);
    const nullifierHash = await computeNullifierHash(note);

    return generateWithdrawProof(
      note.secret, note.nullifier, merkleProof, recipient,
      note.amount, relayer, relayerFee, nullifierHash, wasmPath, zkeyPath
    );
  }

  async canWithdraw(note: DepositNote): Promise<boolean> {
    if (note.leafIndex === undefined) return false;
    const nullifierHash = await computeNullifierHash(note);
    return !(await isNullifierSpent(this.connection, note.pool, nullifierHash, this.programId));
  }

  async withdraw(proof: WithdrawProof, poolAddress: PublicKey): Promise<TransactionResult> {
    this.requireWallet();

    const pool = await this.getPoolByAddress(poolAddress);
    if (!pool) throw new PsolError(PsolErrorCode.InvalidMint, 'Pool not found');
    if (pool.isPaused) throw new PsolError(PsolErrorCode.PoolPaused, 'Pool is paused');

    const isSpent = await isNullifierSpent(
      this.connection, poolAddress, proof.publicInputs.nullifierHash, this.programId
    );
    if (isSpent) throw new PsolError(PsolErrorCode.NullifierAlreadySpent, 'Already withdrawn');

    const tx = this.buildWithdrawTransaction(poolAddress, pool, proof);
    const signature = await sendAndConfirmTransaction(
      this.connection, tx, [this.wallet!.payer], { commitment: this.config.commitment }
    );
    const txInfo = await this.connection.getTransaction(signature, { commitment: 'confirmed' });

    return { signature, slot: txInfo?.slot || 0, confirmations: null };
  }

  private buildWithdrawTransaction(
    _poolConfig: PublicKey, _pool: PoolConfig, _proof: WithdrawProof
  ): Transaction {
    void [_poolConfig, _pool, _proof];
    throw new Error('Use with Anchor program - see examples');
  }

  async initializePool(tokenMint: PublicKey, params: InitializePoolParams = {}): Promise<TransactionResult> {
    this.requireWallet();
    const treeDepth = params.treeDepth || PSOL_CONSTANTS.DEFAULT_TREE_DEPTH;
    if (treeDepth < PSOL_CONSTANTS.MIN_TREE_DEPTH || treeDepth > PSOL_CONSTANTS.MAX_TREE_DEPTH) {
      throw new PsolError(PsolErrorCode.InvalidTreeDepth, 'Invalid tree depth');
    }
    void [tokenMint, params, deriveAllPoolPDAs];
    throw new Error('Use with Anchor program - see examples');
  }

  async setVerificationKey(_tokenMint: PublicKey, _vkParams: SetVerificationKeyParams): Promise<TransactionResult> {
    this.requireWallet();
    throw new Error('Use with Anchor program - see examples');
  }

  async lockVerificationKey(_tokenMint: PublicKey): Promise<TransactionResult> {
    this.requireWallet();
    throw new Error('Use with Anchor program - see examples');
  }

  async initiateAuthorityTransfer(_tokenMint: PublicKey, _newAuthority: PublicKey): Promise<TransactionResult> {
    this.requireWallet();
    throw new Error('Use with Anchor program - see examples');
  }

  async acceptAuthorityTransfer(_tokenMint: PublicKey): Promise<TransactionResult> {
    this.requireWallet();
    throw new Error('Use with Anchor program - see examples');
  }

  async cancelAuthorityTransfer(_tokenMint: PublicKey): Promise<TransactionResult> {
    this.requireWallet();
    throw new Error('Use with Anchor program - see examples');
  }

  async pausePool(_tokenMint: PublicKey): Promise<TransactionResult> {
    this.requireWallet();
    throw new Error('Use with Anchor program - see examples');
  }

  async unpausePool(_tokenMint: PublicKey): Promise<TransactionResult> {
    this.requireWallet();
    throw new Error('Use with Anchor program - see examples');
  }

  private async buildMerkleTreeFromChain(poolAddress: PublicKey): Promise<MerkleTree> {
    const pool = await this.getPoolByAddress(poolAddress);
    if (!pool) throw new PsolError(PsolErrorCode.InvalidMint, 'Pool not found');
    const tree = new MerkleTree(pool.treeDepth);
    await tree.initialize();
    return tree;
  }

  private async getLeafIndexFromTx(signature: string): Promise<number> {
    const tx = await this.connection.getTransaction(signature, { commitment: 'confirmed' });
    if (!tx?.meta?.logMessages) {
      throw new PsolError(PsolErrorCode.CorruptedData, 'Transaction logs not found');
    }
    for (const log of tx.meta.logMessages) {
      const match = log.match(/leaf index: (\d+)/i);
      if (match) return parseInt(match[1], 10);
    }
    throw new PsolError(PsolErrorCode.CorruptedData, 'Leaf index not found in logs');
  }

  private decodePoolConfig(data: Buffer): PoolConfig {
    let offset = 8;
    const authority = new PublicKey(data.slice(offset, offset + 32)); offset += 32;
    const pendingAuthority = new PublicKey(data.slice(offset, offset + 32)); offset += 32;
    const tokenMint = new PublicKey(data.slice(offset, offset + 32)); offset += 32;
    const vault = new PublicKey(data.slice(offset, offset + 32)); offset += 32;
    const merkleTree = new PublicKey(data.slice(offset, offset + 32)); offset += 32;
    const verificationKey = new PublicKey(data.slice(offset, offset + 32)); offset += 32;
    const treeDepth = data.readUInt8(offset); offset += 1;
    const bump = data.readUInt8(offset); offset += 1;
    const isPaused = data.readUInt8(offset) === 1; offset += 1;
    const vkConfigured = data.readUInt8(offset) === 1; offset += 1;
    const vkLocked = data.readUInt8(offset) === 1; offset += 1;
    offset += 3;
    const totalDeposits = new BN(data.slice(offset, offset + 8), 'le'); offset += 8;
    const totalWithdrawals = new BN(data.slice(offset, offset + 8), 'le'); offset += 8;
    const totalValueDeposited = new BN(data.slice(offset, offset + 8), 'le'); offset += 8;
    const totalValueWithdrawn = new BN(data.slice(offset, offset + 8), 'le'); offset += 8;
    const version = data.readUInt8(offset);

    return {
      authority, pendingAuthority, tokenMint, vault, merkleTree, verificationKey,
      treeDepth, bump, isPaused, vkConfigured, vkLocked,
      totalDeposits, totalWithdrawals, totalValueDeposited, totalValueWithdrawn, version,
    };
  }

  private requireWallet(): void {
    if (!this.wallet) throw new PsolError(PsolErrorCode.Unauthorized, 'Wallet not connected');
  }
}
