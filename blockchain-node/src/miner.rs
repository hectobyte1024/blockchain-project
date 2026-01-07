//! Mining Daemon - Proof of Work block production
//!
//! This module implements the mining loop that:
//! - Selects pending transactions from mempool
//! - Creates block templates
//! - Performs PoW mining (nonce search)
//! - Submits mined blocks to consensus
//! - Awards mining rewards

use std::sync::Arc;
use tokio::time::{Duration, sleep};
use tracing::{info, warn, error, debug};
use blockchain_core::{
    block::{Block, BlockHeader},
    Hash256,
};
use crate::blockchain::BlockchainBackend;

/// Mining statistics
#[derive(Debug, Clone)]
pub struct MiningStats {
    pub blocks_mined: u64,
    pub hashes_computed: u64,
    pub mining_since: std::time::Instant,
    pub last_block_time: Option<std::time::Instant>,
}

impl Default for MiningStats {
    fn default() -> Self {
        Self {
            blocks_mined: 0,
            hashes_computed: 0,
            mining_since: std::time::Instant::now(),
            last_block_time: None,
        }
    }
}

/// Mining daemon that runs in background
pub struct MiningDaemon {
    blockchain: Arc<BlockchainBackend>,
    validator_address: String,
    stats: Arc<tokio::sync::RwLock<MiningStats>>,
    should_stop: Arc<tokio::sync::RwLock<bool>>,
}

impl MiningDaemon {
    /// Create new mining daemon
    pub fn new(blockchain: Arc<BlockchainBackend>, validator_address: String) -> Self {
        Self {
            blockchain,
            validator_address,
            stats: Arc::new(tokio::sync::RwLock::new(MiningStats::default())),
            should_stop: Arc::new(tokio::sync::RwLock::new(false)),
        }
    }
    
    /// Start mining in background task
    pub fn start(self) -> tokio::task::JoinHandle<()> {
        info!("⛏️  Starting mining daemon for validator: {}", self.validator_address);
        
        tokio::spawn(async move {
            self.mining_loop().await;
        })
    }
    
    /// Get current mining statistics
    pub async fn get_stats(&self) -> MiningStats {
        self.stats.read().await.clone()
    }
    
    /// Stop mining daemon
    pub async fn stop(&self) {
        let mut should_stop = self.should_stop.write().await;
        *should_stop = true;
        info!("🛑 Mining daemon stopping...");
    }
    
    /// Main mining loop
    async fn mining_loop(&self) {
        info!("⛏️  Mining loop started");
        
        loop {
            // Check if we should stop
            {
                let should_stop = self.should_stop.read().await;
                if *should_stop {
                    info!("✅ Mining daemon stopped");
                    break;
                }
            }
            
            // Try to mine a block
            match self.mine_one_block().await {
                Ok(true) => {
                    // Successfully mined a block
                    let mut stats = self.stats.write().await;
                    stats.blocks_mined += 1;
                    stats.last_block_time = Some(std::time::Instant::now());
                    
                    let elapsed = stats.mining_since.elapsed().as_secs();
                    let rate = if elapsed > 0 {
                        stats.blocks_mined as f64 / elapsed as f64
                    } else {
                        0.0
                    };
                    
                    info!("✨ Block mined! Total: {} | Rate: {:.3} blocks/sec", 
                          stats.blocks_mined, rate);
                    
                    // Brief pause before next block
                    sleep(Duration::from_millis(100)).await;
                }
                Ok(false) => {
                    // No transactions to mine, wait a bit
                    debug!("No transactions in mempool, waiting...");
                    sleep(Duration::from_secs(5)).await;
                }
                Err(e) => {
                    error!("Mining error: {}", e);
                    sleep(Duration::from_secs(2)).await;
                }
            }
        }
    }
    
