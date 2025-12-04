/**
 * pSOL Privacy Pool - Complete Example Flow
 * ==========================================
 * Demonstrates: Pool initialization, deposit, and withdrawal flow
 * 
 * Prerequisites:
 * 1. Run `anchor build` to generate IDL
 * 2. Deploy program: `anchor deploy --provider.cluster devnet`
 * 3. Update PROGRAM_ID below with deployed address
 * 
 * Usage:
 * npx ts-node examples/complete-flow.ts
 */

import * as anchor from '@coral-xyz/anchor';
import { Program, AnchorProvider, BN, Wallet } from '@coral-xyz/anchor';
import {
  Connection,
  Keypair,
  PublicKey,
  LAMPORTS_PER_SOL,
  clusterApiUrl,
} from '@solana/web3.js';
import {
  TOKEN_PROGRAM_ID,
  createMint,
  createAccount,
  mintTo,
  getAccount,
} from '@solana/spl-token';
import * as fs from 'fs';
import * as path from 'path';

// ============================================================================
// CONFIGURATION
// ============================================================================

const PROGRAM_ID = new PublicKey('7kK3aVXN9nTv1dNubmr85FB85fK6PeRrDBsisu9Z4gQ9');
const CLUSTER = 'devnet';
const TREE_DEPTH = 20;
const ROOT_HISTORY_SIZE = 100;
const DEPOSIT_AMOUNT = new BN(1_000_000_000); // 1 token with 9 decimals

// ============================================================================
// UTILITIES
// ============================================================================

function log(msg: string) {
  console.log(`\x1b[32m[pSOL]\x1b[0m ${msg}`);
}

function warn(msg: string) {
  console.log(`\x1b[33m[WARN]\x1b[0m ${msg}`);
}

function error(msg: string) {
  console.error(`\x1b[31m[ERROR]\x1b[0m ${msg}`);
}

async function airdrop(connection: Connection, pubkey: PublicKey, amount: number = 2) {
  log(`Requesting ${amount} SOL airdrop...`);
  const sig = await connection.requestAirdrop(pubkey, amount * LAMPORTS_PER_SOL);
  await connection.confirmTransaction(sig);
  const balance = await connection.getBalance(pubkey);
  log(`Balance: ${balance / LAMPORTS_PER_SOL} SOL`);
}

function generateRandomCommitment(): Buffer {
  const commitment = Buffer.alloc(32);
  for (let i = 0; i < 32; i++) {
    commitment[i] = Math.floor(Math.random() * 256);
  }
  return commitment;
}

function derivePDAs(programId: PublicKey, tokenMint: PublicKey) {
  const [poolConfig] = PublicKey.findProgramAddressSync(
    [Buffer.from('pool'), tokenMint.toBuffer()],
    programId
  );

  const [merkleTree] = PublicKey.findProgramAddressSync(
    [Buffer.from('merkle_tree'), poolConfig.toBuffer()],
    programId
  );

  const [verificationKey] = PublicKey.findProgramAddressSync(
    [Buffer.from('verification_key'), poolConfig.toBuffer()],
    programId
  );

  const [vault] = PublicKey.findProgramAddressSync(
    [Buffer.from('vault'), poolConfig.toBuffer()],
    programId
  );

  return { poolConfig, merkleTree, verificationKey, vault };
}

// ============================================================================
// MAIN FLOW
// ============================================================================

