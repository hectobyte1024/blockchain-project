//! Blockchain protocol messages for P2P communication
//!
//! This module defines the message format and protocol for communication
//! between blockchain nodes in the hybrid network.

use crate::{NetworkError, Result};
use blockchain_core::{Hash256, BlockHeight};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// Magic bytes for message headers (network identification)
pub const MAINNET_MAGIC: u32 = 0xD9B4BEF9;
pub const TESTNET_MAGIC: u32 = 0x0709110B;

/// Maximum message payload size (32MB)
pub const MAX_PAYLOAD_SIZE: usize = 32 * 1024 * 1024;

/// Protocol message types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MessageType {
    /// Version handshake message
    Version,
    /// Version acknowledgment
    VerAck,
    /// Ping for keep-alive
    Ping,
    /// Pong response to ping
    Pong,
    /// Request peer addresses
    GetAddr,
    /// Peer address list
    Addr,
    /// Request block inventory
    GetBlocks,
    /// Request specific blocks
    GetData,
    /// Block data
    Block,
    /// Transaction data
    Tx,
    /// Transaction inventory
    Inv,
    /// Memory pool request
    MemPool,
    /// Reject message
    Reject,
    /// Alert message
    Alert,
    /// Request blockchain height
    GetBlockchainHeight,
    /// Response with blockchain height
    BlockchainHeight,
    /// Request block by height
    GetBlockByHeight,
    /// Block data response
    BlockData,
    /// Request block headers
    GetHeaders,
    /// Block headers response
    Headers,
    /// Not found response
    NotFound,
}

/// Main protocol message structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Message type
    pub message_type: MessageType,
    /// Message payload
    pub payload: MessagePayload,
    /// Timestamp when message was created
    pub timestamp: u64,
    /// Message checksum for integrity
    pub checksum: u32,
}

/// Message payload variants
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessagePayload {
    /// Version handshake
    Version(VersionMessage),
    /// Version acknowledgment (no data)
    VerAck,
    /// Ping with nonce
    Ping(PingMessage),
    /// Pong response with nonce
    Pong(PongMessage),
    /// Address request (no data)
    GetAddr,
    /// Address list
    Addr(AddrMessage),
    /// Block inventory request
    GetBlocks(GetBlocksMessage),
    /// Data request
    GetData(GetDataMessage),
    /// Block data
    Block(BlockMessage),
    /// Transaction data
    Tx(TxMessage),
    /// Inventory announcement
    Inv(InvMessage),
    /// Memory pool request (no data)
    MemPool,
    /// Rejection message
    Reject(RejectMessage),
    /// Alert message
    Alert(AlertMessage),
    /// Request blockchain height (no data)
    GetBlockchainHeight,
    /// Blockchain height response
    BlockchainHeight(BlockchainHeightMessage),
    /// Request block by height
    GetBlockByHeight(GetBlockByHeightMessage),
    /// Block data response
    BlockData(BlockDataMessage),
    /// Request block headers
    GetHeaders(GetHeadersMessage),
    /// Block headers response
    Headers(HeadersMessage),
    /// Not found response
    NotFound(NotFoundMessage),
}

/// Version handshake message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionMessage {
    /// Protocol version
    pub version: u32,
    /// Services supported by this node
    pub services: u64,
    /// Current timestamp
    pub timestamp: u64,
    /// Remote peer address
    pub addr_recv: NetworkAddress,
    /// Local address
    pub addr_from: NetworkAddress,
    /// Random nonce for connection
    pub nonce: u64,
    /// User agent string
    pub user_agent: String,
    /// Latest block height
    pub start_height: BlockHeight,
    /// Whether to relay transactions
    pub relay: bool,
}

/// Network address structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkAddress {
    /// Services provided by this address
    pub services: u64,
    /// IPv6 address (IPv4 mapped if needed)
    pub ip: [u8; 16],
    /// Port number
    pub port: u16,
    /// Timestamp when address was last seen
    pub timestamp: u64,
}

/// Ping message for keep-alive
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingMessage {
    /// Random nonce
    pub nonce: u64,
}

/// Pong response message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PongMessage {
    /// Nonce from ping message
    pub nonce: u64,
}

/// Address list message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddrMessage {
    /// List of known addresses
    pub addresses: Vec<NetworkAddress>,
}