    /// Mine a single block
    /// Returns Ok(true) if block was mined, Ok(false) if no work to do
    async fn mine_one_block(&self) -> Result<bool, anyhow::Error> {
        // Get current blockchain state
        let height = self.blockchain.get_height().await;
        let next_height = height + 1;
        
        info!("Attempting to mine block at height {}", next_height);
        
        // Get mempool stats to check if there are transactions
        let mempool_stats = self.blockchain.get_mempool_stats().await;
        let tx_count = mempool_stats.get("transactions")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        
        info!("Mempool has {} transactions", tx_count);
        
        // Get transactions from mempool
        let pending_txs = self.blockchain.get_pending_transactions().await;
        info!("Retrieved {} transactions from mempool for mining", pending_txs.len());
        
        // Allow mining empty blocks if needed (no transaction limit)
        // Mining will continue even with empty blocks to maintain chain progression
        
        // Create block template
        let block_template = self.create_block_template(next_height).await?;
        
        // Mine the block (Proof of Work)
        let mined_block = self.mine_block(block_template).await?;
        
        // Submit to consensus
        self.submit_block(mined_block).await?;
        
        Ok(true)
    }
    
    /// Create coinbase transaction for mining reward
    fn create_coinbase_transaction(&self, height: u64) -> Result<blockchain_core::transaction::Transaction, anyhow::Error> {
        use blockchain_core::transaction::{Transaction, TransactionInput, TransactionOutput};
        
        // Coinbase reward: 50 EDU (50,000,000 satoshis)
        let block_reward = 50_000_000_u64;
        
        // Create coinbase input (includes block height in script_sig)
        let coinbase_data = format!("Block Height: {}", height).into_bytes();
        let coinbase_input = TransactionInput::create_coinbase(coinbase_data);
        
        // Create output to validator address
        let coinbase_output = TransactionOutput::create_p2pkh(
            block_reward,
            &self.validator_address
        )?;
        
        // Create coinbase transaction using Transaction::new
        let tx = Transaction::new(
            1, // version
            vec![coinbase_input],
            vec![coinbase_output]
        );
        
        Ok(tx)
    }
    
    /// Create a block template ready for mining
    async fn create_block_template(&self, height: u64) -> Result<Block, anyhow::Error> {
        // Get chain state
        let chain_state = self.blockchain.consensus.get_chain_state().await;
        
        // Get pending transactions from mempool
        let mut transactions = self.blockchain.get_pending_transactions().await;
        
        // Create coinbase transaction (mining reward)
        let coinbase_tx = self.create_coinbase_transaction(height)?;
        
        // Coinbase must be first transaction
        transactions.insert(0, coinbase_tx);
        
        // Calculate merkle root
        let merkle_root = if transactions.is_empty() {
            Hash256::default()
        } else {
            let tx_hashes: Vec<Hash256> = transactions.iter()
                .map(|tx| tx.calculate_hash())
                .collect();
            Block::compute_merkle_root(tx_hashes)
        };
        
        info!("Block template: {} transactions, merkle root: {}", 
              transactions.len(), hex::encode(&merkle_root));
        
        // Get previous block hash
        let prev_hash = if height > 0 {
            self.blockchain.get_block_by_height(height - 1)
                .await
                .map(|b| b.header.calculate_hash())
                .unwrap_or_default()
        } else {
            Hash256::default()
        };
        
        // Create block header
        let header = BlockHeader::new(
            1, // version
            prev_hash,
            merkle_root,
            chain_state.next_difficulty,
            height as u32,
        );
        
        Ok(Block::new(header, transactions))
    }
    
