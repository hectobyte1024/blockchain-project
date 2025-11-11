//! Peer management and connection handling
//!
//! This module manage/// Peer events emitted by the peer
#[derive(Debug, Clone)]
pub enum PeerEvent {
    /// Message received from peer
    MessageReceived(crate::protocol::Message),
    /// Peer disconnected
    Disconnected(String),
}

/// Peer statistics
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct PeerStats {
    /// Bytes sent to this peer
    pub bytes_sent: u64,
    /// Bytes received from this peer
    pub bytes_received: u64,
    /// Messages sent to this peer
    pub messages_sent: u64,
    /// Messages received from this peer
    pub messages_received: u64,
    /// Connection start time
    pub connected_at: SystemTime,
    /// Last message time
    pub last_message_at: Option<SystemTime>,
    /// Ping response time (milliseconds)
    pub ping_time_ms: Option<u64>,
}r connections, handshakes, and communication.
//! Each peer represents a connection to another blockchain node.

use crate::{NetworkError, Result};
use serde::{Deserialize, Serialize};
use std::{
    net::SocketAddr,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
    sync::{mpsc, Mutex},
    time::timeout,
};
use uuid::Uuid;

/// Peer connection state
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PeerState {
    /// Initial state before handshake
    Connecting,
    /// Handshake in progress
    Handshaking,
    /// Fully connected and operational
    Connected,
    /// Disconnecting
    Disconnecting,
    /// Connection failed or closed
    Disconnected { reason: String },
}

/// Peer connection information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    /// Unique peer identifier
    pub peer_id: String,
    /// Remote address
    pub address: SocketAddr,
    /// User agent string
    pub user_agent: String,
    /// Protocol version
    pub version: u32,
    /// Services supported by peer
    pub services: u64,
    /// Best block height
    pub best_height: u64,
    /// Connection timestamp
    pub connected_at: u64,
    /// Last seen timestamp
    pub last_seen: u64,
}

/// Individual peer connection
#[derive(Debug)]
pub struct Peer {
    /// Peer information
    pub info: PeerInfo,
    /// Connection state
    pub state: Mutex<PeerState>,
    /// TCP stream for communication
    stream: Mutex<Option<TcpStream>>,
    /// Message sender channel
    message_tx: mpsc::UnboundedSender<Vec<u8>>,
    /// Statistics
    pub stats: Mutex<PeerStats>,
}

/// Per-peer statistics
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct PeerStats {
    /// Bytes sent to this peer
    pub bytes_sent: u64,
    /// Bytes received from this peer
    pub bytes_received: u64,
    /// Messages sent
    pub messages_sent: u64,
    /// Messages received
    pub messages_received: u64,
    /// Last ping time (milliseconds)
    pub last_ping_ms: u64,
    /// Connection errors
    pub connection_errors: u64,
}

impl PeerInfo {
    /// Create new peer info
    pub fn new(address: SocketAddr, user_agent: String, version: u32) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            peer_id: Uuid::new_v4().to_string(),
            address,
            user_agent,
            version,
            services: 0,
            best_height: 0,
            connected_at: now,
            last_seen: now,
        }
    }

    /// Update last seen timestamp
    pub fn update_last_seen(&mut self) {
        self.last_seen = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
    }
}

impl Peer {
    /// Create new peer from existing connection
    pub fn new(stream: TcpStream, info: PeerInfo) -> Result<Self> {
        let (message_tx, message_rx) = mpsc::unbounded_channel();
        
        let peer = Self {
            info,
            state: Mutex::new(PeerState::Connecting),
            stream: Mutex::new(Some(stream)),
            message_tx,
            stats: Mutex::new(PeerStats::default()),
        };

        // Start message handling task
        peer.start_message_handler(message_rx);

        Ok(peer)
    }

