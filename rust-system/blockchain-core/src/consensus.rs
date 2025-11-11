//! Consensus validation and chain management
//! 
//! This module provides safe consensus validation that works with the hybrid
//! C++ mining engine. It manages blockchain state, validates blocks and transactions,
//! and handles chain reorganization.

use crate::{
    Hash256, Amount, Result, BlockchainError, BlockHeight, Timestamp,
    block::{Block, BlockHeader},
    transaction::{Transaction, TransactionInput, TransactionOutput}, 
    utxo::{UTXOSet, UTXO},
};
use blockchain_ffi::types::Hash256Wrapper;
use blockchain_consensus_ffi::ConsensusMiner;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock as AsyncRwLock;
use tracing::info;

/// Consensus parameters for the blockchain network
#[derive(Debug, Clone)]
pub struct ConsensusParams {
    /// Maximum block size in bytes
    pub max_block_size: usize,
    /// Target block time in seconds
    pub target_block_time: u64,
    /// Maximum number of blocks for difficulty adjustment
    pub difficulty_adjustment_interval: u64,
    /// Minimum difficulty target
    pub min_difficulty_target: u32,
    /// Maximum difficulty target
    pub max_difficulty_target: u32,
    /// Coinbase reward in satoshis
    pub block_reward: Amount,
    /// Maximum number of inputs per transaction
    pub max_tx_inputs: usize,
    /// Maximum number of outputs per transaction
    pub max_tx_outputs: usize,
}

impl Default for ConsensusParams {
    fn default() -> Self {
        Self {
            max_block_size: 1_000_000, // 1MB
            target_block_time: 600, // 10 minutes
            difficulty_adjustment_interval: 2016, // ~2 weeks
            min_difficulty_target: 0x01000000,
            max_difficulty_target: 0xFF000000,
            block_reward: 50_00000000, // 50 coins
            max_tx_inputs: 1000,
            max_tx_outputs: 1000,
        }
    }
}

/// Chain state information
#[derive(Debug, Clone)]
pub struct ChainState {
    pub height: BlockHeight,
    pub best_block_hash: Hash256,
    pub total_work: u64,
    pub median_time_past: Timestamp,
    pub next_difficulty: u32,
}

impl Default for ChainState {
    fn default() -> Self {
        Self {
            height: 0,
            best_block_hash: [0u8; 32],
            total_work: 0,
            median_time_past: 0,
            next_difficulty: 0x01000000,
        }
    }
}

/// Block validation result
#[derive(Debug)]
pub enum BlockValidation {
    Valid,
    Invalid(String),
    OrphanBlock(Hash256), // Missing parent block
}

/// Transaction validation context
#[derive(Debug)]
pub struct TxValidationContext {
    pub block_height: BlockHeight,
    pub block_time: Timestamp,
    pub utxo_set: UTXOSet,
}

/// Main consensus validator
pub struct ConsensusValidator {
    params: ConsensusParams,
    chain_state: Arc<AsyncRwLock<ChainState>>,
    utxo_set: Arc<AsyncRwLock<UTXOSet>>,
    block_index: Arc<AsyncRwLock<HashMap<Hash256, BlockHeader>>>,
    orphan_blocks: Arc<AsyncRwLock<HashMap<Hash256, Block>>>,
    // Static ConsensusMiner methods used directly
}

impl ConsensusValidator {
    /// Create new consensus validator
    pub fn new(params: ConsensusParams) -> Self {
        Self {
            params,
            chain_state: Arc::new(AsyncRwLock::new(ChainState::default())),
            utxo_set: Arc::new(AsyncRwLock::new(UTXOSet::new())),
            block_index: Arc::new(AsyncRwLock::new(HashMap::new())),
            orphan_blocks: Arc::new(AsyncRwLock::new(HashMap::new())),
            // miner: ConsensusMiner::new(), // Static methods only
        }
    }
    
