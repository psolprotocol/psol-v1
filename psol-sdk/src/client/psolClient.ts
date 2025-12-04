/**
 * pSOL Privacy Pool SDK Client
 * Complete client for interacting with pSOL privacy pool
 */

import {
  Connection,
  PublicKey,
  Keypair,
  Transaction,
  SystemProgram,
  TransactionInstruction,
  sendAndConfirmTransaction,
} from '@solana/web3.js';
import {
  TOKEN_PROGRAM_ID,
  getAssociatedTokenAddress,
  createAssociatedTokenAccountInstruction,
  getAccount,
} from '@solana/spl-token';
import { Program, AnchorProvider, BN, Wallet } from '@coral-xyz/anchor';
import { createHash, randomBytes } from 'crypto';

// Import IDL - will be generated after anchor build
// import idl from './idl/psol_privacy.json';

export const PROGRAM_ID = new PublicKey('7kK3aVXN9nTv1dNubmr85FB85fK6PeRrDBsisu9Z4gQ9');

/**
 * PDA derivation utilities
 */
export class PsolPDA {
  constructor(public programId: PublicKey = PROGRAM_ID) {}

  poolConfig(tokenMint: PublicKey): [PublicKey, number] {
    return PublicKey.findProgramAddressSync(
      [Buffer.from('pool'), tokenMint.toBuffer()],
      this.programId
    );
  }

  merkleTree(poolConfig: PublicKey): [PublicKey, number] {
    return PublicKey.findProgramAddressSync(
      [Buffer.from('merkle_tree'), poolConfig.toBuffer()],
      this.programId
    );
  }

  verificationKey(poolConfig: PublicKey): [PublicKey, number] {
    return PublicKey.findProgramAddressSync(
      [Buffer.from('verification_key'), poolConfig.toBuffer()],
      this.programId
    );
  }

  vault(poolConfig: PublicKey): [PublicKey, number] {
    return PublicKey.findProgramAddressSync(
      [Buffer.from('vault'), poolConfig.toBuffer()],
      this.programId
    );
  }

  spentNullifier(poolConfig: PublicKey, nullifierHash: Buffer): [PublicKey, number] {
    return PublicKey.findProgramAddressSync(
      [Buffer.from('nullifier'), poolConfig.toBuffer(), nullifierHash],
      this.programId
    );
  }
}

/**
 * Note - represents a private commitment
 */
export interface Note {
  secret: Buffer;
  nullifier: Buffer;
  commitment: Buffer;
  amount: BN;
  leafIndex?: number;
}

/**
 * Generate a new note for deposit
 */
export function generateNote(amount: BN): Note {
  const secret = randomBytes(32);
  const nullifier = randomBytes(32);
  
  // In production, use Poseidon hash via circomlibjs
  // commitment = Poseidon(secret, nullifier, amount)
  // For now, use placeholder
  const commitment = createHash('sha256')
    .update(Buffer.concat([secret, nullifier, amount.toBuffer('le', 8)]))
    .digest();

  return {
    secret,
    nullifier,
    commitment,
    amount,
  };
}

/**
 * Compute nullifier hash
 */
export function computeNullifierHash(note: Note): Buffer {
  // In production: Poseidon(nullifier, secret)
  return createHash('sha256')
    .update(Buffer.concat([note.nullifier, note.secret]))
    .digest();
}

/**
 * Serialize note to JSON-safe format
 */
export function serializeNote(note: Note): string {
  return JSON.stringify({
    secret: note.secret.toString('hex'),
    nullifier: note.nullifier.toString('hex'),
    commitment: note.commitment.toString('hex'),
    amount: note.amount.toString(),
    leafIndex: note.leafIndex,
  });
}

/**
 * Deserialize note from JSON
 */
export function deserializeNote(json: string): Note {
  const data = JSON.parse(json);
  return {
    secret: Buffer.from(data.secret, 'hex'),
    nullifier: Buffer.from(data.nullifier, 'hex'),
    commitment: Buffer.from(data.commitment, 'hex'),
    amount: new BN(data.amount),
    leafIndex: data.leafIndex,
  };
}

/**
 * Pool information
 */
export interface PoolInfo {
  address: PublicKey;
  authority: PublicKey;
  tokenMint: PublicKey;
  vault: PublicKey;
  treeDepth: number;
  isPaused: boolean;
  vkConfigured: boolean;
  vkLocked: boolean;
  totalDeposits: BN;
  totalWithdrawals: BN;
  totalValueDeposited: BN;
  totalValueWithdrawn: BN;
}

