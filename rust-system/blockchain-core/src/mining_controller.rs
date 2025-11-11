//! High-performance mining controller using C++ FFI
//!
//! This module provides a safe Rust wrapper around the C++ mining engine,
//! combining the performance of C++ with the safety and ergonomics of Rust.

use crate::{BlockchainError, Result, Hash256, block::{Block, BlockHeader}, transaction::Transaction};
use blockchain_ffi::{
    c_mine_block, c_verify_proof_of_work, c_calculate_next_difficulty, c_should_adjust_difficulty, CMiningResult
};
use serde::{Deserialize, Serialize};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::{RwLock, mpsc, broadcast};
use tracing::{info, warn, error, debug};
use uuid::Uuid;

/// Mining configuration parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiningConfig {
    /// Mining difficulty target (bits format)
    pub difficulty_target: u32,
    /// Maximum iterations per mining attempt
    pub max_iterations: u64,
    /// Mining thread count
    pub thread_count: usize,
    /// Mining timeout in seconds
    pub timeout_seconds: u64,
    /// Coinbase reward in satoshis
    pub coinbase_reward: u64,
    /// Miner address for coinbase transaction
    pub miner_address: String,
}

impl Default for MiningConfig {
    fn default() -> Self {
        Self {
            difficulty_target: 0x1d00ffff, // Bitcoin's initial difficulty
            max_iterations: 1_000_000,
            thread_count: num_cpus::get(),
            timeout_seconds: 300, // 5 minutes
            coinbase_reward: 50_0000_0000, // 50 EDU
            miner_address: "edu1qdefaultmineraddress".to_string(),
        }
    }
}

/// Mining result from C++ engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiningResult {
    /// Whether mining was successful
    pub success: bool,
    /// Found nonce (if successful)
    pub nonce: u32,
    /// Block hash (if successful)
    pub block_hash: Hash256,
    /// Number of hash operations performed
    pub hash_operations: u64,
    /// Time taken for mining
    pub elapsed_time: Duration,
}

impl From<CMiningResult> for MiningResult {
    fn from(c_result: CMiningResult) -> Self {
        Self {
            success: c_result.success,
            nonce: c_result.nonce,
            block_hash: c_result.block_hash,
            hash_operations: c_result.hash_operations,
            elapsed_time: Duration::from_secs_f64(c_result.elapsed_seconds),
        }
    }
}

/// Mining statistics
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct MiningStats {
    /// Total blocks mined
    pub blocks_mined: u64,
    /// Total hash operations
    pub total_hash_operations: u64,
    /// Average hash rate (hashes/second)
    pub hash_rate: f64,
    /// Mining success rate
    pub success_rate: f64,
    /// Total mining time
    pub total_mining_time: Duration,
    /// Current mining status
    pub is_mining: bool,
    /// Current mining job ID
    pub current_job_id: Option<Uuid>,
}

/// Mining events for subscribers
#[derive(Debug, Clone)]
pub enum MiningEvent {
    /// Mining started
    MiningStarted { job_id: Uuid, config: MiningConfig },
    /// Block successfully mined
    BlockMined { job_id: Uuid, result: MiningResult, block: Block },
    /// Mining attempt failed
    MiningFailed { job_id: Uuid, reason: String },
    /// Mining stopped
    MiningStopped { job_id: Uuid },
}

/// High-performance mining controller using C++ FFI
pub struct MiningController {
    /// Current mining configuration
    config: Arc<RwLock<MiningConfig>>,
    /// Mining statistics
    stats: Arc<RwLock<MiningStats>>,
    /// Mining job cancel channel
    cancel_sender: Option<mpsc::Sender<()>>,
    /// Event broadcaster for mining events
    event_sender: broadcast::Sender<MiningEvent>,
}

impl MiningController {
    /// Create new mining controller
    pub fn new() -> Self {
        let (event_sender, _) = broadcast::channel(100);
        
        Self {
            config: Arc::new(RwLock::new(MiningConfig::default())),
            stats: Arc::new(RwLock::new(MiningStats::default())),
            cancel_sender: None,
            event_sender,
        }
    }

    /// Get current mining configuration
    pub async fn get_config(&self) -> MiningConfig {
        let config = self.config.read().await;
        config.clone()
    }

    /// Update mining configuration
    pub async fn update_config(&self, new_config: MiningConfig) -> Result<()> {
        let mut config = self.config.write().await;
        *config = new_config.clone();
        
        info!("Updated mining configuration: difficulty_target={:x}, max_iterations={}", 
              new_config.difficulty_target, new_config.max_iterations);
        Ok(())
    }

    /// Get current mining statistics
    pub async fn get_stats(&self) -> MiningStats {
        let stats = self.stats.read().await;
        stats.clone()
    }

    /// Check if currently mining
    pub async fn is_mining(&self) -> bool {
        let stats = self.stats.read().await;
        stats.is_mining
    }

