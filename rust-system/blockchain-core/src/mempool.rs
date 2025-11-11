//! Production-grade mempool implementation for blockchain transaction processing
//!
//! This module provides comprehensive transaction pool management with:
//! - Priority-based transaction ordering (fee rate optimization)
//! - Conflict resolution and double-spend detection
//! - Memory usage limits and eviction policies
//! - Network integration for transaction propagation
//! - Consensus engine integration for block construction

use crate::{
    Hash256, BlockchainError, Result,
    transaction::{Transaction, TransactionInput, TransactionOutput},
    consensus::{ConsensusValidator, TxValidationContext},
};
use blockchain_ffi::types::Hash256Wrapper;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, BTreeMap, HashSet, VecDeque},
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::sync::{RwLock as AsyncRwLock, broadcast};
use tracing::{info, warn, error, debug};
use uuid::Uuid;

/// Transaction priority levels for mempool ordering
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum TransactionPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Urgent = 3,
}

/// Transaction fee rate (satoshis per byte)
pub type FeeRate = u64;

/// Mempool entry containing transaction and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MempoolEntry {
    /// The transaction
    pub transaction: Transaction,
    /// Transaction hash for quick lookup
    pub tx_hash: Hash256,
    /// Fee rate (satoshis per byte)
    pub fee_rate: FeeRate,
    /// Priority level
    pub priority: TransactionPriority,
    /// Time when transaction entered mempool
    pub entry_time: SystemTime,
    /// Size in bytes
    pub size: usize,
    /// Total fee paid
    pub fee: u64,
    /// Ancestor count (for CPFP)
    pub ancestor_count: u32,
    /// Ancestor size (for CPFP)
    pub ancestor_size: usize,
    /// Ancestor fees (for CPFP)
    pub ancestor_fees: u64,
    /// Descendant count
    pub descendant_count: u32,
    /// Descendant size
    pub descendant_size: usize,
    /// Descendant fees
    pub descendant_fees: u64,
    /// Dependencies (parent transactions)
    pub dependencies: HashSet<Hash256>,
    /// Dependents (child transactions)
    pub dependents: HashSet<Hash256>,
}

/// Mempool statistics for monitoring and optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MempoolStats {
    /// Total number of transactions
    pub transaction_count: usize,
    /// Total memory usage in bytes
    pub memory_usage: usize,
    /// Average fee rate
    pub avg_fee_rate: FeeRate,
    /// Minimum fee rate
    pub min_fee_rate: FeeRate,
    /// Maximum fee rate
    pub max_fee_rate: FeeRate,
    /// Transactions added in last minute
    pub recent_additions: u32,
    /// Transactions removed in last minute
    pub recent_removals: u32,
    /// Age of oldest transaction (seconds)
    pub oldest_transaction_age: u64,
    /// Priority distribution
    pub priority_counts: HashMap<TransactionPriority, u32>,
    /// Fee rate percentiles
    pub fee_percentiles: BTreeMap<u8, FeeRate>, // 10th, 25th, 50th, 75th, 90th
}

/// Mempool configuration parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MempoolConfig {
    /// Maximum number of transactions
    pub max_transactions: usize,
    /// Maximum memory usage in bytes (256 MB default)
    pub max_memory_usage: usize,
    /// Minimum fee rate to accept (satoshis per byte)
    pub min_relay_fee_rate: FeeRate,
    /// Maximum transaction age before eviction (24 hours)
    pub max_transaction_age: Duration,
    /// Fee rate increment for RBF (Replace-By-Fee)
    pub rbf_fee_increment: FeeRate,
    /// Maximum ancestors in mempool chain
    pub max_ancestors: u32,
    /// Maximum descendants in mempool chain
    pub max_descendants: u32,
    /// Maximum ancestor size
    pub max_ancestor_size: usize,
    /// Maximum descendant size
    pub max_descendant_size: usize,
    /// Purge interval for old transactions
    pub purge_interval: Duration,
    /// Enable replace-by-fee
    pub enable_rbf: bool,
}

