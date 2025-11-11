//! Peer discovery and address management
//!
//! This module handles finding and managing peer addresses through DNS seeds,
//! peer exchange, and address caching.

use crate::{NetworkError, Result, protocol::NetworkAddress};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    net::{IpAddr, Ipv4Addr, SocketAddr},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::{
    sync::RwLock,
    time::sleep,
};
use tracing::{info, warn, error, debug};

/// Maximum number of addresses to store
const MAX_ADDRESSES: usize = 10000;

/// Address freshness threshold (24 hours)
const ADDRESS_FRESHNESS_THRESHOLD: u64 = 24 * 60 * 60;

/// Maximum addresses per DNS seed query
const MAX_DNS_ADDRESSES: usize = 256;

/// Peer address with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerAddress {
    /// Network address
    pub address: NetworkAddress,
    /// Socket address for connection
    pub socket_addr: SocketAddr,
    /// Last connection attempt timestamp
    pub last_attempt: Option<u64>,
    /// Last successful connection timestamp
    pub last_success: Option<u64>,
    /// Number of failed attempts
    pub failed_attempts: u32,
    /// Source of this address
    pub source: AddressSource,
    /// Address quality score
    pub score: i32,
}

/// Source of peer address
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AddressSource {
    /// From DNS seed
    DnsSeed(String),
    /// From peer exchange
    PeerExchange(String),
    /// Manual configuration
    Manual,
    /// Self-discovered (local network)
    SelfDiscovered,
}

/// Address manager for peer discovery
pub struct AddressManager {
    /// All known addresses
    addresses: RwLock<HashMap<SocketAddr, PeerAddress>>,
    /// Banned addresses (temporary)
    banned_addresses: RwLock<HashSet<SocketAddr>>,
    /// DNS seed addresses for bootstrapping
    dns_seeds: Vec<String>,
    /// Statistics
    stats: RwLock<DiscoveryStats>,
}

/// Discovery statistics
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct DiscoveryStats {
    /// Total addresses discovered
    pub addresses_discovered: u64,
    /// DNS seed queries performed
    pub dns_queries: u64,
    /// Successful connections
    pub successful_connections: u64,
    /// Failed connections
    pub failed_connections: u64,
    /// Addresses from peer exchange
    pub peer_exchange_addresses: u64,
}

impl PeerAddress {
    /// Create new peer address
    pub fn new(
        socket_addr: SocketAddr,
        services: u64,
        source: AddressSource,
    ) -> Self {
        let network_addr = match socket_addr.ip() {
            IpAddr::V4(ipv4) => NetworkAddress::from_ipv4(
                ipv4.octets(),
                socket_addr.port(),
                services,
            ),
            IpAddr::V6(ipv6) => NetworkAddress::new(
                ipv6.octets(),
                socket_addr.port(),
                services,
            ),
        };

        Self {
            address: network_addr,
            socket_addr,
            last_attempt: None,
            last_success: None,
            failed_attempts: 0,
            source,
            score: 0,
        }
    }

    /// Record connection attempt
    pub fn record_attempt(&mut self, success: bool) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        self.last_attempt = Some(now);

        if success {
            self.last_success = Some(now);
            self.failed_attempts = 0;
            self.score = (self.score + 1).min(100);
        } else {
            self.failed_attempts += 1;
            self.score = (self.score - 2).max(-100);
        }
    }

    /// Check if address is fresh (recently seen)
    pub fn is_fresh(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        now - self.address.timestamp < ADDRESS_FRESHNESS_THRESHOLD
    }

    /// Check if address should be attempted
    pub fn should_attempt(&self) -> bool {
        // Don't attempt if too many failures
        if self.failed_attempts > 3 {
            return false;
        }

        // Check attempt rate limiting
        if let Some(last_attempt) = self.last_attempt {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            // Wait longer between attempts based on failures
            let wait_time = match self.failed_attempts {
                0 => 0,
                1 => 60,      // 1 minute
                2 => 300,     // 5 minutes  
                3 => 900,     // 15 minutes
                _ => 3600,    // 1 hour
            };

            if now - last_attempt < wait_time {
                return false;
            }
        }

        true
    }
}

impl AddressManager {
    /// Create new address manager
    pub fn new(dns_seeds: Vec<String>) -> Self {
        Self {
            addresses: RwLock::new(HashMap::new()),
            banned_addresses: RwLock::new(HashSet::new()),
            dns_seeds,
            stats: RwLock::new(DiscoveryStats::default()),
        }
    }

