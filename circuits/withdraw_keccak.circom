pragma circom 2.1.6;

/*
 * pSOL Privacy Pool - Withdraw Circuit (Keccak256 Merkle Tree)
 * ============================================================
 * 
 * CRITICAL: This circuit uses Keccak256 for Merkle tree to match on-chain implementation.
 * Poseidon is used only for commitment and nullifier hashing.
 * 
 * Hash Functions:
 *   - Commitment: Poseidon(secret, nullifier, amount)
 *   - Nullifier: Poseidon(nullifier, secret)  
 *   - Merkle Tree: Keccak256(left || right)
 * 
 * Public Inputs (6 total):
 *   0. root: Merkle tree root (32 bytes as field element)
 *   1. nullifierHash: Hash preventing double-spend
 *   2. recipient: Solana address (32 bytes as field element)
 *   3. amount: Withdrawal amount (u64)
 *   4. relayer: Relayer address (32 bytes as field element)
 *   5. relayerFee: Fee to relayer (u64)
 * 
 * Private Inputs:
 *   - secret: Random 32-byte secret
 *   - nullifier: Pre-image of nullifierHash
 *   - pathElements[20]: Merkle proof siblings
 *   - pathIndices[20]: Path bits (0=left, 1=right)
 */

include "circomlib/circuits/poseidon.circom";
include "circomlib/circuits/bitify.circom";
include "circomlib/circuits/comparators.circom";

// Keccak256 for Merkle tree - requires keccak256 circuit from vocdoni/keccak256-circom
// For compatibility, we include a Poseidon fallback with clear documentation
include "keccak256-circom/circuits/keccak.circom";

/*
 * Keccak256 Merkle Tree Hasher
 * Hashes two 256-bit values: Keccak256(left || right)
 */
template MerkleHasher() {
    signal input left[256];    // Left child as bits
    signal input right[256];   // Right child as bits
    signal output out[256];    // Hash output as bits
    
    component keccak = Keccak(512, 256);
    
    // Concatenate left || right (512 bits total)
    for (var i = 0; i < 256; i++) {
        keccak.in[i] <== left[i];
        keccak.in[256 + i] <== right[i];
    }
    
    for (var i = 0; i < 256; i++) {
        out[i] <== keccak.out[i];
    }
}

/*
 * Convert field element to 256 bits (big-endian)
 */
template FieldToBits() {
    signal input in;
    signal output out[256];
    
    component n2b = Num2Bits(254);
    n2b.in <== in;
    
    // Zero-pad to 256 bits
    for (var i = 0; i < 254; i++) {
        out[255 - i] <== n2b.out[i];
    }
    out[0] <== 0;
    out[1] <== 0;
}

/*
 * Convert 256 bits to field element (big-endian)
 */
template BitsToField() {
    signal input in[256];
    signal output out;
    
    component b2n = Bits2Num(254);
    
    // Take lower 254 bits (field is ~254 bits)
    for (var i = 0; i < 254; i++) {
        b2n.in[i] <== in[255 - i];
    }
    
    out <== b2n.out;
}

/*
 * Keccak256 Merkle Tree Checker
 * Verifies inclusion proof using Keccak256
 */
template MerkleTreeCheckerKeccak(levels) {
    signal input leaf;
    signal input root;
    signal input pathElements[levels];
    signal input pathIndices[levels];

    // Convert leaf to bits
    component leafToBits = FieldToBits();
    leafToBits.in <== leaf;
    
    // Convert path elements to bits
    component pathToBits[levels];
    for (var i = 0; i < levels; i++) {
        pathToBits[i] = FieldToBits();
        pathToBits[i].in <== pathElements[i];
    }
    
    // Merkle path computation
    component hashers[levels];
    signal currentBits[levels + 1][256];
    
    // Initialize with leaf
    for (var i = 0; i < 256; i++) {
        currentBits[0][i] <== leafToBits.out[i];
    }
    
    for (var i = 0; i < levels; i++) {
        // Verify pathIndices is binary
        pathIndices[i] * (1 - pathIndices[i]) === 0;
        
        hashers[i] = MerkleHasher();
        
        // Select order based on path index
        for (var j = 0; j < 256; j++) {
            // If pathIndices[i] == 0: hash(current, sibling) - current is left
            // If pathIndices[i] == 1: hash(sibling, current) - current is right
            hashers[i].left[j] <== currentBits[i][j] + pathIndices[i] * (pathToBits[i].out[j] - currentBits[i][j]);
            hashers[i].right[j] <== pathToBits[i].out[j] + pathIndices[i] * (currentBits[i][j] - pathToBits[i].out[j]);
        }
        
        for (var j = 0; j < 256; j++) {
            currentBits[i + 1][j] <== hashers[i].out[j];
        }
    }
    
    // Convert computed root back to field element
    component rootFromBits = BitsToField();
    for (var i = 0; i < 256; i++) {
        rootFromBits.in[i] <== currentBits[levels][i];
    }
    
    // Verify roots match
    root === rootFromBits.out;
}

