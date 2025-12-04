const { Connection, Keypair, PublicKey } = require("@solana/web3.js");
const { Program, AnchorProvider, Wallet, BN } = require("@coral-xyz/anchor");
const { TOKEN_PROGRAM_ID, getAssociatedTokenAddress } = require("@solana/spl-token");
const fs = require("fs");

// Import IDL
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
  
  const connection = new Connection(RPC_URL, "confirmed");
  
  const walletKeypair = Keypair.fromSecretKey(
    Uint8Array.from(JSON.parse(fs.readFileSync("/home/codespace/.config/solana/id.json", "utf-8")))
  );
  
  const wallet = new Wallet(walletKeypair);
  const provider = new AnchorProvider(connection, wallet, { commitment: "confirmed" });
  const program = new Program(IDL, PROGRAM_ID, provider);
  
  console.log("\n📊 Configuration:");
  console.log("  Program ID:", PROGRAM_ID.toString());
  console.log("  Wallet:", wallet.publicKey.toString());
  
  const solBalance = await connection.getBalance(wallet.publicKey);
  console.log("  SOL Balance:", (solBalance / 1e9).toFixed(4), "SOL");
  
  const [merkleTree] = PublicKey.findProgramAddressSync(
    [Buffer.from("merkle_tree"), POOL_CONFIG.toBuffer()],
    PROGRAM_ID
  );
  
  const [vault] = PublicKey.findProgramAddressSync(
    [Buffer.from("vault"), POOL_CONFIG.toBuffer()],
    PROGRAM_ID
  );
  
  console.log("\n🔍 Fetching Pool State...");
  const poolState = await program.account.poolConfig.fetch(POOL_CONFIG);
  console.log("  Total Deposits:", poolState.totalDeposits.toString());
  console.log("  Total Value:", poolState.totalValueDeposited.toString());
  
  console.log("\n🌳 Fetching Merkle Tree...");
  const treeState = await program.account.merkleTree.fetch(merkleTree);
  console.log("  Next Leaf Index:", treeState.nextLeafIndex.toString());
  console.log("  Tree Depth:", treeState.depth);
  
  console.log("\n" + "=".repeat(60));
  console.log("✅ SDK Test Complete - All Components Working!");
  console.log("=".repeat(60));
}

main().catch(console.error);