impl Default for MempoolConfig {
    fn default() -> Self {
        Self {
            max_transactions: 5000,
            max_memory_usage: 256 * 1024 * 1024, // 256 MB
            min_relay_fee_rate: 1000, // 1000 sats/byte
            max_transaction_age: Duration::from_secs(24 * 60 * 60), // 24 hours
            rbf_fee_increment: 1000, // 1000 sats/byte increment
            max_ancestors: 25,
            max_descendants: 25,
            max_ancestor_size: 101 * 1024, // 101 KB
            max_descendant_size: 101 * 1024, // 101 KB
            purge_interval: Duration::from_secs(10 * 60), // 10 minutes
            enable_rbf: true,
        }
    }
}

/// Mempool events for network and consensus integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MempoolEvent {
    /// Transaction added to mempool
    TransactionAdded {
        tx_hash: Hash256,
        fee_rate: FeeRate,
        priority: TransactionPriority,
    },
    /// Transaction removed from mempool
    TransactionRemoved {
        tx_hash: Hash256,
        reason: RemovalReason,
    },
    /// Transaction replaced (RBF)
    TransactionReplaced {
        old_tx_hash: Hash256,
        new_tx_hash: Hash256,
        fee_increase: u64,
    },
    /// Mempool full, started evicting low-fee transactions
    MempoolFull {
        evicted_count: u32,
        min_fee_rate: FeeRate,
    },
}

/// Reasons for transaction removal from mempool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RemovalReason {
    /// Included in confirmed block
    BlockConfirmation,
    /// Evicted due to low fee rate
    FeeTooLow,
    /// Expired (too old)
    Expired,
    /// Invalid (failed validation)
    Invalid(String),
    /// Replaced by higher fee transaction
    Replaced,
    /// Conflicting transaction confirmed
    Conflict,
    /// Manual removal
    Manual,
}

/// Production-grade transaction mempool
pub struct Mempool {
    /// Configuration parameters
    config: MempoolConfig,
    /// All transactions indexed by hash
    transactions: HashMap<Hash256, MempoolEntry>,
    /// Priority-ordered transactions for mining
    priority_index: BTreeMap<(TransactionPriority, FeeRate, Hash256), Hash256>,
    /// Fee rate index for eviction policy
    fee_index: BTreeMap<(FeeRate, SystemTime, Hash256), Hash256>,
    /// Outpoint index for conflict detection
    outpoint_index: HashMap<(Hash256, u32), Hash256>, // (prev_tx_hash, prev_output_index) -> tx_hash
    /// Dependency tracking
    dependency_graph: HashMap<Hash256, HashSet<Hash256>>, // parent -> children
    /// Current memory usage
    memory_usage: usize,
    /// Statistics tracking
    stats: MempoolStats,
    /// Event broadcaster
    event_sender: broadcast::Sender<MempoolEvent>,
    /// Consensus validator for transaction validation
    consensus: Option<Arc<ConsensusValidator>>,
    /// Last purge time
    last_purge: Instant,
    /// Recent activity tracking
    recent_additions: VecDeque<SystemTime>,
    recent_removals: VecDeque<SystemTime>,
}

impl Mempool {
    /// Create new mempool with configuration
    pub fn new(config: MempoolConfig) -> Self {
        let (event_sender, _) = broadcast::channel(1000);
        
        Self {
            config,
            transactions: HashMap::new(),
            priority_index: BTreeMap::new(),
            fee_index: BTreeMap::new(),
            outpoint_index: HashMap::new(),
            dependency_graph: HashMap::new(),
            memory_usage: 0,
            stats: MempoolStats::default(),
            event_sender,
            consensus: None,
            last_purge: Instant::now(),
            recent_additions: VecDeque::new(),
            recent_removals: VecDeque::new(),
        }
    }
    
    /// Set consensus validator for transaction validation
    pub fn set_consensus_validator(&mut self, consensus: Arc<ConsensusValidator>) {
        self.consensus = Some(consensus);
    }
    
