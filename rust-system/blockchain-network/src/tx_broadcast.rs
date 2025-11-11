use crate::{
    NetworkManager, NetworkError, protocol::{Message, MessagePayload, TxMessage, MessageType}
};
use blockchain_core::{
    transaction::Transaction,
    Hash256, BlockchainError, Result as BlockchainResult
};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH, Duration, Instant},
};
use tokio::sync::RwLock;
use tracing::{info, warn, error, debug};
use uuid::Uuid;

/// Transaction broadcast manager
pub struct TransactionBroadcaster {
    network: Arc<NetworkManager>,
    /// Pending transactions awaiting confirmation
    pending_transactions: Arc<RwLock<HashMap<Hash256, PendingTransaction>>>,
    /// Recently seen transaction hashes (for duplicate detection)
    seen_transactions: Arc<RwLock<HashSet<Hash256>>>,
    /// Maximum time to keep pending transactions
    max_pending_time: Duration,
    /// Maximum number of transactions to keep in memory
    max_pending_count: usize,
    /// Successful broadcast counter
    successful_broadcasts: Arc<std::sync::atomic::AtomicU64>,
    /// Failed broadcast counter
    failed_broadcasts: Arc<std::sync::atomic::AtomicU64>,
    /// Duplicate reception counter
    duplicate_receptions: Arc<std::sync::atomic::AtomicU64>,
}

/// Pending transaction metadata
#[derive(Debug, Clone)]
pub struct PendingTransaction {
    /// The actual transaction
    pub transaction: Transaction,
    /// When it was first broadcast
    pub broadcast_time: Instant,
    /// Number of peers it was sent to
    pub peer_count: usize,
    /// Rebroadcast attempts
    pub rebroadcast_count: u32,
    /// Transaction priority (higher = more important)
    pub priority: TransactionPriority,
}

/// Transaction priority levels
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum TransactionPriority {
    Low,
    Normal,
    High,
    Critical,
}

/// Transaction broadcast statistics
#[derive(Debug, Clone, serde::Serialize)]
pub struct BroadcastStats {
    pub total_broadcasted: u64,
    pub pending_count: usize,
    pub seen_count: usize,
    pub successful_broadcasts: u64,
    pub failed_broadcasts: u64,
    pub duplicate_receptions: u64,
}