    /// Start mining a block
    pub async fn start_mining(&mut self, block_template: Block) -> Result<Uuid> {
        // Check if already mining
        if self.is_mining().await {
            return Err(BlockchainError::InvalidInput("Already mining".to_string()));
        }

        let job_id = Uuid::new_v4();
        let config = self.get_config().await;
        
        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.is_mining = true;
            stats.current_job_id = Some(job_id);
        }

        // Create cancel channel
        let (cancel_tx, mut cancel_rx) = mpsc::channel::<()>(1);
        self.cancel_sender = Some(cancel_tx);

        // Clone necessary data for the mining task
        let stats_arc = self.stats.clone();
        let event_sender = self.event_sender.clone();
        
        // Broadcast mining started event
        let _ = self.event_sender.send(MiningEvent::MiningStarted {
            job_id,
            config: config.clone(),
        });

        // Start mining task
        tokio::spawn(async move {
            let result = Self::mine_block_async(
                block_template,
                config,
                job_id,
                stats_arc.clone(),
                &mut cancel_rx,
            ).await;

            match result {
                Ok((mining_result, mined_block)) => {
                    // Update statistics
                    {
                        let mut stats = stats_arc.write().await;
                        stats.blocks_mined += 1;
                        stats.total_hash_operations += mining_result.hash_operations;
                        stats.total_mining_time += mining_result.elapsed_time;
                        
                        // Calculate hash rate
                        if !stats.total_mining_time.is_zero() {
                            stats.hash_rate = stats.total_hash_operations as f64 / 
                                stats.total_mining_time.as_secs_f64();
                        }
                        
                        // Calculate success rate
                        stats.success_rate = stats.blocks_mined as f64 / 
                            (stats.blocks_mined + 0).max(1) as f64; // Placeholder for failed attempts
                        
                        stats.is_mining = false;
                        stats.current_job_id = None;
                    }

                    // Broadcast success event
                    let _ = event_sender.send(MiningEvent::BlockMined {
                        job_id,
                        result: mining_result,
                        block: mined_block,
                    });
                }
                Err(e) => {
                    // Update statistics
                    {
                        let mut stats = stats_arc.write().await;
                        stats.is_mining = false;
                        stats.current_job_id = None;
                    }

                    // Broadcast failure event
                    let _ = event_sender.send(MiningEvent::MiningFailed {
                        job_id,
                        reason: e.to_string(),
                    });
                }
            }
        });