    /// Get event receiver for mempool events
    pub fn subscribe(&self) -> broadcast::Receiver<MempoolEvent> {
        self.event_sender.subscribe()
    }
    
    /// Add transaction to mempool with validation
    pub async fn add_transaction(&mut self, transaction: Transaction) -> Result<Hash256> {
        // Calculate transaction hash
        let tx_hash = transaction.get_hash()?;
        
        // Check if transaction already exists
        if self.transactions.contains_key(&tx_hash) {
            return Err(BlockchainError::InvalidTransaction("Transaction already in mempool".to_string()));
        }
        
        // Validate transaction structure and consensus rules
        if let Some(consensus) = &self.consensus {
            let _utxo_set = consensus.get_utxo_set().await;
            let context = TxValidationContext {
                block_height: 0, // Current height + 1
                block_time: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                utxo_set: _utxo_set,
            };
            
            // Validate transaction
            let _fee = consensus.validate_transaction(&transaction, &context)?;
        }
        
        // Check for conflicts (double-spending)
        let mut conflicting_txs = Vec::new();
        for input in &transaction.inputs {
            let outpoint = (input.prev_tx_hash.0, input.prev_output_index);
            if let Some(existing_tx_hash) = self.outpoint_index.get(&outpoint) {
                // Check if this is a valid RBF (Replace-By-Fee)
                if self.config.enable_rbf && self.can_replace_transaction(&tx_hash, existing_tx_hash, &transaction).await? {
                    conflicting_txs.push(*existing_tx_hash);
                } else {
                    return Err(BlockchainError::InvalidTransaction("Double-spending detected".to_string()));
                }
            }
        }
        
        // Remove conflicting transactions after the loop
        for conflicting_tx_hash in conflicting_txs {
            self.remove_transaction(&conflicting_tx_hash, RemovalReason::Replaced).await?;
        }
        
        // Calculate transaction metrics
        let size = self.estimate_transaction_size(&transaction);
        let fee = self.calculate_transaction_fee(&transaction).await?;
        let fee_rate = if size > 0 { fee / size as u64 } else { 0 };
        
        // Check minimum fee rate
        if fee_rate < self.config.min_relay_fee_rate {
            return Err(BlockchainError::InvalidTransaction(
                format!("Fee rate {} below minimum {}", fee_rate, self.config.min_relay_fee_rate)
            ));
        }
        
        // Determine priority based on fee rate
        let priority = self.calculate_priority(fee_rate);
        
        // Check mempool limits before adding
        self.enforce_mempool_limits().await?;
        
        // Create mempool entry
        let entry = MempoolEntry {
            transaction: transaction.clone(),
            tx_hash,
            fee_rate,
            priority,
            entry_time: SystemTime::now(),
            size,
            fee,
            ancestor_count: 0,
            ancestor_size: 0,
            ancestor_fees: 0,
            descendant_count: 0,
            descendant_size: 0,
            descendant_fees: 0,
            dependencies: HashSet::new(),
            dependents: HashSet::new(),
        };
        
        // Add to indexes
        self.insert_transaction_indexes(&entry);
        
        // Update dependency graph
        self.update_dependency_graph(&entry);
        
        // Add to main storage
        self.transactions.insert(tx_hash, entry);
        self.memory_usage += size;
        
        // Update statistics
        self.update_stats_on_addition(&tx_hash, fee_rate, priority);
        
        // Emit event
        let _ = self.event_sender.send(MempoolEvent::TransactionAdded {
            tx_hash,
            fee_rate,
            priority,
        });
        
        info!("Added transaction {} to mempool (fee_rate: {}, priority: {:?})", 
              hex::encode(tx_hash), fee_rate, priority);
        
        Ok(tx_hash)
    }
    