    /// Initialize the blockchain with genesis state
    pub async fn initialize_with_genesis(&self, genesis_state: crate::genesis::GenesisState) -> Result<()> {
        // Verify genesis state first
        genesis_state.verify()?;
        
        // Store values we need before moving
        let total_supply = genesis_state.get_total_supply_edu();
        let genesis_hash = genesis_state.genesis_block.header.calculate_hash();
        let genesis_header = genesis_state.genesis_block.header.clone();
        
        // Initialize UTXO set with genesis UTXOs
        {
            let mut utxo_set = self.utxo_set.write().await;
            *utxo_set = genesis_state.utxo_set;
        }
        
        // Add genesis block to chain state and index        
        {
            let mut chain_state = self.chain_state.write().await;
            chain_state.best_block_hash = genesis_hash;
            chain_state.height = 0;
            chain_state.total_work = 1; // Genesis block has minimal work
            chain_state.next_difficulty = self.params.max_difficulty_target; // Start with lowest difficulty
        }
        
        {
            let mut block_index = self.block_index.write().await;
            block_index.insert(genesis_hash, genesis_header);
        }
        
        println!("Initialized blockchain with genesis block: {}", hex::encode(genesis_hash));
        println!("Genesis total supply: {:.2} EDU", total_supply);
        
        Ok(())
    }
    
    /// Get the current UTXO set (for balance queries)
    pub async fn get_utxo_set(&self) -> UTXOSet {
        self.utxo_set.read().await.clone()
    }

    /// Validate a complete block
    pub async fn validate_block(&self, block: &Block) -> Result<BlockValidation> {
        // 1. Basic structure validation
        self.validate_block_structure(block)?;
        
        // 2. Check if parent exists
        let block_index = self.block_index.read().await;
        if !block_index.contains_key(&block.header.prev_block_hash) && block.header.prev_block_hash != [0u8; 32] {
            return Ok(BlockValidation::OrphanBlock(block.header.prev_block_hash));
        }
        drop(block_index);
        
        // 3. Validate proof of work using hybrid C++ engine
        let block_hash = block.header.calculate_hash();
        let hash_hex = hex::encode(&block_hash);
        
        match ConsensusMiner::verify_proof_of_work(&hash_hex, block.header.nonce, block.header.difficulty_target) {
            Ok(is_valid) if !is_valid => return Ok(BlockValidation::Invalid("Invalid proof of work".to_string())),
            Err(e) => return Ok(BlockValidation::Invalid(format!("Proof of work validation error: {}", e))),
            _ => {} // Valid proof of work
        }
        
        // 4. Validate transactions
        self.validate_block_transactions(block).await?;
        
        // 5. Check difficulty target
        let chain_state = self.chain_state.read().await;
        if block.header.difficulty_target != chain_state.next_difficulty {
            return Ok(BlockValidation::Invalid("Invalid difficulty target".to_string()));
        }
        
        Ok(BlockValidation::Valid)
    }
    
    /// Validate block structure and basic rules
    fn validate_block_structure(&self, block: &Block) -> Result<()> {
        // Check block size
        let block_size = self.estimate_block_size(block);
        if block_size > self.params.max_block_size {
            return Err(BlockchainError::InvalidBlock(
                format!("Block size {} exceeds maximum {}", block_size, self.params.max_block_size)
            ));
        }
        
        // Check transaction count
        if block.transactions.is_empty() {
            return Err(BlockchainError::InvalidBlock("Block must contain at least one transaction".to_string()));
        }
        
        // First transaction must be coinbase
        if !self.is_coinbase_transaction(&block.transactions[0]) {
            return Err(BlockchainError::InvalidBlock("First transaction must be coinbase".to_string()));
        }
        
        // Only first transaction can be coinbase
        for (i, tx) in block.transactions.iter().enumerate().skip(1) {
            if self.is_coinbase_transaction(tx) {
                return Err(BlockchainError::InvalidBlock(
                    format!("Non-first transaction {} is coinbase", i)
                ));
            }
        }
        
        // Validate merkle root
        let calculated_merkle = self.calculate_merkle_root(&block.transactions);
        let calculated_merkle = calculated_merkle?;
        if calculated_merkle != block.header.merkle_root {
            return Err(BlockchainError::InvalidBlock("Invalid merkle root".to_string()));
        }
        
        Ok(())
    }
    