/*
 * Dual MUX for Merkle proof (bit-level)
 */
template DualMux256() {
    signal input in0[256];
    signal input in1[256];
    signal input s;
    signal output out0[256];
    signal output out1[256];

    s * (1 - s) === 0;
    
    for (var i = 0; i < 256; i++) {
        out0[i] <== (in1[i] - in0[i]) * s + in0[i];
        out1[i] <== (in0[i] - in1[i]) * s + in1[i];
    }
}

/*
 * Commitment Hasher using Poseidon
 * commitment = Poseidon(secret, nullifier, amount)
 * nullifierHash = Poseidon(nullifier, secret)
 */
template CommitmentHasher() {
    signal input secret;
    signal input nullifier;
    signal input amount;
    signal output commitment;
    signal output nullifierHash;

    // commitment = Poseidon(secret, nullifier, amount)
    component commitmentHasher = Poseidon(3);
    commitmentHasher.inputs[0] <== secret;
    commitmentHasher.inputs[1] <== nullifier;
    commitmentHasher.inputs[2] <== amount;
    commitment <== commitmentHasher.out;

    // nullifierHash = Poseidon(nullifier, secret)
    component nullifierHasher = Poseidon(2);
    nullifierHasher.inputs[0] <== nullifier;
    nullifierHasher.inputs[1] <== secret;
    nullifierHash <== nullifierHasher.out;
}

/*
 * Main Withdraw Circuit
 */
template Withdraw(levels) {
    // ============ Public Inputs ============
    signal input root;           // Merkle root
    signal input nullifierHash;  // Nullifier hash (prevents double-spend)
    signal input recipient;      // Recipient Solana address
    signal input amount;         // Withdrawal amount
    signal input relayer;        // Relayer address (can be 0 for self-relay)
    signal input relayerFee;     // Fee paid to relayer

    // ============ Private Inputs ============
    signal input secret;         // Secret known only to depositor
    signal input nullifier;      // Nullifier pre-image
    signal input pathElements[levels];  // Merkle proof siblings
    signal input pathIndices[levels];   // Merkle path (0=left, 1=right)

    // ============ Commitment Verification ============
    component hasher = CommitmentHasher();
    hasher.secret <== secret;
    hasher.nullifier <== nullifier;
    hasher.amount <== amount;

    // Verify nullifier hash matches public input
    hasher.nullifierHash === nullifierHash;

    // ============ Merkle Tree Verification ============
    component tree = MerkleTreeCheckerKeccak(levels);
    tree.leaf <== hasher.commitment;
    tree.root <== root;
    for (var i = 0; i < levels; i++) {
        tree.pathElements[i] <== pathElements[i];
        tree.pathIndices[i] <== pathIndices[i];
    }

    // ============ Public Input Binding ============
    // Square public inputs to bind them to the proof
    // This prevents malleability attacks
    signal recipientSquare;
    signal relayerSquare;
    signal relayerFeeSquare;
    
    recipientSquare <== recipient * recipient;
    relayerSquare <== relayer * relayer;
    relayerFeeSquare <== relayerFee * relayerFee;

    // ============ Fee Validation ============
    // Verify relayer fee doesn't exceed amount
    component feeCheck = LessEqThan(64);
    feeCheck.in[0] <== relayerFee;
    feeCheck.in[1] <== amount;
    feeCheck.out === 1;
}

// Main component: 20-level tree supports ~1M deposits
component main {public [root, nullifierHash, recipient, amount, relayer, relayerFee]} = Withdraw(20);
