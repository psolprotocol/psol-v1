use anchor_lang::prelude::*;
use solana_program::keccak;

/// Merkle tree state account
#[account]
pub struct MerkleTree {
    /// Reference to parent pool
    pub pool: Pubkey,

    /// Tree depth
    pub depth: u8,

    /// Current leaf count
    pub next_leaf_index: u32,

    /// Current root
    pub current_root: [u8; 32],

    /// Root history (circular buffer)
    pub root_history: Vec<[u8; 32]>,

    /// Current position in root history buffer
    pub root_history_index: u16,

    /// Filled subtrees for incremental updates
    /// Length = depth + 1
    pub filled_subtrees: Vec<[u8; 32]>,
}

impl MerkleTree {
    /// Calculate space needed for merkle tree account
    ///
    /// Important: this must match exactly what gets serialized:
    /// - Vec<T> is encoded as: 4 bytes (len) + len * size_of(T)
    /// - filled_subtrees has length depth + 1
    pub fn space(depth: u8, root_history_size: u16) -> usize {
        let depth_usize = depth as usize;
        let history_usize = root_history_size as usize;

        8   // discriminator
        + 32 // pool
        + 1  // depth
        + 4  // next_leaf_index
        + 32 // current_root
        + 4  // root_history vec length prefix
        + 32 * history_usize // root_history entries
        + 2  // root_history_index
        + 4  // filled_subtrees vec length prefix
        + 32 * (depth_usize + 1) // filled_subtrees entries (depth + 1)
    }

    /// Initialize merkle tree with zero values
    pub fn initialize(&mut self, pool: Pubkey, depth: u8, root_history_size: u16) -> Result<()> {
        require!(
            depth > 0 && depth <= 32,
            crate::error::PrivacyError::InvalidTreeDepth
        );

        self.pool = pool;
        self.depth = depth;
        self.next_leaf_index = 0;
        self.root_history_index = 0;

        // Initialize root history buffer
        self.root_history = vec![[0u8; 32]; root_history_size as usize];

        // Compute zero values for each level (0..=depth)
        let zeros = Self::compute_zero_values(depth);

        // Initialize filled subtrees with zeros
        // Length will be depth + 1
        self.filled_subtrees = zeros.clone();

        // Set initial root (root of empty tree)
        self.current_root = zeros[depth as usize];

        // Store initial root in history
        self.root_history[0] = self.current_root;

        Ok(())
    }

    /// Compute zero hash values for each tree level
    /// Level 0 = leaf level, Level depth = root
    fn compute_zero_values(depth: u8) -> Vec<[u8; 32]> {
        let mut zeros = Vec::with_capacity((depth + 1) as usize);

        // Level 0: zero leaf
        zeros.push([0u8; 32]);

        // Compute zero hash for each level
        for i in 1..=depth {
            let prev = zeros[(i - 1) as usize];
            zeros.push(Self::hash_pair(&prev, &prev));
        }

        zeros
    }

    /// Hash two values together (placeholder - will be replaced with Poseidon)
    /// For now using Keccak256, structure allows swapping to Poseidon later
    fn hash_pair(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
        let mut combined = [0u8; 64];
        combined[..32].copy_from_slice(left);
        combined[32..].copy_from_slice(right);
        keccak::hash(&combined).to_bytes()
    }

    /// Insert a new leaf (commitment) into the tree
    pub fn insert_leaf(&mut self, commitment: [u8; 32]) -> Result<u32> {
        // Check tree not full
        let max_leaves = 1u32 << self.depth;
        require!(
            self.next_leaf_index < max_leaves,
            crate::error::PrivacyError::MerkleTreeFull
        );

        let leaf_index = self.next_leaf_index;
        let mut current_hash = commitment;
        let mut current_index = leaf_index;

        // Pre-compute all zero values ONCE
        let zeros = Self::compute_zero_values(self.depth);

        // Update filled subtrees and compute new root
        for level in 0..self.depth {
            let is_right_node = (current_index & 1) == 1;
            current_index >>= 1;

            if !is_right_node {
                // Left node - update filled subtree for this level
                self.filled_subtrees[level as usize] = current_hash;

                // Hash with zero for this level
                let zero = zeros[level as usize];
                current_hash = Self::hash_pair(&current_hash, &zero);
            } else {
                // Right node - hash with existing left sibling
                let left_sibling = self.filled_subtrees[level as usize];
                current_hash = Self::hash_pair(&left_sibling, &current_hash);
            }
        }

        // Update root
        self.current_root = current_hash;

        // Add to root history (circular buffer)
        let history_len = self.root_history.len() as u16;
        if history_len > 0 {
            self.root_history_index = (self.root_history_index + 1) % history_len;
            self.root_history[self.root_history_index as usize] = current_hash;
        }

        // Increment leaf counter
        self.next_leaf_index = self
            .next_leaf_index
            .checked_add(1)
            .ok_or(error!(crate::error::PrivacyError::ArithmeticOverflow))?;

        Ok(leaf_index)
    }

    /// Check if a root is in recent history
    pub fn is_known_root(&self, root: &[u8; 32]) -> bool {
        self.root_history.iter().any(|r| r == root)
    }

    /// Get current root
    pub fn get_current_root(&self) -> [u8; 32] {
        self.current_root
    }
}