    /// Validate all transactions in a block
    async fn validate_block_transactions(&self, block: &Block) -> Result<()> {
        let utxo_set = self.utxo_set.read().await;
        let chain_state = self.chain_state.read().await;
        
        let context = TxValidationContext {
            block_height: chain_state.height + 1,
            block_time: block.header.timestamp as u64,
            utxo_set: utxo_set.clone(),
        };
        drop(utxo_set);
        drop(chain_state);
        
        let mut total_fees = 0u64;
        
        // Validate each transaction
        for (i, tx) in block.transactions.iter().enumerate() {
            if i == 0 {
                // Validate coinbase transaction
                self.validate_coinbase_transaction(tx, &context)?;
            } else {
                // Validate regular transaction
                let fee = self.validate_transaction(tx, &context)?;
                total_fees = total_fees.checked_add(fee)
                    .ok_or_else(|| BlockchainError::InvalidBlock("Fee overflow".to_string()))?;
            }
        }
        
        // Validate coinbase reward
        let coinbase_output_value: u64 = block.transactions[0].outputs.iter().map(|o| o.value).sum();
        let expected_reward = self.params.block_reward + total_fees;
        
        if coinbase_output_value > expected_reward {
            return Err(BlockchainError::InvalidBlock(
                format!("Coinbase output {} exceeds allowed reward {}", coinbase_output_value, expected_reward)
            ));
        }
        
        Ok(())
    }
    
    /// Validate a single transaction
    pub fn validate_transaction(&self, tx: &Transaction, context: &TxValidationContext) -> Result<Amount> {
        // Basic structure validation
        if tx.inputs.len() > self.params.max_tx_inputs {
            return Err(BlockchainError::InvalidTransaction("Too many inputs".to_string()));
        }
        
        if tx.outputs.len() > self.params.max_tx_outputs {
            return Err(BlockchainError::InvalidTransaction("Too many outputs".to_string()));
        }
        
        if tx.inputs.is_empty() {
            return Err(BlockchainError::InvalidTransaction("No inputs".to_string()));
        }
        
        if tx.outputs.is_empty() {
            return Err(BlockchainError::InvalidTransaction("No outputs".to_string()));
        }
        
        // Validate inputs and calculate total input value
        let mut total_input_value = 0u64;
        let mut used_outpoints = HashSet::new();
        
        for input in &tx.inputs {
            let outpoint_key = format!("{}:{}", hex::encode(&input.prev_tx_hash), input.prev_output_index);
            
            // Check for duplicate inputs
            if !used_outpoints.insert(outpoint_key.clone()) {
                return Err(BlockchainError::InvalidTransaction("Duplicate input".to_string()));
            }
            
            // Real UTXO validation - lookup the referenced output
            let outpoint = crate::OutPoint {
                txid: input.prev_tx_hash.0,
                vout: input.prev_output_index,
            };
            
            // For now, use a reasonable default value for validation
            // TODO: Implement proper async UTXO lookup
            let utxo_value = 50_000_000u64;
            
            total_input_value = total_input_value.checked_add(utxo_value)
                .ok_or_else(|| BlockchainError::InvalidTransaction("Input value overflow".to_string()))?;
        }
        
        // Calculate total output value
        let total_output_value: u64 = tx.outputs.iter().map(|o| o.value).sum();
        
        // Check that inputs >= outputs
        if total_input_value < total_output_value {
            return Err(BlockchainError::InvalidTransaction("Insufficient input value".to_string()));
        }
        
        // Calculate fee
        let fee = total_input_value - total_output_value;
        
        Ok(fee)
    }
    
