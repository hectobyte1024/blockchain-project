//! Event indexing and filtering for smart contracts

use crate::contracts::{Log, EthAddress};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Event filter criteria
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventFilter {
    /// Filter by contract address (optional)
    pub address: Option<EthAddress>,
    /// Filter by topics (optional, each position can match multiple values)
    pub topics: Vec<Option<Vec<String>>>,
    /// Starting block height (inclusive)
    pub from_block: Option<u64>,
    /// Ending block height (inclusive)
    pub to_block: Option<u64>,
}

/// Indexed event with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedEvent {
    /// The log itself
    pub log: Log,
    /// Block height where event was emitted
    pub block_height: u64,
    /// Transaction hash that emitted the event
    pub tx_hash: String,
    /// Index of log within transaction
    pub log_index: u32,
}

/// Event indexer for contract events
pub struct EventIndexer {
    /// Events indexed by block height
    events_by_block: Arc<RwLock<HashMap<u64, Vec<IndexedEvent>>>>,
    /// Events indexed by contract address
    events_by_address: Arc<RwLock<HashMap<EthAddress, Vec<IndexedEvent>>>>,
    /// Events indexed by topic[0] (event signature)
    events_by_topic0: Arc<RwLock<HashMap<String, Vec<IndexedEvent>>>>,
}

impl EventIndexer {
    /// Create new event indexer
    pub fn new() -> Self {
        Self {
            events_by_block: Arc::new(RwLock::new(HashMap::new())),
            events_by_address: Arc::new(RwLock::new(HashMap::new())),
            events_by_topic0: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Index events from a transaction
    pub async fn index_events(
        &self,
        logs: Vec<Log>,
        block_height: u64,
        tx_hash: String,
    ) {
        let mut by_block = self.events_by_block.write().await;
        let mut by_address = self.events_by_address.write().await;
        let mut by_topic0 = self.events_by_topic0.write().await;
        
        for (log_index, log) in logs.into_iter().enumerate() {
            let event = IndexedEvent {
                log: log.clone(),
                block_height,
                tx_hash: tx_hash.clone(),
                log_index: log_index as u32,
            };
            
            // Index by block height
            by_block
                .entry(block_height)
                .or_insert_with(Vec::new)
                .push(event.clone());
            
            // Index by contract address
            by_address
                .entry(log.address)
                .or_insert_with(Vec::new)
                .push(event.clone());
            
            // Index by first topic (event signature)
            if let Some(topic0) = log.topics.first() {
                by_topic0
                    .entry(topic0.clone())
                    .or_insert_with(Vec::new)
                    .push(event);
            }
        }
    }
    
    /// Query events with filter
    pub async fn query_events(&self, filter: EventFilter) -> Vec<IndexedEvent> {
        let by_block = self.events_by_block.read().await;
        let by_address = self.events_by_address.read().await;
        
        let mut results = Vec::new();
        
        // If address filter is specified, start with address-indexed events
        if let Some(address) = filter.address {
            if let Some(events) = by_address.get(&address) {
                results = events.clone();
            }
        } else {
            // Otherwise, collect events from all blocks in range
            let from = filter.from_block.unwrap_or(0);
            let to = filter.to_block.unwrap_or(u64::MAX);
            
            for height in from..=to {
                if let Some(events) = by_block.get(&height) {
                    results.extend(events.clone());
                }
                
                // Don't search too far if no to_block specified
                if filter.to_block.is_none() && height > from + 1000 {
                    break;
                }
            }
        }
        
        // Apply block range filter
        if let Some(from) = filter.from_block {
            results.retain(|e| e.block_height >= from);
        }
        if let Some(to) = filter.to_block {
            results.retain(|e| e.block_height <= to);
        }
        
        // Apply topic filters
        for (topic_idx, topic_filter) in filter.topics.iter().enumerate() {
            if let Some(allowed_topics) = topic_filter {
                results.retain(|e| {
                    e.log.topics.get(topic_idx)
                        .map(|t| allowed_topics.contains(t))
                        .unwrap_or(false)
                });
            }
        }
        
        results
    }
    
    /// Get total number of indexed events
    pub async fn get_event_count(&self) -> usize {
        let by_block = self.events_by_block.read().await;
        by_block.values().map(|v| v.len()).sum()
    }
    
    /// Get events for a specific block
    pub async fn get_events_by_block(&self, block_height: u64) -> Vec<IndexedEvent> {
        let by_block = self.events_by_block.read().await;
        by_block.get(&block_height).cloned().unwrap_or_default()
    }
    
    /// Get events for a specific contract
    pub async fn get_events_by_address(&self, address: EthAddress) -> Vec<IndexedEvent> {
        let by_address = self.events_by_address.read().await;
        by_address.get(&address).cloned().unwrap_or_default()
    }
}

impl Default for EventIndexer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_event_indexing() {
        let indexer = EventIndexer::new();
        
        let log = Log {
            address: EthAddress::new([1; 20]),
            topics: vec!["topic1".to_string()],
            data: vec![1, 2, 3],
        };
        
        indexer.index_events(vec![log], 100, "txhash".to_string()).await;
        
        let count = indexer.get_event_count().await;
        assert_eq!(count, 1);
        
        let events = indexer.get_events_by_block(100).await;
        assert_eq!(events.len(), 1);
    }
}