    /// Perform Proof of Work mining on a block
    async fn mine_block(&self, mut block: Block) -> Result<Block, anyhow::Error> {
        let target = block.header.difficulty_target;
        let start_time = std::time::Instant::now();
        
        debug!("⛏️  Mining block at height {} with difficulty {}", 
               block.header.height, target);
        
        // Mine with nonce search
        let mut nonce = 0u32;
        let mut hashes = 0u64;
        
        loop {
            block.header.nonce = nonce;
            hashes += 1;
            
            // Calculate hash
            let hash = block.header.calculate_hash();
            
            // Check if hash meets difficulty target
            if Self::hash_meets_target(&hash, target) {
                let elapsed = start_time.elapsed();
                let hash_rate = if elapsed.as_secs() > 0 {
                    hashes as f64 / elapsed.as_secs() as f64
                } else {
                    hashes as f64
                };
                
                info!("⛏️  Block mined! Height: {} | Nonce: {} | Hashes: {} | Rate: {:.0} H/s | Time: {:.2}s",
                      block.header.height, nonce, hashes, hash_rate, elapsed.as_secs_f64());
                
                // Update stats
                {
                    let mut stats = self.stats.write().await;
                    stats.hashes_computed += hashes;
                }
                
                return Ok(block);
            }
            
            nonce = nonce.wrapping_add(1);
            
            // Periodically check if we should stop
            if nonce % 10000 == 0 {
                let should_stop = self.should_stop.read().await;
                if *should_stop {
                    return Err(anyhow::anyhow!("Mining stopped"));
                }
                
                // Yield to allow other tasks to run
                tokio::task::yield_now().await;
            }
            
            // Log progress every 100k hashes
            if hashes % 100000 == 0 {
                debug!("Mining progress: {} hashes, nonce: {}", hashes, nonce);
            }
        }
    }
    
    /// Check if hash meets the difficulty target
    fn hash_meets_target(hash: &Hash256, target: u32) -> bool {
        // Simplified difficulty for development:
        // Just check if hash is less than a threshold
        // Target 0x1d00ffff means very easy mining - require just 1 leading zero byte
        
        // For development: require first byte < target threshold
        // Higher threshold = easier mining
        let threshold = if target > 0x1d000000 {
            255u8 // Very easy - almost all hashes pass
        } else if target > 0x1c000000 {
            128
        } else if target > 0x1b000000 {
            64
        } else {
            16 // Harder
        };
        
        hash[0] < threshold
    }
    
    /// Submit mined block to consensus
    async fn submit_block(&self, block: Block) -> Result<(), anyhow::Error> {
        info!("📦 Submitting mined block at height {}", block.header.height);
        
        // Validate the block
        let validation = self.blockchain.consensus.validate_block(&block).await?;
        
        match validation {
            blockchain_core::consensus::BlockValidation::Valid => {
                info!("✅ Block validation passed");
            }
            blockchain_core::consensus::BlockValidation::Invalid(reason) => {
                warn!("⚠️  Mined block failed validation: {}", reason);
                return Err(anyhow::anyhow!("Block validation failed: {}", reason));
            }
            blockchain_core::consensus::BlockValidation::OrphanBlock(parent_hash) => {
                warn!("⚠️  Mined block is orphan, missing parent: {:?}", parent_hash);
                return Err(anyhow::anyhow!("Block is orphan"));
            }
        }
        
        // Add block to consensus
        self.blockchain.consensus.add_block(block.clone()).await?;
        
        // Sync UTXO set from consensus to backend after adding block
        {
            let consensus_utxo_set = self.blockchain.consensus.get_utxo_set().await;
            let mut backend_utxo_set = self.blockchain.utxo_set.write().await;
            *backend_utxo_set = consensus_utxo_set;
        }
        
        info!("✅ Block added to blockchain at height {}", block.header.height);
        
        // TODO: Broadcast block to network once P2P is wired
        // self.blockchain.network.broadcast_block(block).await?;
        
        Ok(())
    }
}

/// Get mining statistics as JSON
pub async fn get_mining_info(daemon: &MiningDaemon) -> serde_json::Value {
    let stats = daemon.get_stats().await;
    let elapsed = stats.mining_since.elapsed();
    
    let avg_block_time = if stats.blocks_mined > 0 {
        elapsed.as_secs_f64() / stats.blocks_mined as f64
    } else {
        0.0
    };
    
    let hash_rate = if elapsed.as_secs() > 0 {
        stats.hashes_computed as f64 / elapsed.as_secs() as f64
    } else {
        0.0
    };
    
    serde_json::json!({
        "blocks_mined": stats.blocks_mined,
        "hashes_computed": stats.hashes_computed,
        "mining_duration_secs": elapsed.as_secs(),
        "average_block_time_secs": avg_block_time,
        "hash_rate": hash_rate,
        "last_block_ago_secs": stats.last_block_time.map(|t| t.elapsed().as_secs()),
    })
}
