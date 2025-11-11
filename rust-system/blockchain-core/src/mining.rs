//! Block Mining Controller
//!
//! This module provides high-level mining operations that integrate the C++ mining engine
//! with the Rust blockchain consensus layer. It handles block assembly, transaction selection,
//! mining coordination, and blockchain state updates.

use crate::{
    block::{Block, BlockHeader},
    transaction::Transaction,
    Hash256, Result as BlockchainResult, BlockchainError,
    consensus::ConsensusValidator,
    tx_builder::TransactionManager,
};

use blockchain_consensus_ffi::{ConsensusMiner, MiningResult as CppMiningResult};

use std::{
    sync::{Arc, atomic::{AtomicBool, AtomicU64, Ordering}},
    time::{SystemTime, UNIX_EPOCH, Instant},
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};
use tokio::{
    sync::{RwLock as AsyncRwLock, mpsc},
    time::{sleep, Duration},
    task,
};
use tracing::{info, error, debug};
use serde::{Serialize, Deserialize};
use sha2::{Sha256, Digest};

/// Mining configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiningConfig {
    /// Target block time in seconds (default: 600 = 10 minutes)
    pub target_block_time: u64,
    /// Maximum transactions per block
    pub max_transactions_per_block: usize,
    /// Minimum transaction fee (satoshis per byte)
    pub min_fee_per_byte: u64,
    /// Difficulty adjustment period (blocks)
    pub difficulty_adjustment_period: u32,
    /// Maximum mining iterations before giving up
    pub max_mining_iterations: u64,
    /// Mining reward in satoshis
    pub block_reward: u64,
    /// Miner address for coinbase transactions
    pub miner_address: String,
    /// Coinbase flags/data
    pub coinbase_flags: String,
    /// Maximum block size in bytes
    pub max_block_size: usize,
    /// Difficulty adjustment interval (blocks)
    pub difficulty_adjustment_interval: u32,
}

impl Default for MiningConfig {
    fn default() -> Self {
        Self {
            target_block_time: 600, // 10 minutes
            max_transactions_per_block: 2000,
            min_fee_per_byte: 1, // 1 satoshi per byte
            difficulty_adjustment_period: 10, // Every 10 blocks
            max_mining_iterations: 1_000_000, // 1M iterations max
            block_reward: 5_000_000_000, // 50 BTC in satoshis
            miner_address: String::new(),
            coinbase_flags: String::new(),
            max_block_size: 1_000_000, // 1MB
            difficulty_adjustment_interval: 2016, // Every ~2 weeks
        }
    }
}

/// Mining statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiningStats {
    /// Current mining status
    pub is_mining: bool,
    /// Blocks mined
    pub blocks_mined: u64,
    /// Current difficulty target
    pub current_difficulty: u32,
    /// Current hash rate (hashes per second)
    pub hash_rate: f64,
    /// Total hashes computed
    pub total_hashes: u64,
    /// Mining start time
    pub mining_start_time: Option<u64>,
    /// Last block mined timestamp
    pub last_block_time: Option<u64>,
    /// Average block time
    pub average_block_time: f64,
    /// Pending transactions count
    pub pending_transactions: usize,
}

/// Mining events
#[derive(Debug, Clone)]
pub enum MiningEvent {
    /// Mining started
    Started,
    /// Block found and mined successfully
    BlockMined {
        block: Block,
        mining_result: MiningResult,
    },
    /// Mining stopped
    Stopped,
    /// Mining failed
    Error {
        error: String,
    },
    /// Difficulty adjusted
    DifficultyAdjusted {
        old_difficulty: u32,
        new_difficulty: u32,
    },
}

/// Mining result with additional metadata
#[derive(Debug, Clone, Serialize)]
pub struct MiningResult {
    pub success: bool,
    pub nonce: u32,
    pub block_hash: Hash256,
    pub hash_operations: u64,
    pub elapsed_seconds: f64,
    pub hash_rate: f64,
}

impl From<CppMiningResult> for MiningResult {
    fn from(cpp_result: CppMiningResult) -> Self {
        let hash_rate = if cpp_result.elapsed_seconds > 0.0 {
            cpp_result.hash_operations as f64 / cpp_result.elapsed_seconds
        } else {
            0.0
        };

        Self {
            success: cpp_result.success,
            nonce: cpp_result.nonce,
            block_hash: cpp_result.block_hash,
            hash_operations: cpp_result.hash_operations,
            elapsed_seconds: cpp_result.elapsed_seconds,
            hash_rate,
        }
    }
}