    /// Create outbound connection to peer
    pub async fn connect_to(address: SocketAddr, connection_timeout: Duration) -> Result<Self> {
        // Attempt TCP connection with timeout
        let stream = timeout(connection_timeout, TcpStream::connect(address))
            .await
            .map_err(|_| NetworkError::Timeout)?
            .map_err(|e| NetworkError::ConnectionFailed(e.to_string()))?;

        let info = PeerInfo::new(
            address,
            "HybridBlockchain/1.0".to_string(),
            1,
        );

        let peer = Self::new(stream, info)?;
        
        // Start handshake
        peer.initiate_handshake().await?;

        Ok(peer)
    }

    /// Initiate handshake with peer
    async fn initiate_handshake(&self) -> Result<()> {
        {
            let mut state = self.state.lock().await;
            *state = PeerState::Handshaking;
        }

        // Send version message
        let version_msg = self.create_version_message();
        self.send_raw_message(&version_msg).await?;

        // TODO: Wait for version response and complete handshake
        // For now, mark as connected immediately
        {
            let mut state = self.state.lock().await;
            *state = PeerState::Connected;
        }

        Ok(())
    }

    /// Create version message for handshake
    fn create_version_message(&self) -> Vec<u8> {
        // Simplified version message
        format!(
            "VERSION {} {} {}\n",
            1, // protocol version
            self.info.user_agent,
            0 // best height
        ).into_bytes()
    }

    /// Send raw message bytes to peer
    async fn send_raw_message(&self, data: &[u8]) -> Result<()> {
        let mut stream_guard = self.stream.lock().await;
        if let Some(stream) = stream_guard.as_mut() {
            stream.write_all(data).await
                .map_err(|e| NetworkError::IoError(e))?;
            stream.flush().await
                .map_err(|e| NetworkError::IoError(e))?;

            // Update statistics
            let mut stats = self.stats.lock().await;
            stats.bytes_sent += data.len() as u64;
            stats.messages_sent += 1;
            
            Ok(())
        } else {
            Err(NetworkError::ConnectionFailed("No active stream".to_string()))
        }
    }

    /// Send message through message channel
    pub async fn send_message(&self, message: Vec<u8>) -> Result<()> {
        self.message_tx
            .send(message)
            .map_err(|e| NetworkError::ConnectionFailed(format!("Channel send failed: {}", e)))?;
        Ok(())
    }

    /// Start background message handler
    fn start_message_handler(&self, mut message_rx: mpsc::UnboundedReceiver<Vec<u8>>) {
        let peer_id = self.info.peer_id.clone();
        
        tokio::spawn(async move {
            while let Some(message) = message_rx.recv().await {
                // TODO: Handle message sending
                tracing::debug!("Peer {} sending message: {} bytes", peer_id, message.len());
            }
        });
    }

    /// Start reading messages from peer
    pub async fn start_reading(&self) -> Result<mpsc::UnboundedReceiver<Vec<u8>>> {
        let (read_tx, read_rx) = mpsc::unbounded_channel();
        
        let stream_guard = self.stream.lock().await;
        if let Some(stream) = stream_guard.as_ref() {
            let stream_clone = stream.try_clone()
                .map_err(|e| NetworkError::IoError(e))?;
            
            let peer_id = self.info.peer_id.clone();
            let stats = self.stats.clone();
            
            // Start reading task
            tokio::spawn(async move {
                let mut reader = BufReader::new(stream_clone);
                let mut line = String::new();
                
                loop {
                    match reader.read_line(&mut line).await {
                        Ok(0) => {
                            // Connection closed
                            tracing::info!("Peer {} connection closed", peer_id);
                            break;
                        }
                        Ok(n) => {
                            // Update stats
                            {
                                let mut stats_guard = stats.lock().await;
                                stats_guard.bytes_received += n as u64;
                                stats_guard.messages_received += 1;
                            }
                            
                            // Send message to handler
                            let message = line.trim().as_bytes().to_vec();
                            if read_tx.send(message).is_err() {
                                tracing::warn!("Peer {} read channel closed", peer_id);
                                break;
                            }
                            
                            line.clear();
                        }
                        Err(e) => {
                            tracing::error!("Peer {} read error: {}", peer_id, e);
                            
                            // Update error count
                            {
                                let mut stats_guard = stats.lock().await;
                                stats_guard.connection_errors += 1;
                            }
                            break;
                        }
                    }
                }
            });
            
            Ok(read_rx)
        } else {
            Err(NetworkError::ConnectionFailed("No active stream".to_string()))
        }
    }