    /// Add address from manual configuration
    pub async fn add_manual_address(&self, addr: SocketAddr, services: u64) -> Result<()> {
        let peer_addr = PeerAddress::new(addr, services, AddressSource::Manual);
        
        let mut addresses = self.addresses.write().await;
        addresses.insert(addr, peer_addr);
        
        info!("Added manual address: {}", addr);
        Ok(())
    }

    /// Add addresses from peer exchange
    pub async fn add_peer_addresses(
        &self,
        peer_id: &str,
        addresses: Vec<NetworkAddress>,
    ) -> Result<()> {
        let mut addr_map = self.addresses.write().await;
        let mut stats = self.stats.write().await;
        let mut added_count = 0;

        for network_addr in addresses {
            if addr_map.len() >= MAX_ADDRESSES {
                break;
            }

            // Convert to socket address
            let socket_addr = if network_addr.is_ipv4() {
                if let Some(ipv4) = network_addr.get_ipv4() {
                    SocketAddr::new(
                        IpAddr::V4(Ipv4Addr::from(ipv4)),
                        network_addr.port,
                    )
                } else {
                    continue;
                }
            } else {
                SocketAddr::new(
                    IpAddr::V6(std::net::Ipv6Addr::from(network_addr.ip)),
                    network_addr.port,
                )
            };

            // Skip if already known or banned
            if addr_map.contains_key(&socket_addr) {
                continue;
            }

            let banned = self.banned_addresses.read().await;
            if banned.contains(&socket_addr) {
                continue;
            }
            drop(banned);

            // Add new address
            let peer_addr = PeerAddress {
                address: network_addr,
                socket_addr,
                last_attempt: None,
                last_success: None,
                failed_attempts: 0,
                source: AddressSource::PeerExchange(peer_id.to_string()),
                score: 0,
            };

            addr_map.insert(socket_addr, peer_addr);
            added_count += 1;
        }

        stats.addresses_discovered += added_count as u64;
        stats.peer_exchange_addresses += added_count as u64;

        info!("Added {} addresses from peer {}", added_count, peer_id);
        Ok(())
    }

    /// Discover addresses from DNS seeds
    pub async fn discover_from_dns_seeds(&self) -> Result<()> {
        info!("Starting DNS seed discovery...");

        for dns_seed in &self.dns_seeds {
            if let Err(e) = self.query_dns_seed(dns_seed).await {
                warn!("DNS seed {} query failed: {}", dns_seed, e);
            }
            
            // Small delay between queries
            sleep(Duration::from_millis(500)).await;
        }

        let stats = self.stats.read().await;
        info!("DNS discovery completed. Total addresses: {}", stats.addresses_discovered);
        
        Ok(())
    }

    /// Query single DNS seed
    async fn query_dns_seed(&self, dns_seed: &str) -> Result<()> {
        debug!("Querying DNS seed: {}", dns_seed);

        // Use tokio's DNS resolution
        match tokio::net::lookup_host((dns_seed, 8333)).await {
            Ok(addresses) => {
                let mut addr_map = self.addresses.write().await;
                let mut stats = self.stats.write().await;
                let mut added_count = 0;

                for socket_addr in addresses.take(MAX_DNS_ADDRESSES) {
                    if addr_map.len() >= MAX_ADDRESSES {
                        break;
                    }

                    // Skip if already known
                    if addr_map.contains_key(&socket_addr) {
                        continue;
                    }

                    // Skip if banned
                    let banned = self.banned_addresses.read().await;
                    if banned.contains(&socket_addr) {
                        continue;
                    }
                    drop(banned);

                    // Add new address
                    let peer_addr = PeerAddress::new(
                        socket_addr,
                        crate::protocol::services::NODE_NETWORK,
                        AddressSource::DnsSeed(dns_seed.to_string()),
                    );

                    addr_map.insert(socket_addr, peer_addr);
                    added_count += 1;
                }

                stats.dns_queries += 1;
                stats.addresses_discovered += added_count as u64;

                info!("DNS seed {} provided {} addresses", dns_seed, added_count);
                Ok(())
            }
            Err(e) => {
                error!("DNS resolution failed for {}: {}", dns_seed, e);
                Err(NetworkError::InvalidAddress(format!("DNS resolution failed: {}", e)))
            }
        }
    }