impl TransactionBroadcaster {
    /// Create a new transaction broadcaster
    pub fn new(network: Arc<NetworkManager>) -> Self {
        Self {
            network,
            pending_transactions: Arc::new(RwLock::new(HashMap::new())),
            seen_transactions: Arc::new(RwLock::new(HashSet::new())),
            max_pending_time: Duration::from_secs(300), // 5 minutes
            max_pending_count: 10000, // Maximum 10k pending transactions
            successful_broadcasts: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            failed_broadcasts: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            duplicate_receptions: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    /// Broadcast a transaction to the network
    pub async fn broadcast_transaction(
        &self,
        transaction: Transaction,
        priority: TransactionPriority,
    ) -> BlockchainResult<Hash256> {
        let tx_hash = transaction.get_hash()?;
        
        // Check if we've already seen this transaction
        {
            let seen = self.seen_transactions.read().await;
            if seen.contains(&tx_hash) {
                debug!("Transaction {} already seen, skipping broadcast", hex::encode(tx_hash));
                
                // Increment duplicate reception counter
                self.duplicate_receptions.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                
                return Ok(tx_hash);
            }
        }

        info!("Broadcasting transaction: {}", hex::encode(tx_hash));

        // Serialize the transaction
        let tx_data = bincode::serialize(&transaction)
            .map_err(|e| BlockchainError::SerializationError(e.to_string()))?;

        // Create network message
        let tx_message = TxMessage {
            tx_hash,
            tx_data,
        };

        let message = Message {
            message_type: MessageType::Tx,
            payload: MessagePayload::Tx(tx_message),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            checksum: self.calculate_checksum(&tx_hash),
        };

        // Broadcast to network
        match self.network.broadcast_message(message).await {
            Ok(peer_count) => {
                info!("Transaction {} broadcast to {} peers", hex::encode(tx_hash), peer_count);
                
                // Increment successful broadcast counter
                self.successful_broadcasts.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                
                // Track the pending transaction
                let pending = PendingTransaction {
                    transaction,
                    broadcast_time: Instant::now(),
                    peer_count,
                    rebroadcast_count: 0,
                    priority,
                };

                {
                    let mut pending_txs = self.pending_transactions.write().await;
                    pending_txs.insert(tx_hash, pending);
                }

                // Mark as seen
                {
                    let mut seen = self.seen_transactions.write().await;
                    seen.insert(tx_hash);
                }

                Ok(tx_hash)
            }
            Err(e) => {
                error!("Failed to broadcast transaction {}: {}", hex::encode(tx_hash), e);
                
                // Increment failed broadcast counter
                self.failed_broadcasts.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                
                Err(BlockchainError::InvalidTransaction(format!("Broadcast failed: {}", e)))
            }
        }
    }

    /// Handle incoming transaction from the network
    pub async fn handle_incoming_transaction(&self, tx_message: TxMessage) -> BlockchainResult<()> {
        let tx_hash = tx_message.tx_hash;
        
        // Check for duplicates
        {
            let seen = self.seen_transactions.read().await;
            if seen.contains(&tx_hash) {
                debug!("Duplicate transaction received: {}", hex::encode(tx_hash));
                return Ok(());
            }
        }

        // Deserialize transaction
        let transaction: Transaction = bincode::deserialize(&tx_message.tx_data)
            .map_err(|e| BlockchainError::SerializationError(e.to_string()))?;

        // Verify transaction hash matches
        let computed_hash = transaction.get_hash()?;
        if computed_hash != tx_hash {
            warn!("Transaction hash mismatch: expected {}, got {}", 
                  hex::encode(tx_hash), hex::encode(computed_hash));
            return Err(BlockchainError::InvalidTransaction("Hash mismatch".to_string()));
        }

        info!("Received valid transaction: {}", hex::encode(tx_hash));

        // Mark as seen
        {
            let mut seen = self.seen_transactions.write().await;
            seen.insert(tx_hash);
        }

        // Add to mempool and validate transaction
        // In a real implementation, this would integrate with the consensus validator
        // and mempool to verify transaction validity and add to pending pool
        
        // For now, perform basic validation
        if !transaction.is_valid() {
            warn!("Received invalid transaction: {}", hex::encode(tx_hash));
            return Err(BlockchainError::InvalidTransaction("Transaction validation failed".to_string()));
        }
        
        // Log successful transaction reception for monitoring
        info!("Transaction {} successfully received and validated", hex::encode(tx_hash));
        
        Ok(())
    }

    /// Rebroadcast high-priority pending transactions
    pub async fn rebroadcast_pending(&self) -> BlockchainResult<usize> {
        let mut rebroadcast_count = 0;
        let now = Instant::now();
        
        let mut pending_txs = self.pending_transactions.write().await;
        let mut to_remove = Vec::new();

        for (tx_hash, pending) in pending_txs.iter_mut() {
            // Remove expired transactions
            if now.duration_since(pending.broadcast_time) > self.max_pending_time {
                to_remove.push(*tx_hash);
                continue;
            }

            // Rebroadcast high-priority transactions that haven't been confirmed
            if pending.priority >= TransactionPriority::High && 
               pending.rebroadcast_count < 3 &&
               now.duration_since(pending.broadcast_time) > Duration::from_secs(60) {
                
                if let Ok(tx_data) = bincode::serialize(&pending.transaction) {
                    let tx_message = TxMessage {
                        tx_hash: *tx_hash,
                        tx_data,
                    };

                    let message = Message {
                        message_type: MessageType::Tx,
                        payload: MessagePayload::Tx(tx_message),
                        timestamp: SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_secs(),
                        checksum: self.calculate_checksum(tx_hash),
                    };

                    if let Ok(peer_count) = self.network.broadcast_message(message).await {
                        pending.rebroadcast_count += 1;
                        pending.peer_count = peer_count;
                        rebroadcast_count += 1;
                        info!("Rebroadcast transaction {} to {} peers", 
                              hex::encode(tx_hash), peer_count);
                    }
                }
            }
        }

        // Remove expired transactions
        for tx_hash in to_remove {
            pending_txs.remove(&tx_hash);
        }

        Ok(rebroadcast_count)
    }

    /// Remove confirmed transaction from pending list
    pub async fn confirm_transaction(&self, tx_hash: &Hash256) {
        let mut pending_txs = self.pending_transactions.write().await;
        if let Some(pending) = pending_txs.remove(tx_hash) {
            info!("Transaction {} confirmed and removed from pending", 
                  hex::encode(tx_hash));
        }
    }

    /// Get broadcast statistics
    pub async fn get_stats(&self) -> BroadcastStats {
        let pending_txs = self.pending_transactions.read().await;
        let seen_txs = self.seen_transactions.read().await;
        
        BroadcastStats {
            total_broadcasted: pending_txs.len() as u64,
            pending_count: pending_txs.len(),
            seen_count: seen_txs.len(),
            successful_broadcasts: self.successful_broadcasts.load(std::sync::atomic::Ordering::Relaxed),
            failed_broadcasts: self.failed_broadcasts.load(std::sync::atomic::Ordering::Relaxed),
            duplicate_receptions: self.duplicate_receptions.load(std::sync::atomic::Ordering::Relaxed),
        }
    }

    /// Clean up old seen transactions to prevent memory bloat
    pub async fn cleanup_seen_transactions(&self) {
        let mut seen = self.seen_transactions.write().await;
        
        // Keep only the most recent 50,000 transaction hashes
        if seen.len() > 50000 {
            let excess = seen.len() - 40000; // Remove 10k to give some buffer
            let mut to_remove: Vec<Hash256> = seen.iter().take(excess).cloned().collect();
            
            for hash in to_remove {
                seen.remove(&hash);
            }
            
            info!("Cleaned up {} old transaction hashes from seen set", excess);
        }
    }

    /// Get pending transactions for a specific priority
    pub async fn get_pending_by_priority(&self, priority: TransactionPriority) -> Vec<Hash256> {
        let pending_txs = self.pending_transactions.read().await;
        pending_txs
            .iter()
            .filter(|(_, pending)| pending.priority == priority)
            .map(|(hash, _)| *hash)
            .collect()
    }

    /// Force rebroadcast a specific transaction
    pub async fn force_rebroadcast(&self, tx_hash: &Hash256) -> BlockchainResult<()> {
        let pending_txs = self.pending_transactions.read().await;
        
        if let Some(pending) = pending_txs.get(tx_hash) {
            let tx_data = bincode::serialize(&pending.transaction)
                .map_err(|e| BlockchainError::SerializationError(e.to_string()))?;

            let tx_message = TxMessage {
                tx_hash: *tx_hash,
                tx_data,
            };

            let message = Message {
                message_type: MessageType::Tx,
                payload: MessagePayload::Tx(tx_message),
                timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                checksum: self.calculate_checksum(tx_hash),
            };

            match self.network.broadcast_message(message).await {
                Ok(peer_count) => {
                    info!("Force rebroadcast transaction {} to {} peers", 
                          hex::encode(tx_hash), peer_count);
                    Ok(())
                }
                Err(e) => {
                    error!("Failed to force rebroadcast transaction {}: {}", 
                           hex::encode(tx_hash), e);
                    Err(BlockchainError::InvalidTransaction(format!("Rebroadcast failed: {}", e)))
                }
            }
        } else {
            Err(BlockchainError::InvalidTransaction("Transaction not found in pending list".to_string()))
        }
    }

    /// Calculate simple checksum for message integrity
    fn calculate_checksum(&self, tx_hash: &Hash256) -> u32 {
        let mut checksum = 0u32;
        for chunk in tx_hash.chunks(4) {
            let value = u32::from_le_bytes([
                chunk.get(0).copied().unwrap_or(0),
                chunk.get(1).copied().unwrap_or(0),
                chunk.get(2).copied().unwrap_or(0),
                chunk.get(3).copied().unwrap_or(0),
            ]);
            checksum = checksum.wrapping_add(value);
        }
        checksum
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use blockchain_core::transaction::{TransactionInput, TransactionOutput};

    #[tokio::test]
    async fn test_transaction_broadcaster_creation() {
        let network = Arc::new(NetworkManager::new(Default::default()).unwrap());
        let broadcaster = TransactionBroadcaster::new(network);
        
        let stats = broadcaster.get_stats().await;
        assert_eq!(stats.pending_count, 0);
        assert_eq!(stats.seen_count, 0);
    }

    #[tokio::test]
    async fn test_duplicate_detection() {
        let network = Arc::new(NetworkManager::new(Default::default()).unwrap());
        let broadcaster = TransactionBroadcaster::new(network);
        
        // Create a test transaction
        let tx = Transaction::new(1, vec![], vec![]);
        let tx_hash = tx.get_hash().unwrap();
        
        // Mark as seen
        {
            let mut seen = broadcaster.seen_transactions.write().await;
            seen.insert(tx_hash);
        }
        
        // Try to broadcast - should be skipped
        let result = broadcaster.broadcast_transaction(tx, TransactionPriority::Normal).await;
        assert!(result.is_ok());
    }
}