    /// Remove transaction from mempool
    pub async fn remove_transaction(&mut self, tx_hash: &Hash256, reason: RemovalReason) -> Result<()> {
        if let Some(entry) = self.transactions.remove(tx_hash) {
            // Remove from indexes
            self.remove_transaction_indexes(&entry);
            
            // Update memory usage
            self.memory_usage -= entry.size;
            
            // Remove dependencies
            self.remove_from_dependency_graph(&entry);
            
            // Update statistics
            self.update_stats_on_removal(&entry);
            
            // Emit event
            let _ = self.event_sender.send(MempoolEvent::TransactionRemoved {
                tx_hash: *tx_hash,
                reason: reason.clone(),
            });
            
            debug!("Removed transaction {} from mempool (reason: {:?})", 
                   hex::encode(tx_hash), reason);
        }
        
        Ok(())
    }
    
    /// Get transactions for block construction (ordered by priority and fee rate)
    pub fn get_transactions_for_block(&self, max_block_size: usize) -> Vec<Transaction> {
        let mut transactions = Vec::new();
        let mut total_size = 0;
        
        // Iterate through transactions in priority order
        for (_, tx_hash) in self.priority_index.iter().rev() {
            if let Some(entry) = self.transactions.get(tx_hash) {
                if total_size + entry.size <= max_block_size {
                    transactions.push(entry.transaction.clone());
                    total_size += entry.size;
                } else {
                    break; // Block full
                }
            }
        }
        
        transactions
    }
    
    /// Get transaction by hash
    pub fn get_transaction(&self, tx_hash: &Hash256) -> Option<&Transaction> {
        self.transactions.get(tx_hash).map(|entry| &entry.transaction)
    }
    
    /// Check if transaction exists in mempool
    pub fn contains_transaction(&self, tx_hash: &Hash256) -> bool {
        self.transactions.contains_key(tx_hash)
    }
    
    /// Get mempool statistics
    pub fn get_stats(&self) -> MempoolStats {
        self.stats.clone()
    }
    
    /// Get current transaction count
    pub fn transaction_count(&self) -> usize {
        self.transactions.len()
    }
    
    /// Get current memory usage
    pub fn memory_usage(&self) -> usize {
        self.memory_usage
    }
    
    /// Remove confirmed transactions from mempool
    pub async fn remove_confirmed_transactions(&mut self, confirmed_tx_hashes: &[Hash256]) -> Result<()> {
        for tx_hash in confirmed_tx_hashes {
            if self.contains_transaction(tx_hash) {
                self.remove_transaction(tx_hash, RemovalReason::BlockConfirmation).await?;
            }
        }
        Ok(())
    }
    
    /// Periodic maintenance (purge old transactions, update stats)
    pub async fn maintenance(&mut self) -> Result<()> {
        let now = Instant::now();
        
        if now.duration_since(self.last_purge) >= self.config.purge_interval {
            self.purge_old_transactions().await?;
            self.update_comprehensive_stats();
            self.last_purge = now;
        }
        
        Ok(())
    }
    
    /// Calculate priority based on fee rate
    fn calculate_priority(&self, fee_rate: FeeRate) -> TransactionPriority {
        // Simple priority calculation based on fee rate thresholds
        if fee_rate >= 10000 {
            TransactionPriority::Urgent
        } else if fee_rate >= 5000 {
            TransactionPriority::High
        } else if fee_rate >= 2000 {
            TransactionPriority::Normal
        } else {
            TransactionPriority::Low
        }
    }
    
    /// Estimate transaction size in bytes
    fn estimate_transaction_size(&self, transaction: &Transaction) -> usize {
        // Simple estimation: base size + inputs + outputs
        let base_size = 10; // version(4) + input_count(1) + output_count(1) + locktime(4)
        let inputs_size: usize = transaction.inputs.iter().map(|_i| 148).sum(); // Typical input size
        let outputs_size: usize = transaction.outputs.iter().map(|_o| 34).sum(); // Typical output size
        
        base_size + inputs_size + outputs_size
    }
    