/**
 * Main pSOL Client
 */
export class PsolClient {
  public connection: Connection;
  public wallet: Keypair;
  public programId: PublicKey;
  public pda: PsolPDA;
  public provider: AnchorProvider;

  constructor(
    connection: Connection,
    wallet: Keypair,
    programId: PublicKey = PROGRAM_ID
  ) {
    this.connection = connection;
    this.wallet = wallet;
    this.programId = programId;
    this.pda = new PsolPDA(programId);
    
    const walletAdapter: Wallet = {
      publicKey: wallet.publicKey,
      signTransaction: async (tx) => {
        tx.partialSign(wallet);
        return tx;
      },
      signAllTransactions: async (txs) => {
        txs.forEach(tx => tx.partialSign(wallet));
        return txs;
      },
    };
    
    this.provider = new AnchorProvider(connection, walletAdapter, {
      commitment: 'confirmed',
    });
  }

  /**
   * Get pool info
   */
  async getPoolInfo(tokenMint: PublicKey): Promise<PoolInfo | null> {
    const [poolConfig] = this.pda.poolConfig(tokenMint);
    
    try {
      const accountInfo = await this.connection.getAccountInfo(poolConfig);
      if (!accountInfo) return null;
      
      // Decode account data (requires IDL)
      // For now, return placeholder
      return {
        address: poolConfig,
        authority: PublicKey.default,
        tokenMint,
        vault: this.pda.vault(poolConfig)[0],
        treeDepth: 20,
        isPaused: false,
        vkConfigured: false,
        vkLocked: false,
        totalDeposits: new BN(0),
        totalWithdrawals: new BN(0),
        totalValueDeposited: new BN(0),
        totalValueWithdrawn: new BN(0),
      };
    } catch {
      return null;
    }
  }

