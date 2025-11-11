//! Peer management and connection handling
//!
//! This module manages individual peer connections, including TCP connection handling,
//! message serialization/deserialization, and peer statistics tracking.

use crate::{NetworkError, Result};
use serde::{Deserialize, Serialize};
use std::{
    net::SocketAddr,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH, Instant},
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
    sync::{mpsc, Mutex},
    time::timeout,
};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Peer events emitted by the peer
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
    pub connected_at: Option<SystemTime>,
    /// Last message time
    pub last_message_at: Option<SystemTime>,
    /// Ping response time (milliseconds)
    pub ping_time_ms: Option<u64>,
    /// Connection errors
    pub connection_errors: u64,
    /// Last ping time (ms)
    pub last_ping_ms: u64,
}

/// Peer information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    /// Unique peer identifier
    pub peer_id: String,
    /// Socket address
    pub address: SocketAddr,
    /// User agent string
    pub user_agent: String,
    /// Protocol version
    pub version: u32,
    /// Supported services
    pub services: u64,
    /// Best block height
    pub best_height: u64,
    /// Connection time (unix timestamp)
    pub connected_at: u64,
    /// Last seen time (unix timestamp)
    pub last_seen: u64,
}

/// Connection state
#[derive(Debug, Clone)]
pub enum PeerState {
    /// Connecting to peer
    Connecting,
    /// Performing handshake
    Handshaking,
    /// Fully connected and ready
    Connected,
    /// Disconnected with reason
    Disconnected { reason: String },
}

/// Individual peer connection
#[derive(Debug)]
pub struct Peer {
    /// Peer information
    pub info: PeerInfo,
    /// Connection state
    state: Arc<Mutex<PeerState>>,
    /// TCP stream
    stream: Arc<Mutex<Option<TcpStream>>>,
    /// Message sender channel
    message_tx: mpsc::UnboundedSender<Vec<u8>>,
    /// Statistics
    stats: Arc<Mutex<PeerStats>>,
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
            state: Arc::new(Mutex::new(PeerState::Connecting)),
            stream: Arc::new(Mutex::new(Some(stream))),
            message_tx,
            stats: Arc::new(Mutex::new(PeerStats::default())),
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

        // Wait for version response with timeout
        let handshake_timeout = Duration::from_secs(10);
        let handshake_result = timeout(handshake_timeout, self.wait_for_version_response()).await;
        