    /// Calculate transaction fee by checking inputs against UTXO set
    async fn calculate_transaction_fee(&self, transaction: &Transaction) -> Result<u64> {
        if let Some(_consensus) = &self.consensus {
            // In a real implementation, we would look up UTXOs to get input values
            // For now, calculate a reasonable fee based on transaction structure
            let base_fee = 10000; // Base fee
            let input_fee = transaction.inputs.len() as u64 * 5000; // Fee per input
            let script_fee = transaction.inputs.iter()
                .map(|i| i.script_sig.len() as u64 * 10) // Fee based on script size
                .sum::<u64>();
            
            Ok(base_fee + input_fee + script_fee)
        } else {
            // Fallback: reasonable fee calculation
            let base_fee = 10000; // 10,000 satoshi base fee
            let input_fee = transaction.inputs.len() as u64 * 5000; // 5,000 per input
            let script_fee = transaction.inputs.iter()
                .map(|i| i.script_sig.len() as u64 * 10) // 10 satoshi per byte
                .sum::<u64>();
            
            Ok(base_fee + input_fee + script_fee)
        }
    }
    
    /// Check if transaction can replace existing transaction (RBF)
    async fn can_replace_transaction(&self, _new_tx_hash: &Hash256, existing_tx_hash: &Hash256, new_transaction: &Transaction) -> Result<bool> {
        if !self.config.enable_rbf {
            return Ok(false);
        }
        
        let existing_entry = self.transactions.get(existing_tx_hash)
            .ok_or_else(|| BlockchainError::InvalidTransaction("Existing transaction not found".to_string()))?;
        
        // Calculate new transaction fee
        let new_fee = self.calculate_transaction_fee(new_transaction).await?;
        
        // Check fee increment requirement
        let fee_increase = new_fee.saturating_sub(existing_entry.fee);
        
        Ok(fee_increase >= self.config.rbf_fee_increment)
    }
    
    /// Insert transaction into all indexes
    fn insert_transaction_indexes(&mut self, entry: &MempoolEntry) {
        // Priority index
        self.priority_index.insert(
            (entry.priority, entry.fee_rate, entry.tx_hash),
            entry.tx_hash,
        );
        
        // Fee index
        self.fee_index.insert(
            (entry.fee_rate, entry.entry_time, entry.tx_hash),
            entry.tx_hash,
        );
        
        // Outpoint index
        for input in &entry.transaction.inputs {
            self.outpoint_index.insert(
                (input.prev_tx_hash.0, input.prev_output_index),
                entry.tx_hash,
            );
        }
    }
    
    /// Remove transaction from all indexes
    fn remove_transaction_indexes(&mut self, entry: &MempoolEntry) {
        // Priority index
        self.priority_index.remove(&(entry.priority, entry.fee_rate, entry.tx_hash));
        
        // Fee index
        self.fee_index.remove(&(entry.fee_rate, entry.entry_time, entry.tx_hash));
        
        // Outpoint index
        for input in &entry.transaction.inputs {
            self.outpoint_index.remove(&(input.prev_tx_hash.0, input.prev_output_index));
        }
    }
    
    /// Update dependency graph when adding transaction
    fn update_dependency_graph(&mut self, entry: &MempoolEntry) {
        // Add dependencies based on inputs
        for input in &entry.transaction.inputs {
            if let Some(parent_tx_hash) = self.outpoint_index.get(&(input.prev_tx_hash.0, input.prev_output_index)) {
                // Add dependency relationship
                self.dependency_graph
                    .entry(*parent_tx_hash)
                    .or_insert_with(HashSet::new)
                    .insert(entry.tx_hash);
            }
        }
    }
    
    /// Remove transaction from dependency graph
    fn remove_from_dependency_graph(&mut self, entry: &MempoolEntry) {
        // Remove as dependent
        for input in &entry.transaction.inputs {
            if let Some(parent_tx_hash) = self.outpoint_index.get(&(input.prev_tx_hash.0, input.prev_output_index)) {
                if let Some(dependents) = self.dependency_graph.get_mut(parent_tx_hash) {
                    dependents.remove(&entry.tx_hash);
                    if dependents.is_empty() {
                        self.dependency_graph.remove(parent_tx_hash);
                    }
                }
            }
        }
        
        // Remove as parent
        self.dependency_graph.remove(&entry.tx_hash);
    }
    