/// Block inventory request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetBlocksMessage {
    /// Protocol version
    pub version: u32,
    /// Block locator hashes (most recent first)
    pub block_locator_hashes: Vec<Hash256>,
    /// Hash to stop at (zero hash for no limit)
    pub hash_stop: Hash256,
}

/// Data request message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetDataMessage {
    /// Inventory items to request
    pub inventory: Vec<InventoryItem>,
}

/// Block message containing block data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockMessage {
    /// Block hash for identification
    pub block_hash: Hash256,
    /// Serialized block data
    pub block_data: Vec<u8>,
}

/// Transaction message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxMessage {
    /// Transaction hash
    pub tx_hash: Hash256,
    /// Serialized transaction data
    pub tx_data: Vec<u8>,
}

/// Inventory message for announcing available data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvMessage {
    /// Inventory items being announced
    pub inventory: Vec<InventoryItem>,
}

/// Inventory item type and identifier
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryItem {
    /// Type of inventory item
    pub item_type: InventoryType,
    /// Hash identifier
    pub hash: Hash256,
}

/// Inventory item types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum InventoryType {
    /// Transaction
    Tx,
    /// Block
    Block,
    /// Filtered block (Merkle block)
    FilteredBlock,
    /// Compact block
    CompactBlock,
}

/// Rejection message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RejectMessage {
    /// Message type being rejected
    pub message: String,
    /// Rejection reason code
    pub ccode: u8,
    /// Human-readable reason
    pub reason: String,
    /// Extra data (optional)
    pub data: Vec<u8>,
}

/// Alert message for network-wide notifications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertMessage {
    /// Alert version
    pub version: u32,
    /// Relay until timestamp
    pub relay_until: u64,
    /// Expiration timestamp
    pub expiration: u64,
    /// Alert ID
    pub id: u32,
    /// Cancel alert IDs
    pub cancel: Vec<u32>,
    /// Minimum version for alert
    pub min_ver: u32,
    /// Maximum version for alert
    pub max_ver: u32,
    /// User agent pattern
    pub sub_ver: String,
    /// Priority level
    pub priority: u32,
    /// Alert comment
    pub comment: String,
    /// Status bar message
    pub status_bar: String,
}

impl Message {
    /// Create new message
    pub fn new(message_type: MessageType, payload: MessagePayload) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            message_type,
            payload,
            timestamp,
            checksum: 0, // Will be calculated during serialization
        }
    }

    /// Create version message
    pub fn version(
        protocol_version: u32,
        services: u64,
        user_agent: String,
        start_height: BlockHeight,
        remote_addr: NetworkAddress,
        local_addr: NetworkAddress,
    ) -> Self {
        let version = VersionMessage {
            version: protocol_version,
            services,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            addr_recv: remote_addr,
            addr_from: local_addr,
            nonce: rand::random(),
            user_agent,
            start_height,
            relay: true,
        };

        Self::new(MessageType::Version, MessagePayload::Version(version))
    }

    /// Create verack message
    pub fn verack() -> Self {
        Self::new(MessageType::VerAck, MessagePayload::VerAck)
    }

    /// Create ping message
    pub fn ping() -> Self {
        let ping = PingMessage {
            nonce: rand::random(),
        };
        Self::new(MessageType::Ping, MessagePayload::Ping(ping))
    }

    /// Create pong message
    pub fn pong(nonce: u64) -> Self {
        let pong = PongMessage { nonce };
        Self::new(MessageType::Pong, MessagePayload::Pong(pong))
    }

    /// Create getaddr message
    pub fn getaddr() -> Self {
        Self::new(MessageType::GetAddr, MessagePayload::GetAddr)
    }

    /// Create addr message
    pub fn addr(addresses: Vec<NetworkAddress>) -> Self {
        let addr = AddrMessage { addresses };
        Self::new(MessageType::Addr, MessagePayload::Addr(addr))
    }

    /// Create block message
    pub fn block(block_hash: Hash256, block_data: Vec<u8>) -> Self {
        let block = BlockMessage {
            block_hash,
            block_data,
        };
        Self::new(MessageType::Block, MessagePayload::Block(block))
    }

    /// Create transaction message
    pub fn tx(tx_hash: Hash256, tx_data: Vec<u8>) -> Self {
        let tx = TxMessage { tx_hash, tx_data };
        Self::new(MessageType::Tx, MessagePayload::Tx(tx))
    }

    /// Create inventory message
    pub fn inv(inventory: Vec<InventoryItem>) -> Self {
        let inv = InvMessage { inventory };
        Self::new(MessageType::Inv, MessagePayload::Inv(inv))
    }

    /// Serialize message to bytes
    pub fn serialize(&self) -> Result<Vec<u8>> {
        bincode::serialize(self)
            .map_err(|e| NetworkError::SerializationError(e.to_string()))
    }

    /// Deserialize message from bytes
    pub fn deserialize(data: &[u8]) -> Result<Self> {
        bincode::deserialize(data)
            .map_err(|e| NetworkError::SerializationError(e.to_string()))
    }

    /// Calculate message checksum
    pub fn calculate_checksum(&self) -> u32 {
        // Simple checksum calculation for message integrity
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        format!("{:?}", self).hash(&mut hasher);
        hasher.finish() as u32
    }

    /// Validate message integrity
    pub fn validate(&self) -> bool {
        // Check payload size
        if let Ok(serialized) = self.serialize() {
            if serialized.len() > MAX_PAYLOAD_SIZE {
                return false;
            }
        }

        // Additional validation logic can be added here
        true
    }
}