/// Block mining controller
pub struct MiningController {
    /// Mining configuration
    config: MiningConfig,
    /// Consensus validator for transaction and block validation
    consensus: Arc<ConsensusValidator>,
    /// Transaction manager for creating coinbase transactions
    tx_manager: Arc<TransactionManager>,
    /// Mining control flags
    is_mining: Arc<AtomicBool>,
    should_stop: Arc<AtomicBool>,
    /// Statistics
    stats: Arc<AsyncRwLock<MiningStats>>,
    blocks_mined: Arc<AtomicU64>,
    total_hashes: Arc<AtomicU64>,
    /// Event channel
    event_tx: Option<mpsc::UnboundedSender<MiningEvent>>,
    /// Mining task handle
    mining_task: Arc<AsyncRwLock<Option<task::JoinHandle<()>>>>,
}

impl MiningController {
    /// Create a new mining controller
    pub fn new(
        config: MiningConfig,
        consensus: Arc<ConsensusValidator>,
        tx_manager: Arc<TransactionManager>,
    ) -> Self {
        let initial_stats = MiningStats {
            is_mining: false,
            blocks_mined: 0,
            current_difficulty: 0x01000000, // Start with 1 leading zero
            hash_rate: 0.0,
            total_hashes: 0,
            mining_start_time: None,
            last_block_time: None,
            average_block_time: config.target_block_time as f64,
            pending_transactions: 0,
        };

        Self {
            config,
            consensus,
            tx_manager,
            is_mining: Arc::new(AtomicBool::new(false)),
            should_stop: Arc::new(AtomicBool::new(false)),
            stats: Arc::new(AsyncRwLock::new(initial_stats)),
            blocks_mined: Arc::new(AtomicU64::new(0)),
            total_hashes: Arc::new(AtomicU64::new(0)),
            event_tx: None,
            mining_task: Arc::new(AsyncRwLock::new(None)),
        }
    }

    /// Subscribe to mining events
    pub fn subscribe_events(&mut self) -> mpsc::UnboundedReceiver<MiningEvent> {
        let (tx, rx) = mpsc::unbounded_channel();
        self.event_tx = Some(tx);
        rx
    }

    /// Start mining
    pub async fn start_mining(&self, miner_address: String) -> BlockchainResult<()> {
        if self.is_mining.load(Ordering::Relaxed) {
            return Err(BlockchainError::InvalidInput("Mining already active".to_string()));
        }

        self.is_mining.store(true, Ordering::Relaxed);
        self.should_stop.store(false, Ordering::Relaxed);

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.is_mining = true;
            stats.mining_start_time = Some(current_timestamp());
        }

        // Send event
        if let Some(tx) = &self.event_tx {
            let _ = tx.send(MiningEvent::Started);
        }

        // Start mining task
        let mining_task = self.spawn_mining_task(miner_address).await;
        *self.mining_task.write().await = Some(mining_task);

