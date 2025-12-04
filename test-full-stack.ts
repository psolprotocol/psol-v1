import { Connection, Keypair, PublicKey } from "@solana/web3.js";
import { Program, AnchorProvider, Wallet } from "@coral-xyz/anchor";
import { TOKEN_PROGRAM_ID, getAssociatedTokenAddress } from "@solana/spl-token";
import { BN } from "@coral-xyz/anchor";
import * as fs from "fs";

// Import your IDL
const IDL = JSON.parse(fs.readFileSync("./psol-sdk/src/idl/psol_privacy.json", "utf-8"));

// Configuration
const PROGRAM_ID = new PublicKey("2uPHpGmCNoTk6mnzzuP3DGbVyMiDPrQYRxkYBHMxwhBi");
const TOKEN_MINT = new PublicKey("9cnm3fpXqBBUU8byYq8rZbeCbMxvCReh5LF6XSjqaaoJ");
const POOL_CONFIG = new PublicKey("EmeSBaC18Arn626HjyvYicGjXzjg4cx1wA1jV91w1NFD");
const RPC_URL = "https://api.devnet.solana.com";

async function main() {
  console.log("=".repeat(60));
  console.log("pSol Privacy Protocol - Full Stack Test");
  console.log("=".repeat(60));
  
  // Setup connection
  const connection = new Connection(RPC_URL, "confirmed");
  
  // Load wallet
  const walletKeypair = Keypair.fromSecretKey(
    Uint8Array.from(JSON.parse(fs.readFileSync("/home/codespace/.config/solana/id.json", "utf-8")))
  );
  
  const wallet = new Wallet(walletKeypair);
  const provider = new AnchorProvider(connection, wallet, {
    commitment: "confirmed"
  });
  
  const program = new Program(IDL, PROGRAM_ID, provider);
  
  console.log("\n📊 Configuration:");
  console.log("  Program ID:", PROGRAM_ID.toString());
  console.log("  Pool Config:", POOL_CONFIG.toString());
  console.log("  Token Mint:", TOKEN_MINT.toString());
  console.log("  Wallet:", wallet.publicKey.toString());
  
  // Get wallet SOL balance
  const solBalance = await connection.getBalance(wallet.publicKey);
  console.log("  SOL Balance:", solBalance / 1e9, "SOL");
  
  // Get token account
  const userTokenAccount = await getAssociatedTokenAddress(
    TOKEN_MINT,
    wallet.publicKey
  );
  
  console.log("  Token Account:", userTokenAccount.toString());
  
  try {
    const tokenBalance = await connection.getTokenAccountBalance(userTokenAccount);
    console.log("  Token Balance:", tokenBalance.value.uiAmount, "tokens");
  } catch (e) {
    console.log("  Token Balance: Account not found");
  }
  
  // Derive PDAs
  const [merkleTree] = PublicKey.findProgramAddressSync(
    [Buffer.from("merkle_tree"), POOL_CONFIG.toBuffer()],
    PROGRAM_ID
  );
  
  const [vault] = PublicKey.findProgramAddressSync(
    [Buffer.from("vault"), POOL_CONFIG.toBuffer()],
    PROGRAM_ID
  );
  
  console.log("\n📍 Derived Accounts:");
  console.log("  Merkle Tree:", merkleTree.toString());
  console.log("  Vault:", vault.toString());
  
  // Fetch pool state
  console.log("\n🔍 Fetching Pool State...");
  try {
    const poolState = await program.account.poolConfig.fetch(POOL_CONFIG);
    console.log("  Total Deposits:", poolState.totalDeposits.toString());
    console.log("  Total Withdrawals:", poolState.totalWithdrawals.toString());
    console.log("  Total Value Deposited:", poolState.totalValueDeposited.toString());
    console.log("  Is Paused:", poolState.isPaused);
    console.log("  VK Configured:", poolState.vkConfigured);
  } catch (e) {
    console.log("  Error fetching pool state:", e.message);
  }
  
  // Fetch merkle tree state
  console.log("\n🌳 Fetching Merkle Tree State...");
  try {
    const treeState = await program.account.merkleTree.fetch(merkleTree);
    console.log("  Next Leaf Index:", treeState.nextLeafIndex);
    console.log("  Tree Depth:", treeState.depth);
    console.log("  Root History Size:", treeState.rootHistorySize);
    console.log("  Current Root:", Buffer.from(treeState.currentRoot).toString("hex").slice(0, 16) + "...");
  } catch (e) {
    console.log("  Error fetching tree state:", e.message);
  }
  
  // Test deposit
  console.log("\n💰 Testing Deposit...");
  console.log("  Generating random commitment...");
  
  const commitment = new Uint8Array(32);
  for (let i = 0; i < 32; i++) {
    commitment[i] = Math.floor(Math.random() * 256);
  }
  
  console.log("  Commitment:", Buffer.from(commitment).toString("hex"));
  
  const amount = new BN(100000000); // 0.1 tokens
  console.log("  Amount:", amount.toString(), "lamports (0.1 tokens)");
  
  try {
    const tx = await program.methods
      .deposit(amount, Array.from(commitment))
      .accounts({
        poolConfig: POOL_CONFIG,
        merkleTree: merkleTree,
        vault: vault,
        depositorTokenAccount: userTokenAccount,
        depositor: wallet.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .rpc();
    
    console.log("\n✅ Deposit Successful!");
    console.log("  Transaction:", tx);
    console.log("  Explorer:", `https://explorer.solana.com/tx/${tx}?cluster=devnet`);
    
  } catch (e) {
    console.log("\n❌ Deposit Failed:");
    console.log("  Error:", e.message);
    if (e.logs) {
      console.log("  Logs:", e.logs.join("\n"));
    }
  }
  
  console.log("\n" + "=".repeat(60));
  console.log("Test Complete!");
  console.log("=".repeat(60));
}

main().catch(console.error);