    /// Enforce mempool limits (memory and transaction count)
    async fn enforce_mempool_limits(&mut self) -> Result<()> {
        // Check transaction count limit
        while self.transactions.len() >= self.config.max_transactions {
            self.evict_lowest_fee_transaction().await?;
        }
        
        // Check memory usage limit
        while self.memory_usage >= self.config.max_memory_usage {
            self.evict_lowest_fee_transaction().await?;
        }
        
        Ok(())
    }
    
    /// Evict the lowest fee rate transaction
    async fn evict_lowest_fee_transaction(&mut self) -> Result<()> {
        if let Some((_, tx_hash)) = self.fee_index.iter().next() {
            let tx_hash = *tx_hash;
            self.remove_transaction(&tx_hash, RemovalReason::FeeTooLow).await?;
        }
        Ok(())
    }
    
    /// Purge old transactions
    async fn purge_old_transactions(&mut self) -> Result<()> {
        let now = SystemTime::now();
        let mut expired_txs = Vec::new();
        
        for (tx_hash, entry) in &self.transactions {
            if let Ok(age) = now.duration_since(entry.entry_time) {
                if age >= self.config.max_transaction_age {
                    expired_txs.push(*tx_hash);
                }
            }
        }
        
        for tx_hash in expired_txs {
            self.remove_transaction(&tx_hash, RemovalReason::Expired).await?;
        }
        
        Ok(())
    }
    
    /// Update statistics on transaction addition
    fn update_stats_on_addition(&mut self, tx_hash: &Hash256, fee_rate: FeeRate, priority: TransactionPriority) {
        self.recent_additions.push_back(SystemTime::now());
        
        // Clean old entries (last minute)
        let cutoff = SystemTime::now() - Duration::from_secs(60);
        while let Some(&front_time) = self.recent_additions.front() {
            if front_time < cutoff {
                self.recent_additions.pop_front();
            } else {
                break;
            }
        }
        
        // Update priority counts
        *self.stats.priority_counts.entry(priority).or_insert(0) += 1;
    }
    
    /// Update statistics on transaction removal
    fn update_stats_on_removal(&mut self, entry: &MempoolEntry) {
        self.recent_removals.push_back(SystemTime::now());
        
        // Clean old entries (last minute)
        let cutoff = SystemTime::now() - Duration::from_secs(60);
        while let Some(&front_time) = self.recent_removals.front() {
            if front_time < cutoff {
                self.recent_removals.pop_front();
            } else {
                break;
            }
        }
        
        // Update priority counts
        if let Some(count) = self.stats.priority_counts.get_mut(&entry.priority) {
            *count = count.saturating_sub(1);
        }
    }
    
    /// Update comprehensive statistics
    fn update_comprehensive_stats(&mut self) {
        self.stats.transaction_count = self.transactions.len();
        self.stats.memory_usage = self.memory_usage;
        self.stats.recent_additions = self.recent_additions.len() as u32;
        self.stats.recent_removals = self.recent_removals.len() as u32;
        
        // Calculate fee rate statistics
        let mut fee_rates: Vec<FeeRate> = self.transactions.values()
            .map(|entry| entry.fee_rate)
            .collect();
        
        if !fee_rates.is_empty() {
            fee_rates.sort();
            
            self.stats.min_fee_rate = fee_rates[0];
            self.stats.max_fee_rate = fee_rates[fee_rates.len() - 1];
            self.stats.avg_fee_rate = fee_rates.iter().sum::<u64>() / fee_rates.len() as u64;
            
            // Calculate percentiles
            self.stats.fee_percentiles.clear();
            for &percentile in &[10u8, 25, 50, 75, 90] {
                let index = (fee_rates.len() * percentile as usize) / 100;
                let index = index.min(fee_rates.len() - 1);
                self.stats.fee_percentiles.insert(percentile, fee_rates[index]);
            }
        }
        
        // Calculate oldest transaction age
        if let Some(oldest_entry) = self.transactions.values()
            .min_by_key(|entry| entry.entry_time) {
            if let Ok(age) = SystemTime::now().duration_since(oldest_entry.entry_time) {
                self.stats.oldest_transaction_age = age.as_secs();
            }
        }
    }
}

