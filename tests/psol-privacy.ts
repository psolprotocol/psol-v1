import BN from "bn.js";
import assert from "assert";
import * as web3 from "@solana/web3.js";
import * as anchor from "@coral-xyz/anchor";
import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PublicKey, Keypair, SystemProgram } from "@solana/web3.js";
import {
  TOKEN_PROGRAM_ID,
  createMint,
  mintTo,
  getOrCreateAssociatedTokenAccount,
} from "@solana/spl-token";
import assert from "assert";
import type { Crypto } from "../target/types/crypto";

describe("pSol Privacy Protocol", () => {
  // Configure the client to use the local cluster
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.Crypto as anchor.Program<Crypto>;
  
  // Configure the client
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.PsolPrivacy as Program<any>;
  const payer = provider.wallet as anchor.Wallet;

  let tokenMint: PublicKey;
  let userTokenAccount: PublicKey;
  let poolConfig: PublicKey;
  let merkleTree: PublicKey;
  let nullifierSet: PublicKey;
  let vault: PublicKey;

  // ============================================
  // MINIMAL PARAMETERS FOR SOLANA PLAYGROUND
  // ============================================
  const TREE_DEPTH = 5; // Reduced from 10
  const ROOT_HISTORY_SIZE = 10; // Reduced from 30

  before(async () => {
    console.log("Setting up test environment...");
    console.log(
      `Tree Depth: ${TREE_DEPTH}, Root History Size: ${ROOT_HISTORY_SIZE}`
    );

    // Create test token mint
    tokenMint = await createMint(
      provider.connection,
      payer.payer,
      payer.publicKey,
      null,
      9 // 9 decimals
    );
    console.log("Token mint created:", tokenMint.toString());

    // Create user token account and mint tokens
    const account = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      payer.payer,
      tokenMint,
      payer.publicKey
    );
    userTokenAccount = account.address;

    // Mint 1000 tokens to user
    await mintTo(
      provider.connection,
      payer.payer,
      tokenMint,
      userTokenAccount,
      payer.publicKey,
      1000_000000000 // 1000 tokens with 9 decimals
    );
    console.log("Minted 1000 tokens to user");

    // Derive PDAs (must match Rust seeds)
    [poolConfig] = PublicKey.findProgramAddressSync(
      [Buffer.from("pool"), tokenMint.toBuffer()],
      program.programId
    );

    [merkleTree] = PublicKey.findProgramAddressSync(
      [Buffer.from("merkle_tree"), poolConfig.toBuffer()],
      program.programId
    );

    [nullifierSet] = PublicKey.findProgramAddressSync(
      [Buffer.from("nullifiers"), poolConfig.toBuffer()],
      program.programId
    );

    [vault] = PublicKey.findProgramAddressSync(
      [Buffer.from("vault"), poolConfig.toBuffer()],
      program.programId
    );

    console.log("PDAs derived:");
    console.log("  Pool Config:", poolConfig.toString());
    console.log("  Merkle Tree:", merkleTree.toString());
    console.log("  Nullifier Set:", nullifierSet.toString());
    console.log("  Vault:", vault.toString());
  });

  it("Initializes the privacy pool", async () => {
    console.log("\n--- Initializing Pool ---");

    try {
      const tx = await program.methods
        .initializePool(TREE_DEPTH, ROOT_HISTORY_SIZE)
        .accounts({
          poolConfig,
          merkleTree,
          nullifierSet,
          vault,
          tokenMint,
          authority: payer.publicKey,
          systemProgram: SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        })
        .rpc({ commitment: "confirmed" });

      console.log("Pool initialized. Transaction:", tx);

      // Wait for confirmation
      await provider.connection.confirmTransaction(tx, "confirmed");

      // Fetch and verify pool state
      const poolAccount = await program.account.poolConfig.fetch(poolConfig);
      assert.strictEqual(
        poolAccount.authority.toString(),
        payer.publicKey.toString(),
        "Authority mismatch"
      );
      assert.strictEqual(
        poolAccount.tokenMint.toString(),
        tokenMint.toString(),
        "Token mint mismatch"
      );
      assert.strictEqual(
        poolAccount.treeDepth,
        TREE_DEPTH,
        "Tree depth mismatch"
      );
      assert.strictEqual(
        poolAccount.totalDeposits.toNumber(),
        0,
        "Total deposits should be 0"
      );
      assert.strictEqual(
        poolAccount.isPaused,
        false,
        "Pool should not be paused"
      );

      console.log("Pool state verified successfully");
    } catch (error: any) {
      console.error("Initialization failed:", error.toString());

      // Log more details for debugging
      if (error.logs) {
        console.error("Transaction logs:", error.logs);
      }
      throw error;
    }
  });

  it("Makes a deposit", async () => {
    const amount = new anchor.BN(10_000000000); // 10 tokens

    // Generate random commitment (in production, this would be computed properly)
    const commitment = Array.from(Keypair.generate().publicKey.toBytes());

    const tx = await program.methods
      .deposit(amount, commitment)
      .accounts({
        poolConfig,
        merkleTree,
        vault,
        userTokenAccount,
        user: payer.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .rpc({ commitment: "confirmed" });

    console.log("Deposit successful. Transaction:", tx);

    // Wait for confirmation
    await provider.connection.confirmTransaction(tx, "confirmed");

    // Verify pool state updated
    const poolAccount = await program.account.poolConfig.fetch(poolConfig);
    assert.strictEqual(
      poolAccount.totalDeposits.toNumber(),
      1,
      "Total deposits should be 1"
    );

    // Verify merkle tree updated
    const treeAccount = await program.account.merkleTree.fetch(merkleTree);
    assert.strictEqual(
      treeAccount.nextLeafIndex,
      1,
      "Next leaf index should be 1"
    );

    console.log("Deposit verified successfully");
    console.log(
      "New merkle root:",
      Buffer.from(treeAccount.currentRoot).toString("hex").slice(0, 16) + "..."
    );
  });

  it("Makes multiple deposits", async () => {
    const amount = new anchor.BN(5_000000000); // 5 tokens

    for (let i = 0; i < 3; i++) {
      const commitment = Array.from(Keypair.generate().publicKey.toBytes());

      const tx = await program.methods
        .deposit(amount, commitment)
        .accounts({
          poolConfig,
          merkleTree,
          vault,
          userTokenAccount,
          user: payer.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .rpc({ commitment: "confirmed" });

      // Wait for confirmation
      await provider.connection.confirmTransaction(tx, "confirmed");

      console.log(`Deposit ${i + 2} successful`);
    }

    // Verify pool state
    const poolAccount = await program.account.poolConfig.fetch(poolConfig);
    assert.strictEqual(
      poolAccount.totalDeposits.toNumber(),
      4,
      "Total deposits should be 4"
    );

    console.log("Multiple deposits verified successfully");
  });

  it("Fails to deposit with zero amount", async () => {
    const commitment = Array.from(Keypair.generate().publicKey.toBytes());

    try {
      await program.methods
        .deposit(new anchor.BN(0), commitment)
        .accounts({
          poolConfig,
          merkleTree,
          vault,
          userTokenAccount,
          user: payer.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .rpc();

      assert.fail("Should have thrown error for zero amount");
    } catch (error: any) {
      const msg = error.toString();
      assert.ok(
        msg.includes("InvalidAmount") || msg.includes("6005"),
        `Error should contain InvalidAmount, got: ${msg}`
      );
      console.log("Correctly rejected zero amount deposit");
    }
  });

  it("Fails to deposit with zero commitment", async () => {
    const commitment = new Array<number>(32).fill(0);
    const amount = new anchor.BN(1_000000000);

    try {
      await program.methods
        .deposit(amount, commitment)
        .accounts({
          poolConfig,
          merkleTree,
          vault,
          userTokenAccount,
          user: payer.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .rpc();

      assert.fail("Should have thrown error for zero commitment");
    } catch (error: any) {
      const msg = error.toString();
      assert.ok(
        msg.includes("InvalidCommitment") || msg.includes("6007"),
        `Error should contain InvalidCommitment, got: ${msg}`
      );
      console.log("Correctly rejected zero commitment");
    }
  });

  it("Attempts withdrawal (will fail or pass depending on verifier mode)", async () => {
    const nullifier = Array.from(Keypair.generate().publicKey.toBytes());
    const recipient = payer.publicKey;
    const amount = new anchor.BN(5_000000000);

    // Get current merkle root
    const treeAccount = await program.account.merkleTree.fetch(merkleTree);
    const merkleRoot = treeAccount.currentRoot as number[];

    // Create dummy proof data (128 bytes)
    const proofData = new Array<number>(128).fill(1);

    try {
      await program.methods
        .withdraw(
          nullifier,
          recipient,
          amount,
          merkleRoot,
          Buffer.from(proofData)
        )
        .accounts({
          poolConfig,
          merkleTree,
          nullifierSet,
          vault,
          recipientTokenAccount: userTokenAccount,
          tokenProgram: TOKEN_PROGRAM_ID,
          withdrawer: payer.publicKey,
        })
        .rpc();

      console.log("Withdrawal executed (development verifier)");
      const nullifierSetAccount = await program.account.nullifierSet.fetch(
        nullifierSet
      );
      console.log("Nullifiers spent:", nullifierSetAccount.count.toNumber());
    } catch (error: any) {
      console.log(
        "Withdrawal failed as expected with current verifier:",
        error.toString().slice(0, 100) + "..."
      );
    }
  });

  it("Fetches final pool state", async () => {
    const poolAccount = await program.account.poolConfig.fetch(poolConfig);
    const treeAccount = await program.account.merkleTree.fetch(merkleTree);
    const nullifierSetAccount = await program.account.nullifierSet.fetch(
      nullifierSet
    );

    console.log("\n=== Final Pool State ===");
    console.log("Total Deposits:", poolAccount.totalDeposits.toNumber());
    console.log("Total Withdrawals:", poolAccount.totalWithdrawals.toNumber());
    console.log("Tree Depth:", poolAccount.treeDepth);
    console.log(
      "Current Root:",
      Buffer.from(treeAccount.currentRoot).toString("hex").slice(0, 32) + "..."
    );
    console.log("Next Leaf Index:", treeAccount.nextLeafIndex);
    console.log("Nullifiers Spent:", nullifierSetAccount.count.toNumber());
    console.log("Pool Paused:", poolAccount.isPaused);
    console.log("========================\n");
  });
});