  /**
   * Initialize a new pool
   */
  async initializePool(
    tokenMint: PublicKey,
    treeDepth: number = 20,
    rootHistorySize: number = 100
  ): Promise<string> {
    const [poolConfig] = this.pda.poolConfig(tokenMint);
    const [merkleTree] = this.pda.merkleTree(poolConfig);
    const [verificationKey] = this.pda.verificationKey(poolConfig);
    const [vault] = this.pda.vault(poolConfig);

    // Build instruction manually (requires IDL for Anchor method)
    const discriminator = this.getInstructionDiscriminator('initialize_pool');
    
    const data = Buffer.concat([
      discriminator,
      Buffer.from([treeDepth]),
      Buffer.from(new Uint16Array([rootHistorySize]).buffer),
    ]);

    const ix = new TransactionInstruction({
      keys: [
        { pubkey: this.wallet.publicKey, isSigner: true, isWritable: true },
        { pubkey: tokenMint, isSigner: false, isWritable: false },
        { pubkey: poolConfig, isSigner: false, isWritable: true },
        { pubkey: merkleTree, isSigner: false, isWritable: true },
        { pubkey: verificationKey, isSigner: false, isWritable: true },
        { pubkey: vault, isSigner: false, isWritable: true },
        { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
        { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      ],
      programId: this.programId,
      data,
    });

    const tx = new Transaction().add(ix);
    return sendAndConfirmTransaction(this.connection, tx, [this.wallet]);
  }

  /**
   * Deposit tokens
   */
  async deposit(
    tokenMint: PublicKey,
    amount: BN,
    note?: Note
  ): Promise<{ signature: string; note: Note }> {
    const depositNote = note || generateNote(amount);
    
    const [poolConfig] = this.pda.poolConfig(tokenMint);
    const [merkleTree] = this.pda.merkleTree(poolConfig);
    const [vault] = this.pda.vault(poolConfig);
    
    const depositorTokenAccount = await getAssociatedTokenAddress(
      tokenMint,
      this.wallet.publicKey
    );

    const discriminator = this.getInstructionDiscriminator('deposit');
    
    const data = Buffer.concat([
      discriminator,
      amount.toBuffer('le', 8),
      depositNote.commitment,
    ]);

    const ix = new TransactionInstruction({
      keys: [
        { pubkey: poolConfig, isSigner: false, isWritable: true },
        { pubkey: merkleTree, isSigner: false, isWritable: true },
        { pubkey: vault, isSigner: false, isWritable: true },
        { pubkey: depositorTokenAccount, isSigner: false, isWritable: true },
        { pubkey: this.wallet.publicKey, isSigner: true, isWritable: true },
        { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      ],
      programId: this.programId,
      data,
    });

    const tx = new Transaction().add(ix);
    const signature = await sendAndConfirmTransaction(this.connection, tx, [this.wallet]);
    
    // Get leaf index from transaction logs
    // depositNote.leafIndex = parseLeafIndexFromLogs(logs);
    
    return { signature, note: depositNote };
  }

  /**
   * Withdraw tokens (requires proof)
   */
  async withdraw(
    tokenMint: PublicKey,
    proofData: Buffer,
    merkleRoot: Buffer,
    nullifierHash: Buffer,
    recipient: PublicKey,
    amount: BN,
    relayer: PublicKey,
    relayerFee: BN
  ): Promise<string> {
    const [poolConfig] = this.pda.poolConfig(tokenMint);
    const [merkleTree] = this.pda.merkleTree(poolConfig);
    const [verificationKey] = this.pda.verificationKey(poolConfig);
    const [vault] = this.pda.vault(poolConfig);
    const [spentNullifier] = this.pda.spentNullifier(poolConfig, nullifierHash);

    const recipientTokenAccount = await getAssociatedTokenAddress(
      tokenMint,
      recipient
    );
    
    const relayerTokenAccount = await getAssociatedTokenAddress(
      tokenMint,
      relayer
    );

    const discriminator = this.getInstructionDiscriminator('withdraw');
    
    // Serialize instruction data
    const proofLengthBuf = Buffer.alloc(4);
    proofLengthBuf.writeUInt32LE(proofData.length);
    
    const data = Buffer.concat([
      discriminator,
      proofLengthBuf,
      proofData,
      merkleRoot,
      nullifierHash,
      recipient.toBuffer(),
      amount.toBuffer('le', 8),
      relayer.toBuffer(),
      relayerFee.toBuffer('le', 8),
    ]);

    const ix = new TransactionInstruction({
      keys: [
        { pubkey: poolConfig, isSigner: false, isWritable: true },
        { pubkey: merkleTree, isSigner: false, isWritable: false },
        { pubkey: verificationKey, isSigner: false, isWritable: false },
        { pubkey: spentNullifier, isSigner: false, isWritable: true },
        { pubkey: vault, isSigner: false, isWritable: true },
        { pubkey: recipientTokenAccount, isSigner: false, isWritable: true },
        { pubkey: relayerTokenAccount, isSigner: false, isWritable: true },
        { pubkey: this.wallet.publicKey, isSigner: true, isWritable: true },
        { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
        { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      ],
      programId: this.programId,
      data,
    });

    const tx = new Transaction().add(ix);
    return sendAndConfirmTransaction(this.connection, tx, [this.wallet]);
  }

  /**
   * Pause pool (admin only)
   */
  async pausePool(tokenMint: PublicKey): Promise<string> {
    const [poolConfig] = this.pda.poolConfig(tokenMint);
    const discriminator = this.getInstructionDiscriminator('pause_pool');

    const ix = new TransactionInstruction({
      keys: [
        { pubkey: this.wallet.publicKey, isSigner: true, isWritable: false },
        { pubkey: poolConfig, isSigner: false, isWritable: true },
      ],
      programId: this.programId,
      data: discriminator,
    });

    const tx = new Transaction().add(ix);
    return sendAndConfirmTransaction(this.connection, tx, [this.wallet]);
  }

  /**
   * Unpause pool (admin only)
   */
  async unpausePool(tokenMint: PublicKey): Promise<string> {
    const [poolConfig] = this.pda.poolConfig(tokenMint);
    const discriminator = this.getInstructionDiscriminator('unpause_pool');

    const ix = new TransactionInstruction({
      keys: [
        { pubkey: this.wallet.publicKey, isSigner: true, isWritable: false },
        { pubkey: poolConfig, isSigner: false, isWritable: true },
      ],
      programId: this.programId,
      data: discriminator,
    });

    const tx = new Transaction().add(ix);
    return sendAndConfirmTransaction(this.connection, tx, [this.wallet]);
  }

  /**
   * Get Anchor instruction discriminator
   */
  private getInstructionDiscriminator(name: string): Buffer {
    const hash = createHash('sha256')
      .update(`global:${name}`)
      .digest();
    return hash.slice(0, 8);
  }
}

export default PsolClient;