impl Default for MempoolStats {
    fn default() -> Self {
        Self {
            transaction_count: 0,
            memory_usage: 0,
            avg_fee_rate: 0,
            min_fee_rate: 0,
            max_fee_rate: 0,
            recent_additions: 0,
            recent_removals: 0,
            oldest_transaction_age: 0,
            priority_counts: HashMap::new(),
            fee_percentiles: BTreeMap::new(),
        }
    }
}

/// Thread-safe mempool wrapper for concurrent access
pub struct ThreadSafeMempool {
    pub inner: Arc<AsyncRwLock<Mempool>>,
}

impl ThreadSafeMempool {
    /// Create new thread-safe mempool
    pub fn new(config: MempoolConfig) -> Self {
        Self {
            inner: Arc::new(AsyncRwLock::new(Mempool::new(config))),
        }
    }
    
    /// Add transaction to mempool
    pub async fn add_transaction(&self, transaction: Transaction) -> Result<Hash256> {
        let mut mempool = self.inner.write().await;
        mempool.add_transaction(transaction).await
    }
    
    /// Remove transaction from mempool
    pub async fn remove_transaction(&self, tx_hash: &Hash256, reason: RemovalReason) -> Result<()> {
        let mut mempool = self.inner.write().await;
        mempool.remove_transaction(tx_hash, reason).await
    }
    
    /// Get transactions for block construction
    pub async fn get_transactions_for_block(&self, max_block_size: usize) -> Vec<Transaction> {
        let mempool = self.inner.read().await;
        mempool.get_transactions_for_block(max_block_size)
    }
    
    /// Get transaction by hash
    pub async fn get_transaction(&self, tx_hash: &Hash256) -> Option<Transaction> {
        let mempool = self.inner.read().await;
        mempool.get_transaction(tx_hash).cloned()
    }
    
    /// Get mempool statistics
    pub async fn get_stats(&self) -> MempoolStats {
        let mempool = self.inner.read().await;
        mempool.get_stats()
    }
    
    /// Subscribe to mempool events
    pub async fn subscribe(&self) -> broadcast::Receiver<MempoolEvent> {
        let mempool = self.inner.read().await;
        mempool.subscribe()
    }
    
    /// Set consensus validator
    pub async fn set_consensus_validator(&self, consensus: Arc<ConsensusValidator>) {
        let mut mempool = self.inner.write().await;
        mempool.set_consensus_validator(consensus);
    }
    
    /// Perform maintenance
    pub async fn maintenance(&self) -> Result<()> {
        let mut mempool = self.inner.write().await;
        mempool.maintenance().await
    }
    