impl NetworkAddress {
    /// Create new network address
    pub fn new(ip: [u8; 16], port: u16, services: u64) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            services,
            ip,
            port,
            timestamp,
        }
    }

    /// Create from IPv4 address
    pub fn from_ipv4(ip: [u8; 4], port: u16, services: u64) -> Self {
        // Map IPv4 to IPv6
        let mut ipv6 = [0u8; 16];
        ipv6[10] = 0xFF;
        ipv6[11] = 0xFF;
        ipv6[12..16].copy_from_slice(&ip);

        Self::new(ipv6, port, services)
    }

    /// Check if address is IPv4
    pub fn is_ipv4(&self) -> bool {
        self.ip[..10] == [0; 10] && self.ip[10..12] == [0xFF; 2]
    }

    /// Get IPv4 address if applicable
    pub fn get_ipv4(&self) -> Option<[u8; 4]> {
        if self.is_ipv4() {
            let mut ipv4 = [0u8; 4];
            ipv4.copy_from_slice(&self.ip[12..16]);
            Some(ipv4)
        } else {
            None
        }
    }
}

impl InventoryItem {
    /// Create new inventory item
    pub fn new(item_type: InventoryType, hash: Hash256) -> Self {
        Self { item_type, hash }
    }

    /// Create transaction inventory item
    pub fn tx(hash: Hash256) -> Self {
        Self::new(InventoryType::Tx, hash)
    }

    /// Create block inventory item
    pub fn block(hash: Hash256) -> Self {
        Self::new(InventoryType::Block, hash)
    }
}

/// Service flags for peer capabilities
pub mod services {
    /// Node can serve full blocks
    pub const NODE_NETWORK: u64 = 1 << 0;
    /// Node can serve UTXO set
    pub const NODE_UTXO: u64 = 1 << 1;
    /// Node can serve bloom-filtered blocks
    pub const NODE_BLOOM: u64 = 1 << 2;
    /// Node can serve witness data
    pub const NODE_WITNESS: u64 = 1 << 3;
    /// Node can serve compact blocks
    pub const NODE_COMPACT_FILTERS: u64 = 1 << 6;
    /// Node can serve network addresses via DNS
    pub const NODE_NETWORK_LIMITED: u64 = 1 << 10;
}

// ==================== SYNC PROTOCOL MESSAGES ====================

/// Blockchain height response message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockchainHeightMessage {
    /// Current blockchain height
    pub height: u64,
    /// Best block hash
    pub best_block_hash: Hash256,
    /// Total chain work
    pub total_work: u64,
}

/// Request block by height message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetBlockByHeightMessage {
    /// Block height to request
    pub height: u64,
}

/// Block data response message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockDataMessage {
    /// Block height
    pub height: u64,
    /// Block hash
    pub block_hash: Hash256,
    /// Serialized block data
    pub block_data: Vec<u8>,
}

/// Request block headers message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetHeadersMessage {
    /// Protocol version
    pub version: u32,
    /// Starting block height
    pub start_height: u64,
    /// Ending block height (inclusive)
    pub end_height: u64,
    /// Stop hash (optional)
    pub stop_hash: Option<Hash256>,
}

/// Block headers response message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeadersMessage {
    /// Number of headers
    pub count: u32,
    /// Block headers
    pub headers: Vec<BlockHeaderInfo>,
}