async function main() {
  console.log('\n' + '='.repeat(60));
  console.log('  pSOL Privacy Pool - Complete Example Flow');
  console.log('='.repeat(60) + '\n');

  // ---------------------------------------------------------------------------
  // 1. Setup Connection and Wallet
  // ---------------------------------------------------------------------------
  log('Step 1: Setting up connection and wallet...');

  const connection = new Connection(clusterApiUrl(CLUSTER), 'confirmed');
  
  // Load or generate keypair
  let authority: Keypair;
  const keypairPath = path.join(process.env.HOME || '', '.config/solana/id.json');
  
  if (fs.existsSync(keypairPath)) {
    const secretKey = JSON.parse(fs.readFileSync(keypairPath, 'utf-8'));
    authority = Keypair.fromSecretKey(Uint8Array.from(secretKey));
    log(`Loaded wallet: ${authority.publicKey.toString()}`);
  } else {
    authority = Keypair.generate();
    log(`Generated new wallet: ${authority.publicKey.toString()}`);
    await airdrop(connection, authority.publicKey, 5);
  }

  // Check balance
  const balance = await connection.getBalance(authority.publicKey);
  if (balance < LAMPORTS_PER_SOL) {
    await airdrop(connection, authority.publicKey, 2);
  }

  // Setup Anchor provider
  const wallet: Wallet = {
    publicKey: authority.publicKey,
    signTransaction: async (tx) => { tx.partialSign(authority); return tx; },
    signAllTransactions: async (txs) => { txs.forEach(tx => tx.partialSign(authority)); return txs; },
  };
  const provider = new AnchorProvider(connection, wallet, { commitment: 'confirmed' });

  // Load IDL
  const idlPath = path.join(__dirname, '../psol-sdk/src/idl/psol_privacy.json');
  if (!fs.existsSync(idlPath)) {
    error('IDL not found. Run `anchor build` first.');
    process.exit(1);
  }
  const idl = JSON.parse(fs.readFileSync(idlPath, 'utf-8'));
  const program = new Program(idl, provider);

  // ---------------------------------------------------------------------------
  // 2. Create Test Token
  // ---------------------------------------------------------------------------
  log('\nStep 2: Creating test token...');

  const tokenMint = await createMint(
    connection,
    authority,
    authority.publicKey,
    null,
    9 // 9 decimals like SOL
  );
  log(`Token mint: ${tokenMint.toString()}`);

  // Create token account and mint tokens
  const userTokenAccount = await createAccount(
    connection,
    authority,
    tokenMint,
    authority.publicKey
  );
  log(`User token account: ${userTokenAccount.toString()}`);

  await mintTo(
    connection,
    authority,
    tokenMint,
    userTokenAccount,
    authority,
    10_000_000_000 // 10 tokens
  );
  log('Minted 10 tokens to user account');

  // ---------------------------------------------------------------------------
  // 3. Derive PDAs
  // ---------------------------------------------------------------------------
  log('\nStep 3: Deriving PDAs...');

  const { poolConfig, merkleTree, verificationKey, vault } = derivePDAs(PROGRAM_ID, tokenMint);
  
  log(`Pool Config: ${poolConfig.toString()}`);
  log(`Merkle Tree: ${merkleTree.toString()}`);
  log(`Verification Key: ${verificationKey.toString()}`);
  log(`Vault: ${vault.toString()}`);

  // ---------------------------------------------------------------------------
  // 4. Initialize Pool
  // ---------------------------------------------------------------------------
  log('\nStep 4: Initializing pool...');

  try {
    const initTx = await program.methods
      .initializePool(TREE_DEPTH, ROOT_HISTORY_SIZE)
      .accounts({
        authority: authority.publicKey,
        tokenMint,
        poolConfig,
        merkleTree,
        verificationKey,
        vault,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([authority])
      .rpc();

    log(`Pool initialized! TX: ${initTx}`);
    log(`Explorer: https://explorer.solana.com/tx/${initTx}?cluster=${CLUSTER}`);
  } catch (e: any) {
    if (e.message?.includes('already in use')) {
      warn('Pool already initialized, continuing...');
    } else {
      throw e;
    }
  }

  // Verify pool state
  const poolState = await program.account.poolConfig.fetch(poolConfig);
  log(`Pool authority: ${poolState.authority.toString()}`);
  log(`Tree depth: ${poolState.treeDepth}`);
  log(`Paused: ${poolState.isPaused}`);
  log(`VK configured: ${poolState.vkConfigured}`);

  // ---------------------------------------------------------------------------
  // 5. Set Verification Key (Mock for testing)
  // ---------------------------------------------------------------------------
  log('\nStep 5: Setting verification key...');

  if (!poolState.vkConfigured) {
    // Mock VK values - replace with real values from your trusted setup
    const mockVkAlphaG1 = new Array(64).fill(1);
    const mockVkBetaG2 = new Array(128).fill(2);
    const mockVkGammaG2 = new Array(128).fill(3);
    const mockVkDeltaG2 = new Array(128).fill(4);
    const mockVkIc = [
      new Array(64).fill(5),
      new Array(64).fill(6),
      new Array(64).fill(7),
      new Array(64).fill(8),
      new Array(64).fill(9),
      new Array(64).fill(10),
      new Array(64).fill(11),
    ];

    const vkTx = await program.methods
      .setVerificationKey(
        mockVkAlphaG1,
        mockVkBetaG2,
        mockVkGammaG2,
        mockVkDeltaG2,
        mockVkIc
      )
      .accounts({
        authority: authority.publicKey,
        poolConfig,
        verificationKey,
      })
      .signers([authority])
      .rpc();

    log(`VK set! TX: ${vkTx}`);
  } else {
    warn('VK already configured');
  }

  // ---------------------------------------------------------------------------
  // 6. Make Deposits
  // ---------------------------------------------------------------------------
  log('\nStep 6: Making deposits...');

  const notes: { commitment: Buffer; index: number }[] = [];

  for (let i = 0; i < 3; i++) {
    const commitment = generateRandomCommitment();
    
    const depositTx = await program.methods
      .deposit(DEPOSIT_AMOUNT, Array.from(commitment))
      .accounts({
        poolConfig,
        merkleTree,
        vault,
        depositorTokenAccount: userTokenAccount,
        depositor: authority.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([authority])
      .rpc();

    notes.push({ commitment, index: i });
    log(`Deposit ${i + 1}/3: ${DEPOSIT_AMOUNT.toString()} tokens`);
    log(`  Commitment: ${commitment.toString('hex').slice(0, 16)}...`);
    log(`  TX: ${depositTx}`);
  }

  // ---------------------------------------------------------------------------
  // 7. Verify Pool State
  // ---------------------------------------------------------------------------
  log('\nStep 7: Verifying pool state...');

  const finalPoolState = await program.account.poolConfig.fetch(poolConfig);
  const merkleState = await program.account.merkleTree.fetch(merkleTree);
  const vaultAccount = await getAccount(connection, vault);

  log(`Total deposits: ${finalPoolState.totalDeposits.toString()}`);
  log(`Total value deposited: ${finalPoolState.totalValueDeposited.toString()}`);
  log(`Vault balance: ${vaultAccount.amount.toString()}`);
  log(`Merkle tree leaves: ${merkleState.nextLeafIndex}`);
  log(`Current root: ${Buffer.from(merkleState.currentRoot).toString('hex').slice(0, 16)}...`);

  // ---------------------------------------------------------------------------
  // 8. Summary
  // ---------------------------------------------------------------------------
  console.log('\n' + '='.repeat(60));
  console.log('  Summary');
  console.log('='.repeat(60));
  console.log(`
  ✅ Pool initialized on ${CLUSTER}
  ✅ Verification key set
  ✅ ${notes.length} deposits made
  
  Pool Address: ${poolConfig.toString()}
  Token Mint:   ${tokenMint.toString()}
  Vault:        ${vault.toString()}
  
  Notes (save these for withdrawal):
  ${notes.map((n, i) => `    ${i + 1}. ${n.commitment.toString('hex').slice(0, 32)}...`).join('\n')}
  
  ⚠️  To withdraw, you need:
     1. A valid ZK proof (requires circuit setup)
     2. Merkle proof for your commitment
     3. Nullifier hash computed correctly
  
  See DEPLOYMENT_GUIDE.md for circuit setup instructions.
  `);
  console.log('='.repeat(60) + '\n');
}

// Run
main()
  .then(() => process.exit(0))
  .catch((err) => {
    error(err.message || err);
    console.error(err);
    process.exit(1);
  });