    /// Get addresses for outbound connections
    pub async fn get_connection_candidates(&self, count: usize) -> Vec<PeerAddress> {
        let addresses = self.addresses.read().await;
        let banned = self.banned_addresses.read().await;

        let mut candidates: Vec<PeerAddress> = addresses
            .values()
            .filter(|addr| {
                !banned.contains(&addr.socket_addr) && 
                addr.should_attempt() &&
                addr.is_fresh()
            })
            .cloned()
            .collect();

        // Sort by score (best first)
        candidates.sort_by(|a, b| b.score.cmp(&a.score));

        // Take requested count
        candidates.truncate(count);
        candidates
    }

    /// Record connection attempt result
    pub async fn record_connection_attempt(
        &self,
        addr: SocketAddr,
        success: bool,
    ) -> Result<()> {
        let mut addresses = self.addresses.write().await;
        let mut stats = self.stats.write().await;

        if let Some(peer_addr) = addresses.get_mut(&addr) {
            peer_addr.record_attempt(success);

            if success {
                stats.successful_connections += 1;
            } else {
                stats.failed_connections += 1;
            }
        }

        Ok(())
    }

    /// Ban address temporarily
    pub async fn ban_address(&self, addr: SocketAddr, duration: Duration) -> Result<()> {
        {
            let mut banned = self.banned_addresses.write().await;
            banned.insert(addr);
        }

        info!("Banned address {} for {:?}", addr, duration);

        // Schedule unban
        tokio::spawn(async move {
            sleep(duration).await;
            // Note: In a real implementation, we'd need a reference to self here
            // For now, this is a simplified version
        });

        Ok(())
    }

    /// Get discovery statistics
    pub async fn get_stats(&self) -> DiscoveryStats {
        let stats = self.stats.read().await;
        let addresses = self.addresses.read().await;
        
        let mut result = stats.clone();
        result.addresses_discovered = addresses.len() as u64;
        result
    }

    /// Get address count by source
    pub async fn get_address_distribution(&self) -> HashMap<String, usize> {
        let addresses = self.addresses.read().await;
        let mut distribution = HashMap::new();

        for addr in addresses.values() {
            let source_key = match &addr.source {
                AddressSource::DnsSeed(seed) => format!("DNS: {}", seed),
                AddressSource::PeerExchange(peer) => format!("Peer: {}", peer),
                AddressSource::Manual => "Manual".to_string(),
                AddressSource::SelfDiscovered => "Self-Discovered".to_string(),
            };

            *distribution.entry(source_key).or_insert(0) += 1;
        }

        distribution
    }

    /// Cleanup old and bad addresses
    pub async fn cleanup_addresses(&self) -> Result<()> {
        let mut addresses = self.addresses.write().await;
        let mut banned = self.banned_addresses.write().await;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Remove very old addresses
        let old_threshold = now - (7 * 24 * 60 * 60); // 7 days
        addresses.retain(|_, addr| addr.address.timestamp > old_threshold);

        // Remove addresses with very low scores
        addresses.retain(|_, addr| addr.score > -50);

        // Clear old bans (this is simplified - real implementation would track ban times)
        banned.clear();

        info!("Address cleanup completed. Remaining addresses: {}", addresses.len());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_peer_address_creation() {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8333);
        let peer_addr = PeerAddress::new(
            addr,
            crate::protocol::services::NODE_NETWORK,
            AddressSource::Manual,
        );

        assert_eq!(peer_addr.socket_addr, addr);
        assert_eq!(peer_addr.source, AddressSource::Manual);
        assert_eq!(peer_addr.failed_attempts, 0);
        assert!(peer_addr.should_attempt());
    }

    #[tokio::test]
    async fn test_address_manager() {
        let manager = AddressManager::new(vec!["seed.test.com".to_string()]);
        
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)), 8333);
        manager.add_manual_address(addr, crate::protocol::services::NODE_NETWORK).await.unwrap();

        let candidates = manager.get_connection_candidates(10).await;
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].socket_addr, addr);
    }

    #[test]
    fn test_address_attempt_logic() {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8333);
        let mut peer_addr = PeerAddress::new(
            addr,
            crate::protocol::services::NODE_NETWORK,
            AddressSource::Manual,
        );

        // Should attempt initially
        assert!(peer_addr.should_attempt());

        // Record failures
        peer_addr.record_attempt(false);
        peer_addr.record_attempt(false);
        peer_addr.record_attempt(false);
        peer_addr.record_attempt(false);

        // Should not attempt after too many failures
        assert!(!peer_addr.should_attempt());

        // Success should reset
        peer_addr.record_attempt(true);
        assert_eq!(peer_addr.failed_attempts, 0);
        assert!(peer_addr.score > 0);
    }
}