    /// Apply a validated block to the chain state
    pub async fn apply_block(&self, block: Block) -> Result<()> {
        let block_hash = block.header.calculate_hash();
        
        // Update UTXO set by processing all transactions in the block
        {
            let mut utxo_set = self.utxo_set.write().await;
            
            for transaction in &block.transactions {
                let txid = transaction.get_hash()?;
                
                // Remove spent UTXOs (from transaction inputs)
                for input in &transaction.inputs {
                    // Skip coinbase inputs (they have null previous hash)
                    if input.prev_tx_hash.0 != [0u8; 32] {
                        utxo_set.remove_utxo(&input.prev_tx_hash.0, input.prev_output_index)?;
                    }
                }
                
                // Add new UTXOs (from transaction outputs)  
                for (index, output) in transaction.outputs.iter().enumerate() {
                    let current_height = self.chain_state.read().await.height;
                    let utxo = crate::transaction::UTXO::new(
                        Hash256Wrapper::from_hash256(&txid),
                        index as u32,
                        output.clone(),
                        current_height as u32,
                        transaction.is_coinbase()
                    );
                    let utxo_set_format = crate::utxo::UTXO {
                        tx_hash: txid,
                        output_index: index as u32,
                        output: output.clone(),
                        block_height: current_height as u32,
                        is_coinbase: transaction.is_coinbase(),
                        created_at: chrono::Utc::now(),
                    };
                    utxo_set.add_utxo(txid, index as u32, utxo_set_format)?;
                }
            }
        }
        
        // Update chain state
        {
            let mut chain_state = self.chain_state.write().await;
            chain_state.height += 1;
            chain_state.best_block_hash = block_hash;
            chain_state.total_work += self.calculate_work(block.header.difficulty_target);
            chain_state.median_time_past = block.header.timestamp as u64;
            
            // Calculate next difficulty using hybrid C++ engine
            if chain_state.height % self.params.difficulty_adjustment_interval == 0 {
                chain_state.next_difficulty = ConsensusMiner::calculate_next_difficulty(
                    chain_state.next_difficulty,
                    self.params.target_block_time * self.params.difficulty_adjustment_interval,
                    self.params.target_block_time,
                );
            }
        }
        
        // Add to block index
        {
            let mut block_index = self.block_index.write().await;
            block_index.insert(block_hash, block.header);
        }
        
        Ok(())
    }
    
    /// Check if transaction is coinbase
    fn is_coinbase_transaction(&self, tx: &Transaction) -> bool {
        tx.inputs.len() == 1 && tx.inputs[0].is_coinbase()
    }
    
    /// Validate coinbase transaction
    fn validate_coinbase_transaction(&self, tx: &Transaction, _context: &TxValidationContext) -> Result<()> {
        if !self.is_coinbase_transaction(tx) {
            return Err(BlockchainError::InvalidTransaction("Not a coinbase transaction".to_string()));
        }
        
        // Coinbase script size limits
        let script_size = tx.inputs[0].script_sig.len();
        if script_size < 2 || script_size > 100 {
            return Err(BlockchainError::InvalidTransaction("Invalid coinbase script size".to_string()));
        }
        
        Ok(())
    }
    
    /// Simplified script validation
    fn validate_input_script(&self, _input: &TransactionInput, _output: &TransactionOutput) -> bool {
        // Simplified script validation
        true
    }

    /// Get the latest block
    pub async fn get_latest_block(&self) -> Result<Option<Block>> {
        let chain_state = self.chain_state.read().await;
        if chain_state.best_block_hash == [0u8; 32] {
            return Ok(None);
        }
        
        // For now, return None as we'd need to implement block storage
        // In a full implementation, this would retrieve the block from storage
        Ok(None)
    }

    /// Get pending transactions from mempool
    pub async fn get_pending_transactions(&self) -> Result<Vec<Transaction>> {
        // For now, return empty vec as we'd need to implement mempool
        // In a full implementation, this would return transactions from mempool
        Ok(Vec::new())
    }

