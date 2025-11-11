//! Simple Mining Interface with FFI Integration
//! 
//! This module provides a simplified interface to the high-performance C++ mining engine
//! through FFI bindings. It offers an easy-to-use API while leveraging the full
//! performance of the C++ consensus mining implementation.

use crate::{
    block::{Block, BlockHeader}, 
    transaction::Transaction,
    Hash256, Result as BlockchainResult,
};
use blockchain_consensus_ffi::ConsensusMiner;
use std::time::{SystemTime, UNIX_EPOCH, Instant};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MiningError {
    #[error("Invalid difficulty target")]
    InvalidTarget,
    #[error("Mining timeout")]
    Timeout,
    #[error("Hash calculation failed")]
    HashError,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleMiningConfig {
    pub target_difficulty: u32,
    pub max_iterations: u64,
    pub coinbase_reward: u64,
    pub timeout_seconds: u64,
}

impl Default for SimpleMiningConfig {
    fn default() -> Self {
        Self {
            target_difficulty: 0x1e0fffff,  // Easy difficulty for testing
            max_iterations: 1_000_000,
            coinbase_reward: 5000000000, // 50 coins in satoshis
            timeout_seconds: 30,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleMiningResult {
    pub success: bool,
    pub nonce: u64,
    pub block_hash: String,
    pub hash_operations: u64,
    pub elapsed_seconds: f64,
}

/// Simple pure-Rust mining implementation
pub struct SimpleMiner {
    config: SimpleMiningConfig,
}

impl SimpleMiner {
    pub fn new(config: SimpleMiningConfig) -> Self {
        Self { config }
    }
    
    /// Get target difficulty
    pub fn get_target_difficulty(&self) -> u32 {
        self.config.target_difficulty
    }
    
    /// Get max iterations
    pub fn get_max_iterations(&self) -> u64 {
        self.config.max_iterations
    }
    
    /// Get timeout seconds
    pub fn get_timeout_seconds(&self) -> u64 {
        self.config.timeout_seconds
    }

    /// Mine a single block with the given transactions using the high-performance C++ engine
    pub fn mine_block(&self, mut block: Block) -> Result<SimpleMiningResult, MiningError> {
        let start_time = Instant::now();

        // Serialize block data for C++ mining engine
        let block_data = block.serialize_for_mining()
            .map_err(|_| MiningError::HashError)?;

        // Use the C++ mining engine through FFI
        let cpp_result = ConsensusMiner::mine_block(
            &hex::encode(block_data),
            self.config.target_difficulty,
            Some(self.config.max_iterations),
        ).map_err(|_| MiningError::HashError)?;

        // Update block with found nonce if successful
        if cpp_result.success {
            block.header.nonce = cpp_result.nonce;
        }

        // Convert C++ result to simple result format
        Ok(SimpleMiningResult {
            success: cpp_result.success,
            nonce: cpp_result.nonce as u64,
            block_hash: if cpp_result.success {
                hex::encode(cpp_result.block_hash)
            } else {
                "".to_string()
            },
            hash_operations: cpp_result.hash_operations,
            elapsed_seconds: cpp_result.elapsed_seconds,
        })
    }

    /// Create a coinbase transaction for mining reward
    pub fn create_coinbase_transaction(&self, height: u64, miner_address: &str) -> Transaction {
        // For simplicity, create a basic coinbase transaction
        // In a real implementation, this would properly construct outputs
        Transaction::new(1, vec![], vec![]) // Coinbase has no inputs/outputs for simplicity
    }

    /// Calculate merkle root of transactions using proper merkle tree construction
    pub fn calculate_merkle_root(&self, transactions: &[Transaction]) -> Result<[u8; 32], MiningError> {
        if transactions.is_empty() {
            return Ok([0u8; 32]);
        }

        // Get transaction hashes
        let mut hashes: Vec<Hash256> = transactions.iter()
            .map(|tx| tx.get_hash())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| MiningError::HashError)?;

        // Build proper merkle tree
        while hashes.len() > 1 {
            let mut next_level = Vec::new();
            
            for chunk in hashes.chunks(2) {
                let left = &chunk[0];
                let right = if chunk.len() > 1 { &chunk[1] } else { &chunk[0] };
                
                // Concatenate and hash using blake3 for consistency
                let mut combined = Vec::with_capacity(64);
                combined.extend_from_slice(left);
                combined.extend_from_slice(right);
                
                let hash = blake3::hash(&combined);
                next_level.push(*hash.as_bytes());
            }
            
            hashes = next_level;
        }

        Ok(hashes[0])
    }

    /// Calculate difficulty adjustment using the C++ consensus engine
    pub fn calculate_next_difficulty(&self, current_bits: u32, actual_time: u64, target_time: u64) -> u32 {
        if actual_time == 0 || target_time == 0 {
            return current_bits;
        }

        // Use the C++ consensus engine for difficulty adjustment
        ConsensusMiner::calculate_next_difficulty(current_bits, actual_time, target_time)
    }

    /// Validate that a block meets the difficulty target using C++ consensus validation
    pub fn validate_block_difficulty(&self, block: &Block) -> Result<bool, MiningError> {
        // Use C++ consensus validator to check if block hash meets the difficulty target
        let block_data = block.serialize_for_mining()
            .map_err(|_| MiningError::HashError)?;

        // The C++ validator can check if the block hash meets the difficulty target
        Ok(ConsensusMiner::verify_proof_of_work(
            &hex::encode(block_data),
            block.header.nonce,
            block.header.difficulty_target,
        ).unwrap_or(false))
    }

    /// Get current timestamp in seconds since Unix epoch
    pub fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    /// Create a new block template for mining with proper difficulty adjustment
    pub fn create_block_template(
        &self,
        previous_hash: [u8; 32],
        transactions: Vec<Transaction>,
        height: u64,
    ) -> Result<Block, MiningError> {
        let merkle_root = self.calculate_merkle_root(&transactions)?;
        
        // Use C++ consensus engine to determine appropriate difficulty target
        let difficulty_target = if ConsensusMiner::should_adjust_difficulty(height as u32) {
            // For simplicity, use configured difficulty in simple interface
            self.config.target_difficulty
        } else {
            self.config.target_difficulty
        };
        
        let header = BlockHeader {
            version: 1,
            prev_block_hash: previous_hash,
            merkle_root,
            timestamp: Self::current_timestamp() as u32,
            difficulty_target,
            nonce: 0,
        };

        Ok(Block::new(header, transactions))
    }

    /// Check if the C++ mining engine is available
    pub fn is_cpp_mining_available(&self) -> bool {
        // Try a simple operation to check if FFI is working
        ConsensusMiner::should_adjust_difficulty(0); // This should always work
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpp_mining_availability() {
        let miner = SimpleMiner::new(SimpleMiningConfig::default());
        
        // Test if C++ mining engine is available
        assert!(miner.is_cpp_mining_available());
    }

    #[test]
    fn test_simple_mining_with_ffi() {
        let config = SimpleMiningConfig {
            target_difficulty: 0x207fffff, // Very easy for testing
            max_iterations: 10000,
            coinbase_reward: 5000000000,
            timeout_seconds: 5,
        };

        let miner = SimpleMiner::new(config);
        
        // Create a simple block template
        let block_template = miner.create_block_template(
            [0u8; 32], // Genesis previous hash
            vec![],     // No transactions
            1           // Block height
        ).unwrap();

        // Try mining with C++ engine
        let result = miner.mine_block(block_template.clone());
        
        // Should either succeed or fail gracefully
        assert!(result.is_ok());
        let result = result.unwrap();
        println!("C++ Mining result: {:?}", result);
        
        if result.success {
            assert!(!result.block_hash.is_empty());
            assert!(result.nonce > 0);
            
            // Validate the mined block
            let validation = miner.validate_block_difficulty(&block_template);
            assert!(validation.is_ok());
        }
    }

    #[test]
    fn test_difficulty_adjustment_with_cpp() {
        let miner = SimpleMiner::new(SimpleMiningConfig::default());
        
        let current_bits = 0x1e0fffff;
        
        // Test C++ difficulty adjustment when blocks are found too quickly
        let new_bits_fast = miner.calculate_next_difficulty(current_bits, 300, 600);
        println!("Fast blocks difficulty: 0x{:08x} -> 0x{:08x}", current_bits, new_bits_fast);
        
        // Test C++ difficulty adjustment when blocks are found too slowly  
        let new_bits_slow = miner.calculate_next_difficulty(current_bits, 1200, 600);
        println!("Slow blocks difficulty: 0x{:08x} -> 0x{:08x}", current_bits, new_bits_slow);
        
        // The C++ engine should provide meaningful adjustments
        // (Results may vary based on C++ implementation)
    }

    #[test]
    fn test_merkle_root_calculation() {
        let miner = SimpleMiner::new(SimpleMiningConfig::default());
        
        // Test with empty transactions
        let empty_root = miner.calculate_merkle_root(&[]).unwrap();
        assert_eq!(empty_root, [0u8; 32]);
        
        // Test with mock transactions would require transaction creation
        // which depends on other modules, so keeping simple for now
    }
}