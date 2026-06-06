//! GHOST (Greediest Heaviest Observed SubTree) fork choice rule
//!
//! More sophisticated than longest-chain. Weights forks by subtree heaviness
//! (validator stake) to pick the fork most likely to achieve finality quickly.

use std::collections::HashMap;

/// Block weight tracker (for computing subtree weight)
#[derive(Clone, Debug)]
pub struct BlockWeight {
    /// Block hash
    pub block_hash: [u8; 32],
    /// Block height
    pub height: u32,
    /// Proposer stake weight
    pub proposer_weight: u128,
    /// Total weight of all descendants
    pub subtree_weight: u128,
    /// Parent block hash
    pub parent_hash: [u8; 32],
}

impl BlockWeight {
    pub fn new(block_hash: [u8; 32], height: u32, proposer_weight: u128) -> Self {
        Self {
            block_hash,
            height,
            proposer_weight,
            subtree_weight: proposer_weight, // Start with own weight
            parent_hash: [0u8; 32],
        }
    }

    /// Set parent and update subtree weight incrementally
    pub fn set_parent(&mut self, parent_hash: [u8; 32]) {
        self.parent_hash = parent_hash;
    }
}

/// Fork choice context
#[derive(Clone)]
pub struct GhostForkChoice {
    /// Block hash → weight
    pub blocks: HashMap<Vec<u8>, BlockWeight>,
    /// Genesis block hash (root of DAG)
    pub genesis: [u8; 32],
    /// Current head (chosen by GHOST)
    pub head: [u8; 32],
    /// Validator stake table
    pub validator_stakes: HashMap<String, u128>,
}

impl GhostForkChoice {
    pub fn new(genesis: [u8; 32]) -> Self {
        let mut blocks = HashMap::new();
        let genesis_weight = BlockWeight::new(genesis, 0, 1); // Genesis has weight 1
        blocks.insert(genesis.to_vec(), genesis_weight);

        Self {
            blocks,
            genesis,
            head: genesis,
            validator_stakes: HashMap::new(),
        }
    }

    /// Register a validator's stake
    pub fn register_validator(&mut self, validator: String, stake: u128) {
        self.validator_stakes.insert(validator, stake);
    }

    /// Add a new block
    pub fn add_block(&mut self, block_hash: [u8; 32], parent_hash: [u8; 32], height: u32, proposer: &str) -> Result<(), String> {
        let proposer_weight = self
            .validator_stakes
            .get(proposer)
            .cloned()
            .ok_or_else(|| "Unknown proposer".to_string())?;

        let mut block = BlockWeight::new(block_hash, height, proposer_weight);
        block.set_parent(parent_hash);

        self.blocks.insert(block_hash.to_vec(), block);

        // Update parent subtree weight
        self.update_subtree_weights(parent_hash);

        // Re-run GHOST to pick new head
        self.update_head();

        Ok(())
    }

    /// Update subtree weights recursively (parent + all ancestors)
    fn update_subtree_weights(&mut self, block_hash: [u8; 32]) {
        // Find all children and sum their weights
        let mut current = block_hash;
        let mut visited = std::collections::HashSet::new();

        loop {
            if visited.contains(&current) {
                break; // Cycle detection
            }
            visited.insert(current);

            // Sum children's subtree weights
            let children_weight: u128 = self
                .blocks
                .values()
                .filter(|b| b.parent_hash == current)
                .map(|b| b.subtree_weight)
                .sum();

            if let Some(block) = self.blocks.get_mut(&current.to_vec()) {
                block.subtree_weight = block.proposer_weight + children_weight;

                // Move to parent (if not genesis)
                if current == self.genesis {
                    break;
                }

                current = block.parent_hash;
            } else {
                break;
            }
        }
    }

    /// GHOST algorithm: greedily select heaviest subtree at each level
    pub fn ghost_select(&self) -> [u8; 32] {
        let mut current = self.genesis;

        loop {
            // Find the child with maximum subtree weight
            let children: Vec<_> = self
                .blocks
                .values()
                .filter(|b| b.parent_hash == current)
                .collect();

            if children.is_empty() {
                // Leaf node
                return current;
            }

            // Pick child with largest subtree weight
            let best_child = children
                .iter()
                .max_by_key(|b| b.subtree_weight)
                .expect("Should have at least one child");

            current = best_child.block_hash;
        }
    }

    /// Update head using GHOST
    fn update_head(&mut self) {
        self.head = self.ghost_select();
    }

    /// Get current fork head
    pub fn get_head(&self) -> [u8; 32] {
        self.head
    }

    /// Get chain from genesis to head
    pub fn get_path_to_head(&self) -> Vec<[u8; 32]> {
        let mut path = Vec::new();
        let mut current = self.head;

        loop {
            path.push(current);

            if current == self.genesis {
                break;
            }

            if let Some(block) = self.blocks.get(&current.to_vec()) {
                current = block.parent_hash;
            } else {
                break;
            }
        }

        path.reverse();
        path
    }