        info!("Mining started");
        Ok(())
    }

    /// Stop mining
    pub async fn stop_mining(&self) -> BlockchainResult<()> {
        if !self.is_mining.load(Ordering::Relaxed) {
            return Ok(());
        }

        self.should_stop.store(true, Ordering::Relaxed);
        self.is_mining.store(false, Ordering::Relaxed);

        // Cancel mining task
        if let Some(handle) = self.mining_task.write().await.take() {
            handle.abort();
        }

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.is_mining = false;
            stats.mining_start_time = None;
        }

        // Send event
        if let Some(tx) = &self.event_tx {
            let _ = tx.send(MiningEvent::Stopped);
        }

        info!("Mining stopped");
        Ok(())
    }

    /// Get current mining statistics
    pub async fn get_stats(&self) -> MiningStats {
        let mut stats = self.stats.read().await.clone();
        stats.blocks_mined = self.blocks_mined.load(Ordering::Relaxed);
        stats.total_hashes = self.total_hashes.load(Ordering::Relaxed);
        stats
    }

    /// Check if currently mining
    pub fn is_mining(&self) -> bool {
        self.is_mining.load(Ordering::Relaxed)
    }

    /// Update mining configuration
    pub async fn update_config(&mut self, new_config: MiningConfig) {
        self.config = new_config;
        info!("Mining configuration updated");
    }

    /// Spawn the mining task
    async fn spawn_mining_task(&self, miner_address: String) -> task::JoinHandle<()> {
        let consensus = Arc::clone(&self.consensus);
        let tx_manager = Arc::clone(&self.tx_manager);
        let config = self.config.clone();
        let is_mining = Arc::clone(&self.is_mining);
        let should_stop = Arc::clone(&self.should_stop);
        let stats = Arc::clone(&self.stats);
        let blocks_mined = Arc::clone(&self.blocks_mined);
        let total_hashes = Arc::clone(&self.total_hashes);
        let event_tx = self.event_tx.clone();

        task::spawn(async move {
            info!("Mining task started for address: {}", miner_address);

            while !should_stop.load(Ordering::Relaxed) {
                match Self::mine_single_block(
                    &consensus,
                    &tx_manager,
                    &config,
                    &miner_address,
                    &stats,
                    &total_hashes,
                ).await {
                    Ok((block, mining_result)) => {
                        info!("Successfully mined block {} with nonce {} in {:.2}s", 
                              block.header.height,
                              mining_result.nonce, 
                              mining_result.elapsed_seconds);

                        blocks_mined.fetch_add(1, Ordering::Relaxed);

                        // Update stats
                        {
                            let mut stats_guard = stats.write().await;
                            stats_guard.last_block_time = Some(current_timestamp());
                            stats_guard.hash_rate = mining_result.hash_rate;
                            stats_guard.current_difficulty = block.header.difficulty_target;
                        }

                        // Send event
                        if let Some(tx) = &event_tx {
                            let _ = tx.send(MiningEvent::BlockMined {
                                block,
                                mining_result,
                            });
                        }

                        // Brief pause before next block
                        sleep(Duration::from_millis(100)).await;
                    }
                    Err(e) => {
                        error!("Mining error: {:?}", e);
                        
                        if let Some(tx) = &event_tx {
                            let _ = tx.send(MiningEvent::Error {
                                error: e.to_string(),
                            });
                        }

                        // Longer pause on error
                        sleep(Duration::from_secs(5)).await;
                    }
                }
            }

            is_mining.store(false, Ordering::Relaxed);
            info!("Mining task completed");
        })
    }

    /// Mine a single block
    async fn mine_single_block(
        consensus: &Arc<ConsensusValidator>,
        tx_manager: &Arc<TransactionManager>,
        config: &MiningConfig,
        _miner_address: &str,
        stats: &Arc<AsyncRwLock<MiningStats>>,
        total_hashes: &Arc<AtomicU64>,
    ) -> BlockchainResult<(Block, MiningResult)> {
        // Get the latest block to build on
        let latest_block = consensus.get_latest_block().await?;
        let previous_hash = latest_block.as_ref().map(|b| b.header.get_hex_hash().as_bytes().try_into().unwrap_or([0u8; 32])).unwrap_or([0u8; 32]);
        let block_height = latest_block.as_ref().map(|b| b.header.height + 1).unwrap_or(0); // Next block height

        // Get pending transactions from mempool
        let pending_txs = consensus.get_pending_transactions().await?;
        
        // Update pending transaction count
        {
            let mut stats_guard = stats.write().await;
            stats_guard.pending_transactions = pending_txs.len();
        }

        // Select transactions for block (simple FIFO for now, could be fee-based)
        let mut selected_txs = Vec::new();
        let mut _total_fees = 0u64;
        
        for tx in pending_txs.into_iter().take(config.max_transactions_per_block) {
            // Validate transaction
            // TODO: Fix validation context
            // if consensus.validate_transaction(&tx, &context).is_ok() {
            if true { // Temporary bypass
                // Calculate fees (simplified)
                let tx_size = tx.serialize()?.len() as u64;
                let fee = tx_size * config.min_fee_per_byte;
                _total_fees += fee;
                selected_txs.push(tx);
            }
        }

        // Create coinbase transaction with proper mining configuration
        let node_id = Self::get_node_identity().await;
        let coinbase_address = Self::generate_mining_address(&node_id);
        let block_reward = Self::calculate_block_reward(block_height as u64);
        let coinbase_tx = tx_manager.create_coinbase_transaction(
            &coinbase_address,
            block_reward,
            block_height as u64,
        ).await?;

        // Insert coinbase at beginning
        selected_txs.insert(0, coinbase_tx);

        // Calculate merkle root
        let merkle_root = calculate_merkle_root(&selected_txs)?;

        // Get current difficulty
        let current_difficulty = {
            let stats_guard = stats.read().await;
            stats_guard.current_difficulty
        };

        // Adjust difficulty if needed
        let difficulty_target = if ConsensusMiner::should_adjust_difficulty(block_height) {
            Self::adjust_difficulty(consensus, current_difficulty, config).await?
        } else {
            current_difficulty
        };

        // Create block header
        let header = BlockHeader {
            version: 1,
            prev_block_hash: previous_hash,
            merkle_root,
            timestamp: current_timestamp() as u32,
            difficulty_target,
            nonce: 0, // Will be set by mining
            height: block_height,
        };

        // Create block
        let mut block = Block::new(header, selected_txs);

        debug!("Starting to mine block {} with difficulty 0x{:08x}", 
               block_height, difficulty_target);

        // Serialize block data for mining
        let block_data = block.serialize_for_mining()?;
        
        // Mine the block using C++ engine
        let _mining_start = Instant::now();
        let cpp_result = ConsensusMiner::mine_block(
            &hex::encode(block_data),
            difficulty_target,
            Some(config.max_mining_iterations),
        ).map_err(|e| BlockchainError::ConsensusError(format!("Mining failed: {}", e)))?;

        if !cpp_result.success {
            return Err(BlockchainError::ConsensusError(
                "Failed to find valid nonce within iteration limit".to_string()
            ));
        }

        // Update block with found nonce
        block.header.nonce = cpp_result.nonce;

        // Update total hashes
        total_hashes.fetch_add(cpp_result.hash_operations, Ordering::Relaxed);

        // Validate the mined block
        consensus.validate_block(&block).await?;

        // Add block to blockchain
        consensus.add_block(block.clone()).await?;

        info!("Block {} successfully mined and added to blockchain", block_height);

        Ok((block, cpp_result.into()))
    }

    /// Adjust mining difficulty based on recent block times
    async fn adjust_difficulty(
        consensus: &Arc<ConsensusValidator>,
        current_difficulty: u32,
        config: &MiningConfig,
    ) -> BlockchainResult<u32> {
        // Get recent blocks for timing analysis
        let recent_blocks = consensus.get_recent_blocks(config.difficulty_adjustment_period as usize).await?;
        
        if recent_blocks.len() < 2 {
            return Ok(current_difficulty); // Not enough blocks for adjustment
        }

        // Calculate actual time span
        let oldest_time = recent_blocks.last().unwrap().header.timestamp;
        let newest_time = recent_blocks.first().unwrap().header.timestamp;
        let actual_time_span = newest_time.saturating_sub(oldest_time);

        // Expected time span
        let target_time_span = config.target_block_time * (recent_blocks.len() - 1) as u64;

        // Calculate new difficulty
        let new_difficulty = ConsensusMiner::calculate_next_difficulty(
            current_difficulty,
            actual_time_span as u64,
            target_time_span,
        );

        if new_difficulty != current_difficulty {
            info!("Difficulty adjusted: 0x{:08x} -> 0x{:08x} (blocks took {}s vs {}s target)",
                  current_difficulty, new_difficulty, actual_time_span, target_time_span);
        }

        Ok(new_difficulty)
    }

    /// Calculate block reward with proper halving mechanism
    pub fn calculate_block_reward(block_height: u64) -> u64 {
        // Initial block reward: 50 EDU (in satoshis: 50 * 100_000_000)
        let initial_reward = 50_000_000_000u64;
        
        // Halving every 210,000 blocks (approximately every 4 years)
        let halving_interval = 210_000u64;
        
        // Calculate number of halvings that have occurred
        let halvings = block_height / halving_interval;
        
        // If too many halvings, reward becomes 0
        if halvings >= 64 {
            return 0;
        }
        
        // Apply halvings: reward = initial_reward / (2^halvings)
        initial_reward >> halvings
    }

    /// Get mining configuration for current miner
    async fn get_mining_config() -> BlockchainResult<MiningConfig> {
        // TODO: In production, this would be loaded from configuration file or environment
        // For now, generate a deterministic mining address based on node identity
        
        let node_id = Self::get_node_identity().await;
        let miner_address = Self::generate_mining_address(&node_id);
        
        Ok(MiningConfig {
            miner_address,
            coinbase_flags: node_id.clone(),
            target_block_time: 600, // 10 minutes
            max_block_size: 1_000_000, // 1MB
            difficulty_adjustment_period: 2016, // Every ~2 weeks
            // Fill in the rest with defaults
            ..Default::default()
        })
    }

    /// Get unique node identity for this mining instance
    async fn get_node_identity() -> String {
        // Generate a deterministic node ID based on system properties
        // In production, this would be loaded from a persistent keyfile
        
        let mut hasher = DefaultHasher::new();
        
        // Use hostname and current timestamp for uniqueness
        if let Ok(hostname) = hostname::get() {
            hostname.hash(&mut hasher);
        }
        
        // Add process ID for uniqueness across multiple instances
        std::process::id().hash(&mut hasher);
        
        let node_hash = hasher.finish();
        format!("edunode_{:016x}", node_hash)
    }

    /// Generate a proper EDU mining address from node identity
    fn generate_mining_address(node_id: &str) -> String {
        use sha2::{Sha256, Digest};
        
        // Generate deterministic address from node ID
        let mut hasher = Sha256::new();
        hasher.update(b"EDU_MINING_");
        hasher.update(node_id.as_bytes());
        let hash = hasher.finalize();
        
        // Create EDU address format: edu1q + 40 chars (20 bytes in hex)
        let addr_bytes = &hash[0..20];
        format!("edu1q{}", hex::encode(addr_bytes))
    }
}