/// Block header information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockHeaderInfo {
    /// Block height
    pub height: u64,
    /// Block hash
    pub hash: Hash256,
    /// Previous block hash
    pub prev_hash: Hash256,
    /// Merkle root
    pub merkle_root: Hash256,
    /// Timestamp
    pub timestamp: u64,
    /// Difficulty target
    pub difficulty: u32,
    /// Nonce
    pub nonce: u32,
}

/// Not found message (block or transaction not found)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotFoundMessage {
    /// Type of item not found
    pub item_type: NotFoundType,
    /// Identifier (height for blocks, hash for transactions)
    pub identifier: Vec<u8>,
}

/// Not found item type
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NotFoundType {
    /// Block not found
    Block,
    /// Transaction not found
    Transaction,
    /// Block header not found
    Header,
}

impl Message {
    /// Create a GetBlockchainHeight message
    pub fn get_blockchain_height() -> Self {
        Self {
            message_type: MessageType::GetBlockchainHeight,
            payload: MessagePayload::GetBlockchainHeight,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            checksum: 0,
        }
    }

    /// Create a BlockchainHeight message
    pub fn blockchain_height(height: u64, best_block_hash: Hash256, total_work: u64) -> Self {
        Self {
            message_type: MessageType::BlockchainHeight,
            payload: MessagePayload::BlockchainHeight(BlockchainHeightMessage {
                height,
                best_block_hash,
                total_work,
            }),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            checksum: 0,
        }
    }

    /// Create a GetBlockByHeight message
    pub fn get_block_by_height(height: u64) -> Self {
        Self {
            message_type: MessageType::GetBlockByHeight,
            payload: MessagePayload::GetBlockByHeight(GetBlockByHeightMessage { height }),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            checksum: 0,
        }
    }

    /// Create a BlockData message
    pub fn block_data(height: u64, block_hash: Hash256, block_data: Vec<u8>) -> Self {
        Self {
            message_type: MessageType::BlockData,
            payload: MessagePayload::BlockData(BlockDataMessage {
                height,
                block_hash,
                block_data,
            }),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            checksum: 0,
        }
    }

    /// Create a GetHeaders message
    pub fn get_headers(version: u32, start_height: u64, end_height: u64, stop_hash: Option<Hash256>) -> Self {
        Self {
            message_type: MessageType::GetHeaders,
            payload: MessagePayload::GetHeaders(GetHeadersMessage {
                version,
                start_height,
                end_height,
                stop_hash,
            }),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            checksum: 0,
        }
    }

    /// Create a Headers message
    pub fn headers(headers: Vec<BlockHeaderInfo>) -> Self {
        Self {
            message_type: MessageType::Headers,
            payload: MessagePayload::Headers(HeadersMessage {
                count: headers.len() as u32,
                headers,
            }),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            checksum: 0,
        }
    }

    /// Create a NotFound message
    pub fn not_found(item_type: NotFoundType, identifier: Vec<u8>) -> Self {
        Self {
            message_type: MessageType::NotFound,
            payload: MessagePayload::NotFound(NotFoundMessage {
                item_type,
                identifier,
            }),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            checksum: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let msg = Message::ping();
        assert_eq!(msg.message_type, MessageType::Ping);
        
        if let MessagePayload::Ping(ping) = msg.payload {
            assert!(ping.nonce != 0);
        } else {
            panic!("Expected ping payload");
        }
    }

    #[test]
    fn test_network_address_ipv4() {
        let addr = NetworkAddress::from_ipv4([192, 168, 1, 1], 8333, services::NODE_NETWORK);
        
        assert!(addr.is_ipv4());
        assert_eq!(addr.get_ipv4(), Some([192, 168, 1, 1]));
        assert_eq!(addr.port, 8333);
        assert_eq!(addr.services, services::NODE_NETWORK);
    }

    #[test]
    fn test_inventory_item_creation() {
        let hash = [1u8; 32];
        let item = InventoryItem::tx(hash);
        
        assert_eq!(item.item_type, InventoryType::Tx);
        assert_eq!(item.hash, hash);
    }

    #[test]
    fn test_message_serialization() {
        let msg = Message::ping();
        let serialized = msg.serialize().unwrap();
        let deserialized = Message::deserialize(&serialized).unwrap();
        
        assert_eq!(msg.message_type, deserialized.message_type);
        assert_eq!(msg.timestamp, deserialized.timestamp);
    }
}