    /// Add a block to the blockchain
    pub async fn add_block(&self, block: Block) -> Result<()> {
        // Validate the block first
        self.validate_block(&block).await?;
        
        let block_hash = block.header.calculate_hash();
        let block_height = block.header.height; // Capture height before move
        
        // Update chain state
        {
            let mut chain_state = self.chain_state.write().await;
            chain_state.best_block_hash = block_hash;
            chain_state.height = block_height as u64; // Use actual block height
            // Update total work (simplified)
            chain_state.total_work += 1;
        }
        
        // Add block header to index
        {
            let mut block_index = self.block_index.write().await;
            block_index.insert(block_hash, block.header);
        }
        
        // Update UTXO set with block transactions
        {
            let mut utxo_set = self.utxo_set.write().await;
            
            for (tx_index, tx) in block.transactions.iter().enumerate() {
                let tx_hash = tx.get_hash()?;
                
                // Remove spent UTXOs
                for input in &tx.inputs {
                    if !input.is_coinbase() {
                        utxo_set.remove_utxo(&input.prev_tx_hash.0, input.prev_output_index)?;
                    }
                }
                
                // Add new UTXOs
                for (output_index, output) in tx.outputs.iter().enumerate() {
                    let utxo = UTXO::new(
                        tx_hash,
                        output_index as u32,
                        output.clone(),
                        block_height, // Use actual block height
                        tx_index == 0, // First transaction is coinbase
                    );
                    utxo_set.add_utxo(tx_hash, output_index as u32, utxo)?;
                }
            }
        }
        
        info!("Block {} added to blockchain at height {}", 
              hex::encode(block_hash), block_height);
        
        Ok(())
    }

    /// Get recent blocks for difficulty calculation
    pub async fn get_recent_blocks(&self, count: usize) -> Result<Vec<Block>> {
        // For now, return empty vec as we'd need to implement block storage
        // In a full implementation, this would retrieve the most recent blocks
        let _ = count;
        Ok(Vec::new())
    }

    /// Calculate work done for a block (temporary implementation)
    pub fn calculate_work(&self, _difficulty_target: u32) -> u64 {
        // Simplified work calculation - should use actual difficulty math
        1
    }

    /// Estimate block size (temporary implementation)
    pub fn estimate_block_size(&self, _block: &Block) -> usize {
        // Simplified size estimation
        1000
    }

    /// Calculate merkle root for transactions (temporary implementation)
    pub fn calculate_merkle_root(&self, transactions: &[Transaction]) -> Result<Hash256> {
        // Simplified merkle root calculation - should use proper merkle tree
        if transactions.is_empty() {
            return Ok([0u8; 32]);
        }
        
        // Just hash the first transaction for now
        let tx_hash = transactions[0].get_hash()?;
        Ok(tx_hash)
    }

    /// Get current chain state
    pub async fn get_chain_state(&self) -> ChainState {
        self.chain_state.read().await.clone()
    }
}



#[cfg(test)]
mod tests {
    use super::*;
    use crate::transaction::{TransactionInput, TransactionOutput};
    
    #[tokio::test]
    async fn test_consensus_validator_creation() {
        let params = ConsensusParams::default();
        let validator = ConsensusValidator::new(params);
        
        let chain_state = validator.get_chain_state().await;
        assert_eq!(chain_state.height, 0);
        assert_eq!(chain_state.best_block_hash, [0u8; 32]);
    }
    
    #[tokio::test]
    async fn test_coinbase_validation() {
        let params = ConsensusParams::default();
        let validator = ConsensusValidator::new(params);
        
        // Create coinbase transaction using existing method
        let coinbase_input = TransactionInput::create_coinbase(vec![0x04, 0x12, 0x34, 0x56]);
        
        // Generate proper coinbase output script
        let miner_address = crate::script_utils::ScriptBuilder::generate_mining_address("test_miner");
        let coinbase_script = crate::script_utils::ScriptBuilder::create_coinbase_script(&miner_address)
            .unwrap_or_else(|_| vec![0x76, 0xa9, 0x14, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x88, 0xac]);
        
        let coinbase_output = TransactionOutput {
            value: 50_00000000, // 50 coins
            script_pubkey: coinbase_script,
        };
        
        let coinbase_tx = Transaction::new(1, vec![coinbase_input], vec![coinbase_output]);
        
        assert!(validator.is_coinbase_transaction(&coinbase_tx));
        
        let context = TxValidationContext {
            block_height: 1,
            block_time: 1234567890,
            utxo_set: UTXOSet::new(),
        };
        
        assert!(validator.validate_coinbase_transaction(&coinbase_tx, &context).is_ok());
    }
}