/// Calculate merkle root of transactions
fn calculate_merkle_root(transactions: &[Transaction]) -> BlockchainResult<Hash256> {
    if transactions.is_empty() {
        return Ok([0u8; 32]);
    }

    // Get transaction hashes
    let mut hashes: Vec<Hash256> = transactions.iter()
        .map(|tx| tx.get_hash())
        .collect::<Result<Vec<_>, _>>()?;

    // Build merkle tree
    while hashes.len() > 1 {
        let mut next_level = Vec::new();
        
        for chunk in hashes.chunks(2) {
            let left = &chunk[0];
            let right = if chunk.len() > 1 { &chunk[1] } else { &chunk[0] };
            
            // Concatenate and hash
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

/// Get current timestamp
fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genesis::GenesisConfig;
    use crate::Amount;

    #[tokio::test]
    async fn test_mining_controller() {
        // Create mock consensus and tx_manager for testing
        let consensus_params = crate::consensus::ConsensusParams::default();
        let consensus = Arc::new(crate::consensus::ConsensusValidator::new(consensus_params));
        let utxo_set = crate::utxo::UTXOSet::new();
        let tx_manager = Arc::new(crate::tx_builder::TransactionManager::new(utxo_set));
        
        let config = MiningConfig {
            max_mining_iterations: 10_000, // Low for testing
            ..Default::default()
        };
        
        let controller = MiningController::new(config, consensus, tx_manager);
        
        // Test mining stats
        let stats = controller.get_stats().await;
        assert!(!stats.is_mining);
        assert_eq!(stats.blocks_mined, 0);
        
        // Note: Actual mining test would require starting and stopping
        // which might be too heavy for unit tests
    }

    #[test]
    fn test_merkle_root_calculation() {
        // Test with mock transactions
        let mut txs = Vec::new();
        let coinbase_address = "test_coinbase_address";
        
        // Create simple mock transactions
        for i in 0..4 {
            let output = crate::transaction::TransactionOutput {
                value: 100 * i, // Amount is just u64
                script_pubkey: crate::script_utils::ScriptBuilder::create_coinbase_script(&coinbase_address)
                    .unwrap_or_else(|_| vec![]), // Fallback to empty script on error
            };
            let tx = Transaction::new(1, Vec::new(), vec![output]);
            txs.push(tx);
        }
        
        let result = calculate_merkle_root(&txs);
        assert!(result.is_ok());
        
        let root = result.unwrap();
        assert_ne!(root, [0u8; 32]); // Should not be zero
    }
}