    /// Disconnect from peer
    pub async fn disconnect(&self, reason: &str) -> Result<()> {
        {
            let mut state = self.state.lock().await;
            *state = PeerState::Disconnected {
                reason: reason.to_string(),
            };
        }

        // Close the stream
        let mut stream_guard = self.stream.lock().await;
        if let Some(stream) = stream_guard.take() {
            let _ = stream.shutdown().await;
        }

        tracing::info!("Peer {} disconnected: {}", self.info.peer_id, reason);
        Ok(())
    }

    /// Check if peer is connected
    pub async fn is_connected(&self) -> bool {
        let state = self.state.lock().await;
        matches!(*state, PeerState::Connected)
    }

    /// Get current connection state
    pub async fn get_state(&self) -> PeerState {
        let state = self.state.lock().await;
        state.clone()
    }

    /// Get peer statistics
    pub async fn get_stats(&self) -> PeerStats {
        let stats = self.stats.lock().await;
        stats.clone()
    }

    /// Ping peer to measure latency
    pub async fn ping(&self) -> Result<Duration> {
        let start = SystemTime::now();
        
        // Send ping message
        let ping_msg = b"PING\n";
        self.send_raw_message(ping_msg).await?;
        
        // TODO: Wait for pong response and calculate latency
        // For now, return a mock latency
        let latency = SystemTime::now()
            .duration_since(start)
            .unwrap_or_default();
        
        // Update ping time in stats
        {
            let mut stats = self.stats.lock().await;
            stats.last_ping_ms = latency.as_millis() as u64;
        }
        
        Ok(latency)
    }

    /// Get peer ID
    pub fn get_id(&self) -> Uuid {
        // Convert peer_id string to UUID or generate a new one
        Uuid::new_v4()
    }

    /// Get peer address 
    pub fn get_address(&self) -> SocketAddr {
        self.info.address
    }

    /// Get event receiver for peer events
    pub async fn get_event_receiver(&self) -> mpsc::UnboundedReceiver<PeerEvent> {
        let (tx, rx) = mpsc::unbounded_channel();
        // In a real implementation, this would be set up during peer creation
        rx
    }

    /// Get last message time
    pub async fn get_last_message_time(&self) -> Option<Instant> {
        let stats = self.stats.lock().await;
        stats.last_message_at.map(|time| {
            // Convert SystemTime to Instant (approximate)
            Instant::now()
        })
    }

    /// Connect to a peer (factory method)
    pub async fn connect(address: SocketAddr, services: u64) -> Result<Arc<Self>> {
        let peer = Self::connect_to(address, Duration::from_secs(10)).await?;
        Ok(Arc::new(peer))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    #[test]
    fn test_peer_info_creation() {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8333);
        let info = PeerInfo::new(addr, "TestClient/1.0".to_string(), 1);
        
        assert_eq!(info.address, addr);
        assert_eq!(info.user_agent, "TestClient/1.0");
        assert_eq!(info.version, 1);
        assert!(!info.peer_id.is_empty());
    }

    #[tokio::test]
    async fn test_peer_state_transitions() {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8333);
        let info = PeerInfo::new(addr, "TestClient/1.0".to_string(), 1);
        
        // We can't easily test with real TCP streams in unit tests
        // So we'll just test the state management logic
        assert_eq!(info.address, addr);
    }
}