        info!("Started mining job: {}", job_id);
        Ok(job_id)
    }

    /// Stop current mining operation
    pub async fn stop_mining(&mut self) -> Result<()> {
        if let Some(cancel_sender) = self.cancel_sender.take() {
            let _ = cancel_sender.send(()).await;
            
            // Update stats
            {
                let mut stats = self.stats.write().await;
                if let Some(job_id) = stats.current_job_id.take() {
                    stats.is_mining = false;
                    
                    // Broadcast stop event
                    let _ = self.event_sender.send(MiningEvent::MiningStopped { job_id });
                }
            }
            
            info!("Stopped mining operation");
            Ok(())
        } else {
            Err(BlockchainError::InvalidInput("Not currently mining".to_string()))
        }
    }

    /// Get event receiver for mining events
    pub fn subscribe_events(&self) -> broadcast::Receiver<MiningEvent> {
        self.event_sender.subscribe()
    }

    /// Mine block using C++ FFI (async wrapper)
    async fn mine_block_async(
        mut block: Block,
        config: MiningConfig,
        job_id: Uuid,
        stats_arc: Arc<RwLock<MiningStats>>,
        cancel_rx: &mut mpsc::Receiver<()>,
    ) -> Result<(MiningResult, Block)> {
        let start_time = Instant::now();
        
        // Create coinbase transaction
        let coinbase_tx = Self::create_coinbase_transaction(
            config.coinbase_reward,
            &config.miner_address,
            block.header.height,
        )?;
        
        // Add coinbase transaction to block
        block.transactions.insert(0, coinbase_tx);
        
        // Calculate merkle root
        block.header.merkle_root = block.calculate_merkle_root()?;
        
        // Serialize block header for mining
        let block_data = format!(
            "{}{}{}{}{}{}",
            block.header.version,
            hex::encode(block.header.previous_hash),
            hex::encode(block.header.merkle_root),
            block.header.timestamp,
            block.header.difficulty_target,
            0u64 // nonce placeholder
        );

        // Mine in thread pool to avoid blocking async runtime
        let block_data_clone = block_data.clone();
        let difficulty_target = config.difficulty_target;
        let max_iterations = config.max_iterations;
        let timeout = Duration::from_secs(config.timeout_seconds);

        let mining_task = tokio::task::spawn_blocking(move || {
            let c_str = std::ffi::CString::new(block_data_clone)
                .map_err(|e| BlockchainError::InvalidInput(format!("Invalid block data: {}", e)))?;
            
            let c_result = unsafe {
                c_mine_block(c_str.as_ptr(), difficulty_target, max_iterations)
            };
            
            Ok::<CMiningResult, BlockchainError>(c_result)
        });

        // Wait for mining completion or cancellation
        tokio::select! {
            // Check for cancellation
            _ = cancel_rx.recv() => {
                return Err(BlockchainError::InvalidInput("Mining cancelled".to_string()));
            }
            
            // Check for timeout
            _ = tokio::time::sleep(timeout) => {
                return Err(BlockchainError::InvalidInput("Mining timeout".to_string()));
            }
            
            // Mining result
            mining_result = mining_task => {
                match mining_result {
                    Ok(Ok(c_result)) => {
                        let result = MiningResult::from(c_result);
                        
                        if result.success {
                            // Update block with found nonce
                            block.header.nonce = result.nonce as u64;
                            block.header.hash = result.block_hash;
                            
                            info!("Successfully mined block {} with nonce {} (job: {})", 
                                  hex::encode(result.block_hash), result.nonce, job_id);
                            
                            Ok((result, block))
                        } else {
                            Err(BlockchainError::InvalidInput("Mining failed to find valid nonce".to_string()))
                        }
                    }
                    Ok(Err(e)) => Err(e),
                    Err(e) => Err(BlockchainError::InvalidInput(format!("Mining task panicked: {}", e))),
                }
            }
        }
    }

    /// Create coinbase transaction for mining reward
    fn create_coinbase_transaction(
        reward: u64,
        miner_address: &str,
        block_height: u64,
    ) -> Result<Transaction> {
        use crate::transaction::TransactionOutput;
        
        // Create coinbase output
        let coinbase_output = TransactionOutput::create_p2pkh(reward, miner_address)?;
        
        // Create coinbase transaction (no inputs, just the reward output)
        let mut transaction = Transaction::new(1, Vec::new(), vec![coinbase_output]);
        
        // Add block height to transaction for uniqueness
        transaction.lock_time = block_height as u32;
        
        Ok(transaction)
    }

    /// Verify proof of work using C++ FFI
    pub async fn verify_proof_of_work(
        block: &Block,
    ) -> Result<bool> {
        let block_data = format!(
            "{}{}{}{}{}{}",
            block.header.version,
            hex::encode(block.header.previous_hash),
            hex::encode(block.header.merkle_root),
            block.header.timestamp,
            block.header.difficulty_target,
            block.header.nonce
        );

        let block_data_cstr = std::ffi::CString::new(block_data)
            .map_err(|e| BlockchainError::InvalidInput(format!("Invalid block data: {}", e)))?;
        
        let is_valid = tokio::task::spawn_blocking(move || {
            unsafe {
                c_verify_proof_of_work(
                    block_data_cstr.as_ptr(),
                    block.header.nonce as u32,
                    block.header.difficulty_target,
                )
            }
        }).await
        .map_err(|e| BlockchainError::InvalidInput(format!("Task panicked: {}", e)))?;

        Ok(is_valid)
    }

    /// Calculate next difficulty using C++ FFI
    pub async fn calculate_next_difficulty(
        current_difficulty: u32,
        actual_time_span: Duration,
        target_time_span: Duration,
    ) -> Result<u32> {
        let actual_seconds = actual_time_span.as_secs();
        let target_seconds = target_time_span.as_secs();

        let new_difficulty = tokio::task::spawn_blocking(move || {
            unsafe {
                c_calculate_next_difficulty(current_difficulty, actual_seconds, target_seconds)
            }
        }).await
        .map_err(|e| BlockchainError::InvalidInput(format!("Task panicked: {}", e)))?;

        Ok(new_difficulty)
    }

    /// Check if difficulty should be adjusted at given height
    pub fn should_adjust_difficulty(block_height: u64) -> bool {
        unsafe { c_should_adjust_difficulty(block_height as u32) }
    }
}

impl Default for MiningController {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mining_controller_creation() {
        let controller = MiningController::new();
        let config = controller.get_config().await;
        
        assert_eq!(config.difficulty_target, 0x1d00ffff);
        assert!(!controller.is_mining().await);
    }

    #[tokio::test]
    async fn test_mining_config_update() {
        let controller = MiningController::new();
        
        let mut new_config = MiningConfig::default();
        new_config.difficulty_target = 0x1e00ffff;
        
        controller.update_config(new_config.clone()).await.unwrap();
        
        let updated_config = controller.get_config().await;
        assert_eq!(updated_config.difficulty_target, 0x1e00ffff);
    }

    #[test]
    fn test_difficulty_check() {
        // Test difficulty adjustment intervals
        assert!(!MiningController::should_adjust_difficulty(1));
        assert!(!MiningController::should_adjust_difficulty(2015));
        // Note: Actual adjustment logic depends on C++ implementation
    }
}