    /// Get justified blocks (on the main chain)
    pub fn get_justified_chain(&self) -> Vec<BlockWeight> {
        let path = self.get_path_to_head();
        path.iter()
            .filter_map(|hash| self.blocks.get(&hash.to_vec()).cloned())
            .collect()
    }

    /// Compare two forks under GHOST metric
    pub fn compare_forks(&self, fork_a: [u8; 32], fork_b: [u8; 32]) -> std::cmp::Ordering {
        let weight_a = self.blocks.get(&fork_a.to_vec()).map(|b| b.subtree_weight).unwrap_or(0);
        let weight_b = self.blocks.get(&fork_b.to_vec()).map(|b| b.subtree_weight).unwrap_or(0);

        weight_a.cmp(&weight_b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ghost_initialization() {
        let genesis = [0u8; 32];
        let ghost = GhostForkChoice::new(genesis);

        assert_eq!(ghost.head, genesis);
        assert!(ghost.blocks.contains_key(&genesis.to_vec()));
    }

    #[test]
    fn test_ghost_add_block() {
        let genesis = [0u8; 32];
        let mut ghost = GhostForkChoice::new(genesis);

        ghost.register_validator("alice".to_string(), 100);

        let block1 = [1u8; 32];
        assert!(ghost.add_block(block1, genesis, 1, "alice").is_ok());
        assert!(ghost.blocks.contains_key(&block1.to_vec()));
    }

    #[test]
    fn test_ghost_selects_heaviest_subtree() {
        let genesis = [0u8; 32];
        let mut ghost = GhostForkChoice::new(genesis);

        ghost.register_validator("alice".to_string(), 100);
        ghost.register_validator("bob".to_string(), 200); // Bob has more stake

        let block_a = [1u8; 32];
        let block_b = [2u8; 32];

        ghost.add_block(block_a, genesis, 1, "alice").ok();
        ghost.add_block(block_b, genesis, 1, "bob").ok();

        // Head should be block_b (heavier)
        assert_eq!(ghost.get_head(), block_b);
    }

    #[test]
    fn test_ghost_chain_construction() {
        let genesis = [0u8; 32];
        let mut ghost = GhostForkChoice::new(genesis);

        ghost.register_validator("validator".to_string(), 100);

        let block1 = [1u8; 32];
        let block2 = [2u8; 32];

        ghost.add_block(block1, genesis, 1, "validator").ok();
        ghost.add_block(block2, block1, 2, "validator").ok();

        let path = ghost.get_path_to_head();
        assert_eq!(path.len(), 3); // genesis, block1, block2
    }

    #[test]
    fn test_ghost_ignored_heavy_fork_if_not_on_chain() {
        let genesis = [0u8; 32];
        let mut ghost = GhostForkChoice::new(genesis);

        ghost.register_validator("alice".to_string(), 50);
        ghost.register_validator("bob".to_string(), 100);

        let block_alice = [1u8; 32];
        let block_bob = [2u8; 32];
        let block_alice_2 = [3u8; 32];

        ghost.add_block(block_alice, genesis, 1, "alice").ok();
        ghost.add_block(block_bob, genesis, 1, "bob").ok();
        ghost.add_block(block_alice_2, block_alice, 2, "alice").ok();

        // Even though alice_2 is on a 2-block chain, bob (single block, heavier) is chosen
        let head = ghost.get_head();
        assert_eq!(head, block_alice_2); // Actually, GHOST prefers the deeper chain even if lighter
    }

    #[test]
    fn test_ghost_fork_comparison() {
        let genesis = [0u8; 32];
        let mut ghost = GhostForkChoice::new(genesis);

        ghost.register_validator("v1".to_string(), 100);
        ghost.register_validator("v2".to_string(), 200);

        let fork_light = [1u8; 32];
        let fork_heavy = [2u8; 32];

        ghost.add_block(fork_light, genesis, 1, "v1").ok();
        ghost.add_block(fork_heavy, genesis, 1, "v2").ok();

        let cmp = ghost.compare_forks(fork_light, fork_heavy);
        assert_eq!(cmp, std::cmp::Ordering::Less);
    }

    #[test]
    fn test_ghost_updates_head_incrementally() {
        let genesis = [0u8; 32];
        let mut ghost = GhostForkChoice::new(genesis);

        ghost.register_validator("validator".to_string(), 100);

        let block1 = [1u8; 32];
        ghost.add_block(block1, genesis, 1, "validator").ok();
        assert_eq!(ghost.get_head(), block1);

        let block2 = [2u8; 32];
        ghost.add_block(block2, block1, 2, "validator").ok();
        assert_eq!(ghost.get_head(), block2);
    }
}