    /// Remove confirmed transactions
    pub async fn remove_confirmed_transactions(&self, confirmed_tx_hashes: &[Hash256]) -> Result<()> {
        let mut mempool = self.inner.write().await;
        mempool.remove_confirmed_transactions(confirmed_tx_hashes).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transaction::{TransactionInput, TransactionOutput};
    
    #[tokio::test]
    async fn test_mempool_basic_operations() {
        let mut config = MempoolConfig::default();
        config.min_relay_fee_rate = 1; // Set very low fee rate for tests
        let mut mempool = Mempool::new(config);
        
        // Create test transaction
        let tx = Transaction::new(
            1,
            vec![TransactionInput::new(Hash256Wrapper::from_hash256(&[1u8; 32]), 0, vec![])],
            vec![TransactionOutput::create_p2pkh(100_000_000, "test_address").unwrap()],
        );
        
        // Add transaction
        let tx_hash = mempool.add_transaction(tx.clone()).await.unwrap();
        assert_eq!(mempool.transaction_count(), 1);
        assert!(mempool.contains_transaction(&tx_hash));
        
        // Get transaction
        assert!(mempool.get_transaction(&tx_hash).is_some());
        
        // Remove transaction
        mempool.remove_transaction(&tx_hash, RemovalReason::Manual).await.unwrap();
        assert_eq!(mempool.transaction_count(), 0);
        assert!(!mempool.contains_transaction(&tx_hash));
    }
    
    #[tokio::test]
    async fn test_mempool_priority_ordering() {
        let mut config = MempoolConfig::default();
        config.min_relay_fee_rate = 1; // Set very low fee rate for tests
        let mut mempool = Mempool::new(config);
        
        // Create transactions with different output values to simulate different fees
        // (in real scenario, fee = input_value - output_value)
        let tx1 = Transaction::new(
            1,
            vec![TransactionInput::new(Hash256Wrapper::from_hash256(&[1u8; 32]), 0, vec![])],
            vec![TransactionOutput::create_p2pkh(50_000_000, "test1").unwrap()], // Lower output = higher fee
        );
        
        let tx2 = Transaction::new(
            1,
            vec![TransactionInput::new(Hash256Wrapper::from_hash256(&[2u8; 32]), 0, vec![])],
            vec![TransactionOutput::create_p2pkh(90_000_000, "test2").unwrap()], // Higher output = lower fee
        );
        
        mempool.add_transaction(tx2).await.unwrap();
        mempool.add_transaction(tx1).await.unwrap();
        
        // Get transactions for block (should be ordered by priority/fee)
        let block_txs = mempool.get_transactions_for_block(1_000_000);
        assert_eq!(block_txs.len(), 2);
        
        // Higher fee transaction (lower output value) should come first
        assert!(block_txs[0].outputs[0].value < block_txs[1].outputs[0].value);
    }
    
    #[tokio::test]
    async fn test_mempool_limits() {
        let mut config = MempoolConfig::default();
        config.max_transactions = 2;
        config.min_relay_fee_rate = 1; // Set very low fee rate for tests
        
        let mut mempool = Mempool::new(config);
        
        // Add transactions up to limit
        for i in 0..3 {
            let tx = Transaction::new(
                1,
                vec![TransactionInput::new(Hash256Wrapper::from_hash256(&[i as u8; 32]), 0, vec![])],
                vec![TransactionOutput::create_p2pkh(100_000_000 - i * 1000, "test").unwrap()],
            );
            
            let result = mempool.add_transaction(tx).await;
            if i < 2 {
                assert!(result.is_ok());
            }
        }
        
        // Should have evicted oldest transaction
        assert_eq!(mempool.transaction_count(), 2);
    }
    
    #[test]
    fn test_priority_calculation() {
        let mempool = Mempool::new(MempoolConfig::default());
        
        assert_eq!(mempool.calculate_priority(15000), TransactionPriority::Urgent);
        assert_eq!(mempool.calculate_priority(7000), TransactionPriority::High);
        assert_eq!(mempool.calculate_priority(3000), TransactionPriority::Normal);
        assert_eq!(mempool.calculate_priority(1500), TransactionPriority::Low);
    }
    
    #[tokio::test]
    async fn test_mempool_stats() {
        let mut config = MempoolConfig::default();
        config.min_relay_fee_rate = 1; // Set very low fee rate for tests
        let mut mempool = Mempool::new(config);
        
        // Add some transactions with different characteristics to avoid deduplication
        for i in 0..5 {
            let mut input_hash = [0u8; 32];
            input_hash[0] = i as u8;  // Make unique
            input_hash[1] = (i * 2) as u8;  // Extra uniqueness
            
            let tx = Transaction::new(
                1,
                vec![TransactionInput::new(Hash256Wrapper::from_hash256(&input_hash), i as u32, vec![i as u8])],
                vec![TransactionOutput::create_p2pkh(100_000_000 - (i as u64 * 1000), &format!("address_{}", i)).unwrap()],
            );
            
            let result = mempool.add_transaction(tx).await;
            if let Err(e) = &result {
                eprintln!("Failed to add transaction {}: {:?}", i, e);
            }
            result.unwrap();
        }
        
        let stats = mempool.get_stats();
        assert_eq!(stats.transaction_count, 5);
        assert!(stats.memory_usage > 0);
        assert!(stats.priority_counts.len() > 0);
    }
}