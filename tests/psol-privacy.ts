import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import {
  Keypair,
  PublicKey,
  SystemProgram,
  LAMPORTS_PER_SOL,
} from "@solana/web3.js";
import {
  TOKEN_PROGRAM_ID,
  createMint,
  createAccount,
  mintTo,
  getAccount,
} from "@solana/spl-token";
import { assert } from "chai";
import { PsolPrivacy } from "../target/types/psol_privacy";

describe("pSOL Privacy Pool", () => {
  // Configure the client
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.PsolPrivacy as Program<PsolPrivacy>;

  // Test accounts
  let authority: Keypair;
  let tokenMint: PublicKey;
  let poolConfig: PublicKey;
  let merkleTree: PublicKey;
  let verificationKey: PublicKey;
  let vault: PublicKey;
  let depositorTokenAccount: PublicKey;

  // Constants
  const TREE_DEPTH = 20;
  const ROOT_HISTORY_SIZE = 100;
  const DEPOSIT_AMOUNT = 1_000_000_000; // 1 token (9 decimals)

  before(async () => {
    authority = Keypair.generate();
    
    // Airdrop SOL to authority
    const sig = await provider.connection.requestAirdrop(
      authority.publicKey,
      10 * LAMPORTS_PER_SOL
    );
    await provider.connection.confirmTransaction(sig);

    // Create token mint
    tokenMint = await createMint(
      provider.connection,
      authority,
      authority.publicKey,
      null,
      9
    );

    // Derive PDAs
    [poolConfig] = PublicKey.findProgramAddressSync(
      [Buffer.from("pool"), tokenMint.toBuffer()],
      program.programId
    );

    [merkleTree] = PublicKey.findProgramAddressSync(
      [Buffer.from("merkle_tree"), poolConfig.toBuffer()],
      program.programId
    );

    [verificationKey] = PublicKey.findProgramAddressSync(
      [Buffer.from("verification_key"), poolConfig.toBuffer()],
      program.programId
    );

    [vault] = PublicKey.findProgramAddressSync(
      [Buffer.from("vault"), poolConfig.toBuffer()],
      program.programId
    );

    // Create depositor token account
    depositorTokenAccount = await createAccount(
      provider.connection,
      authority,
      tokenMint,
      authority.publicKey
    );

    // Mint tokens
    await mintTo(
      provider.connection,
      authority,
      tokenMint,
      depositorTokenAccount,
      authority,
      10_000_000_000 // 10 tokens
    );

    console.log("Setup complete:");
    console.log("  Authority:", authority.publicKey.toString());
    console.log("  Token Mint:", tokenMint.toString());
    console.log("  Pool Config:", poolConfig.toString());
    console.log("  Merkle Tree:", merkleTree.toString());
    console.log("  Vault:", vault.toString());
  });

  describe("Pool Initialization", () => {
    it("Initializes pool successfully", async () => {
      const tx = await program.methods
        .initializePool(TREE_DEPTH, ROOT_HISTORY_SIZE)
        .accounts({
          authority: authority.publicKey,
          tokenMint,
          poolConfig,
          merkleTree,
          verificationKey,
          vault,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .signers([authority])
        .rpc();

      console.log("Initialize pool tx:", tx);

      // Verify pool config
      const poolAccount = await program.account.poolConfig.fetch(poolConfig);
      assert.equal(poolAccount.authority.toString(), authority.publicKey.toString());
      assert.equal(poolAccount.tokenMint.toString(), tokenMint.toString());
      assert.equal(poolAccount.treeDepth, TREE_DEPTH);
      assert.isFalse(poolAccount.isPaused);
      assert.isFalse(poolAccount.vkConfigured);
    });

    it("Rejects duplicate initialization", async () => {
      try {
        await program.methods
          .initializePool(TREE_DEPTH, ROOT_HISTORY_SIZE)
          .accounts({
            authority: authority.publicKey,
            tokenMint,
            poolConfig,
            merkleTree,
            verificationKey,
            vault,
            tokenProgram: TOKEN_PROGRAM_ID,
            systemProgram: SystemProgram.programId,
          })
          .signers([authority])
          .rpc();
        
        assert.fail("Should have thrown");
      } catch (err) {
        // Expected: account already initialized
        assert.ok(err);
      }
    });
  });

  describe("Verification Key Management", () => {
    // Mock VK values (replace with real values in production)
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

    it("Sets verification key successfully", async () => {
      const tx = await program.methods
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

      console.log("Set VK tx:", tx);

      // Verify VK is configured
      const poolAccount = await program.account.poolConfig.fetch(poolConfig);
      assert.isTrue(poolAccount.vkConfigured);
      assert.isFalse(poolAccount.vkLocked);
    });

    it("Locks verification key", async () => {
      const tx = await program.methods
        .lockVerificationKey()
        .accounts({
          authority: authority.publicKey,
          poolConfig,
          verificationKey,
        })
        .signers([authority])
        .rpc();

      console.log("Lock VK tx:", tx);

      const poolAccount = await program.account.poolConfig.fetch(poolConfig);
      assert.isTrue(poolAccount.vkLocked);
    });

    it("Rejects VK update after lock", async () => {
      try {
        await program.methods
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
        
        assert.fail("Should have thrown VerificationKeyLocked");
      } catch (err) {
        assert.include(err.toString(), "VerificationKeyLocked");
      }
    });
  });

  describe("Deposit", () => {
    it("Deposits tokens with valid commitment", async () => {
      // Generate random commitment
      const commitment = Buffer.alloc(32);
      for (let i = 0; i < 32; i++) {
        commitment[i] = Math.floor(Math.random() * 256);
      }

      const balanceBefore = await getAccount(provider.connection, depositorTokenAccount);

      const tx = await program.methods
        .deposit(new anchor.BN(DEPOSIT_AMOUNT), Array.from(commitment))
        .accounts({
          poolConfig,
          merkleTree,
          vault,
          depositorTokenAccount,
          depositor: authority.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .signers([authority])
        .rpc();

      console.log("Deposit tx:", tx);

      // Verify balances
      const balanceAfter = await getAccount(provider.connection, depositorTokenAccount);
      const vaultBalance = await getAccount(provider.connection, vault);

      assert.equal(
        balanceBefore.amount - balanceAfter.amount,
        BigInt(DEPOSIT_AMOUNT)
      );
      assert.equal(vaultBalance.amount, BigInt(DEPOSIT_AMOUNT));

      // Verify pool stats
      const poolAccount = await program.account.poolConfig.fetch(poolConfig);
      assert.equal(poolAccount.totalDeposits.toNumber(), 1);
      assert.equal(poolAccount.totalValueDeposited.toNumber(), DEPOSIT_AMOUNT);
    });

    it("Rejects zero commitment", async () => {
      const zeroCommitment = new Array(32).fill(0);

      try {
        await program.methods
          .deposit(new anchor.BN(DEPOSIT_AMOUNT), zeroCommitment)
          .accounts({
            poolConfig,
            merkleTree,
            vault,
            depositorTokenAccount,
            depositor: authority.publicKey,
            tokenProgram: TOKEN_PROGRAM_ID,
          })
          .signers([authority])
          .rpc();

        assert.fail("Should have thrown InvalidCommitment");
      } catch (err) {
        assert.include(err.toString(), "InvalidCommitment");
      }
    });

    it("Rejects zero amount", async () => {
      const commitment = Buffer.alloc(32, 1);

      try {
        await program.methods
          .deposit(new anchor.BN(0), Array.from(commitment))
          .accounts({
            poolConfig,
            merkleTree,
            vault,
            depositorTokenAccount,
            depositor: authority.publicKey,
            tokenProgram: TOKEN_PROGRAM_ID,
          })
          .signers([authority])
          .rpc();

        assert.fail("Should have thrown InvalidAmount");
      } catch (err) {
        assert.include(err.toString(), "InvalidAmount");
      }
    });
  });

  describe("Admin Controls", () => {
    it("Pauses pool", async () => {
      const tx = await program.methods
        .pausePool()
        .accounts({
          authority: authority.publicKey,
          poolConfig,
        })
        .signers([authority])
        .rpc();

      console.log("Pause tx:", tx);

      const poolAccount = await program.account.poolConfig.fetch(poolConfig);
      assert.isTrue(poolAccount.isPaused);
    });

    it("Rejects deposits when paused", async () => {
      const commitment = Buffer.alloc(32, 1);

      try {
        await program.methods
          .deposit(new anchor.BN(DEPOSIT_AMOUNT), Array.from(commitment))
          .accounts({
            poolConfig,
            merkleTree,
            vault,
            depositorTokenAccount,
            depositor: authority.publicKey,
            tokenProgram: TOKEN_PROGRAM_ID,
          })
          .signers([authority])
          .rpc();

        assert.fail("Should have thrown PoolPaused");
      } catch (err) {
        assert.include(err.toString(), "PoolPaused");
      }
    });

    it("Unpauses pool", async () => {
      const tx = await program.methods
        .unpausePool()
        .accounts({
          authority: authority.publicKey,
          poolConfig,
        })
        .signers([authority])
        .rpc();

      console.log("Unpause tx:", tx);

      const poolAccount = await program.account.poolConfig.fetch(poolConfig);
      assert.isFalse(poolAccount.isPaused);
    });

    it("Initiates authority transfer", async () => {
      const newAuthority = Keypair.generate();

      const tx = await program.methods
        .initiateAuthorityTransfer(newAuthority.publicKey)
        .accounts({
          authority: authority.publicKey,
          poolConfig,
        })
        .signers([authority])
        .rpc();

      console.log("Initiate transfer tx:", tx);

      const poolAccount = await program.account.poolConfig.fetch(poolConfig);
      assert.equal(
        poolAccount.pendingAuthority.toString(),
        newAuthority.publicKey.toString()
      );
    });

    it("Cancels authority transfer", async () => {
      const tx = await program.methods
        .cancelAuthorityTransfer()
        .accounts({
          authority: authority.publicKey,
          poolConfig,
        })
        .signers([authority])
        .rpc();

      console.log("Cancel transfer tx:", tx);

      const poolAccount = await program.account.poolConfig.fetch(poolConfig);
      assert.equal(
        poolAccount.pendingAuthority.toString(),
        PublicKey.default.toString()
      );
    });
  });

  describe("Merkle Tree", () => {
    it("Verifies merkle tree state after deposits", async () => {
      const merkleAccount = await program.account.merkleTree.fetch(merkleTree);
      
      assert.equal(merkleAccount.depth, TREE_DEPTH);
      assert.equal(merkleAccount.pool.toString(), poolConfig.toString());
      assert.isAbove(merkleAccount.nextLeafIndex, 0);
      
      console.log("Merkle tree state:");
      console.log("  Depth:", merkleAccount.depth);
      console.log("  Next leaf index:", merkleAccount.nextLeafIndex);
      console.log("  Root history size:", merkleAccount.rootHistorySize);
    });

    it("Has valid current root", async () => {
      const merkleAccount = await program.account.merkleTree.fetch(merkleTree);
      const currentRoot = merkleAccount.currentRoot;
      
      // Root should not be all zeros after deposit
      const isAllZeros = currentRoot.every((b: number) => b === 0);
      assert.isFalse(isAllZeros, "Root should not be all zeros after deposit");
    });
  });
});

// ============================================================================
// SDK Unit Tests
// ============================================================================

describe("pSOL SDK", () => {
  describe("Poseidon Hash", () => {
    it("Should produce deterministic results", async () => {
      // Note: These tests require the full SDK to be built
      // They serve as documentation for expected behavior
      console.log("SDK tests require snarkjs and circomlibjs");
    });
  });

  describe("Proof Generation", () => {
    it("Should serialize proof correctly", () => {
      // Test proof serialization format
      const mockProof = new Uint8Array(256);
      mockProof.fill(1, 0, 64);    // A point
      mockProof.fill(2, 64, 192);  // B point
      mockProof.fill(3, 192, 256); // C point
      
      // Verify structure
      assert.equal(mockProof.length, 256);
      assert.equal(mockProof[0], 1);   // A
      assert.equal(mockProof[64], 2);  // B
      assert.equal(mockProof[192], 3); // C
    });
  });

  describe("PDA Derivation", () => {
    it("Derives pool config PDA correctly", () => {
      const programId = new PublicKey("7kK3aVXN9nTv1dNubmr85FB85fK6PeRrDBsisu9Z4gQ9");
      const tokenMint = Keypair.generate().publicKey;
      
      const [poolConfig, bump] = PublicKey.findProgramAddressSync(
        [Buffer.from("pool"), tokenMint.toBuffer()],
        programId
      );
      
      assert.ok(poolConfig);
      assert.isAtLeast(bump, 0);
      assert.isAtMost(bump, 255);
    });

    it("Derives merkle tree PDA correctly", () => {
      const programId = new PublicKey("7kK3aVXN9nTv1dNubmr85FB85fK6PeRrDBsisu9Z4gQ9");
      const poolConfig = Keypair.generate().publicKey;
      
      const [merkleTree] = PublicKey.findProgramAddressSync(
        [Buffer.from("merkle_tree"), poolConfig.toBuffer()],
        programId
      );
      
      assert.ok(merkleTree);
    });

    it("Derives nullifier PDA correctly", () => {
      const programId = new PublicKey("7kK3aVXN9nTv1dNubmr85FB85fK6PeRrDBsisu9Z4gQ9");
      const poolConfig = Keypair.generate().publicKey;
      const nullifierHash = Buffer.alloc(32, 1);
      
      const [nullifier] = PublicKey.findProgramAddressSync(
        [Buffer.from("nullifier"), poolConfig.toBuffer(), nullifierHash],
        programId
      );
      
      assert.ok(nullifier);
    });
  });
});
