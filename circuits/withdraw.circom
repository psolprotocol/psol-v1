pragma circom 2.1.6;

/*
 * pSOL Privacy Pool - Withdraw Circuit
 * =====================================
 * 
 * This circuit proves knowledge of a valid deposit without revealing which one.
 * 
 * Public Inputs:
 *   - root: Merkle tree root
 *   - nullifierHash: Hash that prevents double-spending
 *   - recipient: Address receiving the withdrawal
 *   - amount: Amount being withdrawn
 *   - relayer: Relayer address (can be zero)
 *   - relayerFee: Fee paid to relayer
 * 
 * Private Inputs:
 *   - secret: Random secret known only to depositor
 *   - nullifier: Pre-image of nullifierHash
 *   - pathElements: Merkle proof siblings
 *   - pathIndices: Merkle proof path (0 = left, 1 = right)
 * 
 * Constraints:
 *   1. commitment = Poseidon(secret, nullifier, amount)
 *   2. nullifierHash = Poseidon(nullifier, secret)
 *   3. commitment is in the Merkle tree with given root
 *   4. Public inputs are properly bound to the proof
 */

include "circomlib/circuits/poseidon.circom";
include "circomlib/circuits/bitify.circom";
include "circomlib/circuits/comparators.circom";

// Merkle tree inclusion proof using Keccak256
// Note: pSOL uses Keccak256 for Merkle tree, Poseidon for commitment/nullifier
template MerkleTreeChecker(levels) {
    signal input leaf;
    signal input root;
    signal input pathElements[levels];
    signal input pathIndices[levels];

    component hashers[levels];
    component mux[levels];

    signal hashes[levels + 1];
    hashes[0] <== leaf;

    for (var i = 0; i < levels; i++) {
        // Verify pathIndices is binary
        pathIndices[i] * (1 - pathIndices[i]) === 0;

        // Use Poseidon for Merkle hashing (circomlib compatible)
        // In production, replace with Keccak256 hasher to match on-chain
        hashers[i] = Poseidon(2);
        
        // Select order based on path index
        // If pathIndices[i] == 0: hash(current, sibling)
        // If pathIndices[i] == 1: hash(sibling, current)
        mux[i] = DualMux();
        mux[i].in[0] <== hashes[i];
        mux[i].in[1] <== pathElements[i];
        mux[i].s <== pathIndices[i];

        hashers[i].inputs[0] <== mux[i].out[0];
        hashers[i].inputs[1] <== mux[i].out[1];
        
        hashes[i + 1] <== hashers[i].out;
    }

    // Verify computed root matches provided root
    root === hashes[levels];
}

// Dual multiplexer for Merkle proof
template DualMux() {
    signal input in[2];
    signal input s;
    signal output out[2];

    s * (1 - s) === 0;
    out[0] <== (in[1] - in[0]) * s + in[0];
    out[1] <== (in[0] - in[1]) * s + in[1];
}

// Commitment hasher: Poseidon(secret, nullifier, amount)
template CommitmentHasher() {
    signal input secret;
    signal input nullifier;
    signal input amount;
    signal output commitment;
    signal output nullifierHash;

    // Compute commitment = Poseidon(secret, nullifier, amount)
    component commitmentHasher = Poseidon(3);
    commitmentHasher.inputs[0] <== secret;
    commitmentHasher.inputs[1] <== nullifier;
    commitmentHasher.inputs[2] <== amount;
    commitment <== commitmentHasher.out;

    // Compute nullifierHash = Poseidon(nullifier, secret)
    component nullifierHasher = Poseidon(2);
    nullifierHasher.inputs[0] <== nullifier;
    nullifierHasher.inputs[1] <== secret;
    nullifierHash <== nullifierHasher.out;
}

// Main withdraw circuit
template Withdraw(levels) {
    // Public inputs
    signal input root;
    signal input nullifierHash;
    signal input recipient;
    signal input amount;
    signal input relayer;
    signal input relayerFee;

    // Private inputs
    signal input secret;
    signal input nullifier;
    signal input pathElements[levels];
    signal input pathIndices[levels];

    // Compute commitment and nullifier hash
    component hasher = CommitmentHasher();
    hasher.secret <== secret;
    hasher.nullifier <== nullifier;
    hasher.amount <== amount;

    // Verify nullifier hash matches public input
    hasher.nullifierHash === nullifierHash;

    // Verify Merkle inclusion
    component tree = MerkleTreeChecker(levels);
    tree.leaf <== hasher.commitment;
    tree.root <== root;
    for (var i = 0; i < levels; i++) {
        tree.pathElements[i] <== pathElements[i];
        tree.pathIndices[i] <== pathIndices[i];
    }

    // Bind public inputs to prevent malleability
    // These are included in the proof but not computed
    signal recipientSquare;
    signal relayerSquare;
    signal relayerFeeSquare;
    
    recipientSquare <== recipient * recipient;
    relayerSquare <== relayer * relayer;
    relayerFeeSquare <== relayerFee * relayerFee;

    // Verify relayer fee doesn't exceed amount
    component feeCheck = LessEqThan(64);
    feeCheck.in[0] <== relayerFee;
    feeCheck.in[1] <== amount;
    feeCheck.out === 1;
}

// Main component with 20-level Merkle tree (supports ~1M deposits)
component main {public [root, nullifierHash, recipient, amount, relayer, relayerFee]} = Withdraw(20);