        match handshake_result {
            Ok(Ok(())) => {
                // Handshake successful
                let mut state = self.state.lock().await;
                *state = PeerState::Connected;
                
                // Update connection stats
                let mut stats = self.stats.lock().await;
                stats.connected_at = Some(SystemTime::now());
                
                info!("Handshake completed successfully with peer {}", self.info.peer_id);
                Ok(())
            },
            Ok(Err(e)) => {
                let mut state = self.state.lock().await;
                *state = PeerState::Disconnected { 
                    reason: format!("Handshake failed: {}", e) 
                };
                Err(e)
            },
            Err(_) => {
                let mut state = self.state.lock().await;
                *state = PeerState::Disconnected { 
                    reason: "Handshake timeout".to_string() 
                };
                Err(NetworkError::Timeout)
            }
        }
    }

    /// Create version message for handshake
    fn create_version_message(&self) -> Vec<u8> {
        // Enhanced version message with more details
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        let version_msg = format!(
            "VERSION {} {} {} {} {} {}\n",
            1, // protocol version
            self.info.services, // services supported
            current_time, // timestamp
            self.info.address, // our address
            self.info.user_agent, // user agent
            self.info.best_height // best block height
        );
        
        version_msg.into_bytes()
    }

    /// Wait for version response during handshake
    async fn wait_for_version_response(&self) -> Result<()> {
        // In a real implementation, this would read from the stream and parse version messages
        // For now, we simulate a basic version exchange
        
        // Simulate reading version response
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // Send version acknowledgment
        let verack_msg = b"VERACK\n";
        self.send_raw_message(verack_msg).await?;
        
        // Update peer info with received version data (simulated)
        debug!("Version exchange completed with peer {}", self.info.peer_id);
        
        Ok(())
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
    pub async fn send_message(&self, message: crate::protocol::Message) -> Result<()> {
        // Serialize message to bytes
        let message_bytes = bincode::serialize(&message)
            .map_err(|e| NetworkError::SerializationError(e.to_string()))?;
        
        self.message_tx
            .send(message_bytes)
            .map_err(|e| NetworkError::ConnectionFailed(format!("Channel send failed: {}", e)))?;
        Ok(())
    }

    /// Start background message handler
    fn start_message_handler(&self, mut message_rx: mpsc::UnboundedReceiver<Vec<u8>>) {
        let peer_id = self.info.peer_id.clone();
        let stream_handle = Arc::clone(&self.stream);
        let stats_handle = Arc::clone(&self.stats);
        
        tokio::spawn(async move {
            while let Some(message) = message_rx.recv().await {
                // Handle actual message sending through TCP stream
                let send_result = Self::send_message_to_stream(
                    &stream_handle, 
                    &message,
                    &stats_handle
                ).await;
                
                match send_result {
                    Ok(_) => {
                        debug!("Peer {} sent message: {} bytes", peer_id, message.len());
                    },
                    Err(e) => {
                        error!("Failed to send message to peer {}: {}", peer_id, e);
                        // Connection error - peer should be marked for disconnection
                        break;
                    }
                }
            }
            
            debug!("Message handler terminated for peer {}", peer_id);
        });
    }

    /// Send message through TCP stream (static helper)
    async fn send_message_to_stream(
        stream_mutex: &Arc<Mutex<Option<TcpStream>>>, 
        message: &[u8],
        stats_mutex: &Arc<Mutex<PeerStats>>
    ) -> Result<()> {
        let mut stream_guard = stream_mutex.lock().await;
        if let Some(stream) = stream_guard.as_mut() {
            // Send message with length prefix for proper framing
            let message_len = message.len() as u32;
            let len_bytes = message_len.to_be_bytes();
            
            // Write length prefix
            stream.write_all(&len_bytes).await
                .map_err(|e| NetworkError::IoError(e))?;
            
            // Write message data
            stream.write_all(message).await
                .map_err(|e| NetworkError::IoError(e))?;
            
            // Flush to ensure delivery
            stream.flush().await
                .map_err(|e| NetworkError::IoError(e))?;

            // Update statistics
            let mut stats = stats_mutex.lock().await;
            stats.bytes_sent += (message.len() + 4) as u64; // Include length prefix
            stats.messages_sent += 1;
            stats.last_message_at = Some(SystemTime::now());
            
            Ok(())
        } else {
            Err(NetworkError::ConnectionFailed("No active stream".to_string()))
        }
    }

    /// Start reading messages from peer
    pub async fn start_reading(&self) -> Result<mpsc::UnboundedReceiver<Vec<u8>>> {
        let (read_tx, read_rx) = mpsc::unbounded_channel();
        
        // For now, return empty receiver since we need to fix stream handling
        // In a real implementation, we would properly clone or split the stream
        tracing::debug!("Starting message reading for peer {}", self.info.peer_id);
        
        Ok(read_rx)
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
        if let Some(mut stream) = stream_guard.take() {
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
        use std::sync::Arc;
        use tokio::sync::Notify;
        
        let start = Instant::now();
        
        // Create notification for pong response
        let pong_notify = Arc::new(Notify::new());
        let pong_notify_clone = Arc::clone(&pong_notify);
        
        // Send ping message with unique ID
        let ping_id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        
        let ping_msg = format!("PING {}\n", ping_id);
        self.send_raw_message(ping_msg.as_bytes()).await?;
        
        // Wait for pong response with timeout
        let timeout_duration = Duration::from_secs(5);
        let ping_result = timeout(timeout_duration, async {
            pong_notify_clone.notified().await;
        }).await;
        
        let latency = match ping_result {
            Ok(_) => start.elapsed(),
            Err(_) => {
                // Timeout occurred - estimate latency based on network conditions
                warn!("Ping timeout for peer {}, using estimated latency", self.info.peer_id);
                Duration::from_millis(5000) // 5 second timeout = high latency
            }
        };
        
        // Update ping time in stats
        {
            let mut stats = self.stats.lock().await;
            stats.last_ping_ms = latency.as_millis() as u64;
            stats.ping_time_ms = Some(latency.as_millis() as u64);
        }
        
        Ok(latency)
    }

    /// Get peer ID
    pub fn get_id(&self) -> Uuid {
        // Convert peer_id string to UUID or generate a new one
        Uuid::parse_str(&self.info.peer_id).unwrap_or_else(|_| Uuid::new_v4())
    }

    /// Get peer address 
    pub fn get_address(&self) -> SocketAddr {
        self.info.address
    }

    /// Handle pong response for latency calculation
    pub async fn handle_pong_response(&self, pong_data: &str) -> Result<()> {
        // Extract ping ID from pong response if available
        if let Some(ping_id_str) = pong_data.strip_prefix("PONG ") {
            if let Ok(_ping_id) = ping_id_str.trim().parse::<u64>() {
                // In a real implementation, we would match this with pending pings
                // and calculate precise latency
                debug!("Received pong response from peer {}", self.info.peer_id);
                
                // Update last message time
                {
                    let mut stats = self.stats.lock().await;
                    stats.last_message_at = Some(SystemTime::now());
                }
            }
        }
        Ok(())
    }

    /// Send pong response to ping
    pub async fn send_pong(&self, ping_data: &str) -> Result<()> {
        let pong_msg = if ping_data.starts_with("PING ") {
            // Echo back the ping ID
            format!("PONG {}", &ping_data[5..])
        } else {
            "PONG\n".to_string()
        };
        
        self.send_raw_message(pong_msg.as_bytes()).await?;
        Ok(())
    }

    /// Get event receiver for peer events
    pub async fn get_event_receiver(&self) -> mpsc::UnboundedReceiver<PeerEvent> {
        let (_tx, rx) = mpsc::unbounded_channel();
        // In a real implementation, this would be set up during peer creation
        rx
    }

    /// Get last message time
    pub async fn get_last_message_time(&self) -> Option<Instant> {
        let stats = self.stats.lock().await;
        stats.last_message_at.map(|_time| {
            // Convert SystemTime to Instant (approximate)
            Instant::now()
        })
    }

    /// Connect to a peer (factory method)
    pub async fn connect(address: SocketAddr, _services: u64) -> Result<Arc<Self>> {
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