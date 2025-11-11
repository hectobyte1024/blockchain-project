//! High-performance async P2P networking layer for blockchain
//!
//! This module provides the networking infrastructure for the hybrid blockchain,
//! leveraging Rust's async capabilities while integrating with the C++ core engine.

pub mod peer;
pub mod protocol; 
pub mod discovery;
pub mod swarm;
pub mod tx_broadcast;

use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::Arc,
    time::Duration,
};
use tokio::sync::broadcast;
use tracing::info;

// Re-export commonly used types
pub use blockchain_core::{Hash256, block::Block, transaction::Transaction};
pub use uuid::Uuid;
pub use tx_broadcast::{TransactionBroadcaster, TransactionPriority, BroadcastStats};

/// Network error types
#[derive(Debug, thiserror::Error)]
pub enum NetworkError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    
    #[error("Connection error: {0}")]
    ConnectionError(String),
    
    #[error("Connection timeout")]
    ConnectionTimeout,
    
    #[error("Invalid message: {0}")]
    InvalidMessage(String),
    
    #[error("Peer not found: {0}")]
    PeerNotFound(String),
    
    #[error("Network timeout")]
    Timeout,
    
    #[error("Invalid address: {0}")]
    InvalidAddress(String),
    
    #[error("Protocol error: {0}")]
    ProtocolError(String),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    SerializationError(String),
}

pub type Result<T> = std::result::Result<T, NetworkError>;

/// Network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Local listening address
    pub listen_addr: SocketAddr,
    /// Maximum number of peers
    pub max_peers: usize,
    /// Connection timeout
    pub connection_timeout: Duration,
    /// Heartbeat interval
    pub heartbeat_interval: Duration,
    /// Maximum message size
    pub max_message_size: usize,
    /// Network magic bytes (mainnet/testnet)
    pub network_magic: u32,
    /// DNS seed nodes for bootstrapping
    pub dns_seeds: Vec<String>,
    /// Known peer addresses
    pub seed_peers: Vec<SocketAddr>,
    /// Our network services
    pub our_services: u64,
    /// Listening port
    pub listening_port: u16,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            listen_addr: "0.0.0.0:8333".parse().unwrap(),
            max_peers: 125,
            connection_timeout: Duration::from_secs(10),
            heartbeat_interval: Duration::from_secs(30),
            max_message_size: 32 * 1024 * 1024, // 32MB
            network_magic: 0xD9B4BEF9, // Mainnet magic
            dns_seeds: vec![
                "seed.bitcoin.sipa.be".to_string(),
                "dnsseed.bluematt.me".to_string(), 
                "seed.bitcoinstats.com".to_string(),
            ],
            seed_peers: vec![],
            our_services: protocol::services::NODE_NETWORK,
            listening_port: 8333,
        }
    }
}

/// Network statistics
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct NetworkStats {
    /// Number of connected peers
    pub connected_peers: usize,
    /// Outbound connections
    pub outbound_connections: usize,
    /// Inbound connections  
    pub inbound_connections: usize,
    /// Total bytes sent
    pub bytes_sent: u64,
    /// Total bytes received
    pub bytes_received: u64,
    /// Total messages sent
    pub messages_sent: u64,
    /// Total messages received
    pub messages_received: u64,
    /// Connection attempts
    pub connection_attempts: u64,
    /// Failed connections
    pub failed_connections: u64,
    /// Addresses discovered
    pub addresses_discovered: u64,
}

/// Main network manager that coordinates P2P operations
pub struct NetworkManager {
    /// Network configuration
    config: NetworkConfig,
    /// Address manager for peer discovery
    address_manager: Arc<discovery::AddressManager>,
    /// Network swarm for connection management
    swarm: Arc<swarm::NetworkSwarm>,
    /// Event receiver
    event_receiver: Option<broadcast::Receiver<swarm::NetworkEvent>>,
}

impl NetworkManager {
    /// Create new network manager
    pub fn new(config: NetworkConfig) -> Result<Self> {
        // Create address manager
        let address_manager = Arc::new(discovery::AddressManager::new(config.dns_seeds.clone()));
        
        // Add manual seed peers
        for addr in &config.seed_peers {
            let rt = tokio::runtime::Handle::current();
            rt.block_on(async {
                let _ = address_manager.add_manual_address(*addr, config.our_services).await;
            });
        }
        
        // Create network swarm
        let (swarm, event_receiver) = swarm::NetworkSwarm::new(
            address_manager.clone(),
            config.our_services,
            config.listening_port,
        );
        
        Ok(Self {
            config,
            address_manager,
            swarm: Arc::new(swarm),
            event_receiver: Some(event_receiver),
        })
    }
    
    /// Start the network manager
    pub async fn start(&mut self) -> Result<()> {
        info!("Starting network manager on {}", self.config.listen_addr);
        
        // Start DNS seed discovery
        self.address_manager.discover_from_dns_seeds().await?;
        
        // Start the network swarm
        self.swarm.start().await?;
        
        info!("Network manager started successfully");
        Ok(())
    }
    
    /// Get event receiver for network events
    pub fn take_event_receiver(&mut self) -> Option<broadcast::Receiver<swarm::NetworkEvent>> {
        self.event_receiver.take()
    }
    
    /// Broadcast message to all connected peers
    pub async fn broadcast_message(&self, message: protocol::Message) -> Result<usize> {
        self.swarm.broadcast_message(message).await
    }
    
    /// Send message to specific peer
    pub async fn send_to_peer(&self, peer_id: Uuid, message: protocol::Message) -> Result<()> {
        self.swarm.send_to_peer(peer_id, message).await
    }
    
    /// Get connected peer IDs
    pub async fn get_connected_peers(&self) -> Vec<Uuid> {
        self.swarm.get_connected_peers().await
    }
    
    /// Get network statistics
    pub async fn get_stats(&self) -> NetworkStats {
        let swarm_stats = self.swarm.get_stats().await;
        let discovery_stats = self.address_manager.get_stats().await;
        
        NetworkStats {
            connected_peers: swarm_stats.connected_peers,
            outbound_connections: swarm_stats.outbound_connections,
            inbound_connections: swarm_stats.inbound_connections,
            bytes_sent: swarm_stats.bytes_sent,
            bytes_received: swarm_stats.bytes_received,
            messages_sent: swarm_stats.messages_sent,
            messages_received: swarm_stats.messages_received,
            connection_attempts: swarm_stats.connection_attempts,
            failed_connections: swarm_stats.failed_connections,
            addresses_discovered: discovery_stats.addresses_discovered,
        }
    }
    
    /// Disconnect peer
    pub async fn disconnect_peer(&self, peer_id: Uuid, reason: &str) -> Result<()> {
        self.swarm.disconnect_peer(peer_id, reason).await
    }
    
    /// Get address distribution by source
    pub async fn get_address_distribution(&self) -> HashMap<String, usize> {
        self.address_manager.get_address_distribution().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_network_config_default() {
        let config = NetworkConfig::default();
        assert_eq!(config.max_peers, 125);
        assert_eq!(config.listening_port, 8333);
        assert!(!config.dns_seeds.is_empty());
    }
    
    #[tokio::test]
    async fn test_network_manager_creation() {
        let config = NetworkConfig::default();
        let manager = NetworkManager::new(config);
        assert!(manager.is_ok());
    }
}