//! JSON-RPC interface for blockchain communication
//! This allows web clients to talk to blockchain nodes

use serde::{Deserialize, Serialize};

/// Standard JSON-RPC request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcRequest {
    pub jsonrpc: String,
    pub method: String,
    pub params: serde_json::Value,
    pub id: u64,
}

/// Standard JSON-RPC response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcResponse {
    pub jsonrpc: String,
    pub result: Option<serde_json::Value>,
    pub error: Option<RpcError>,
    pub id: u64,
}

/// RPC error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcError {
    pub code: i32,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

/// Blockchain RPC methods
pub mod methods {
    /// Get current block height
    pub const GET_BLOCK_HEIGHT: &str = "blockchain_getBlockHeight";
    
    /// Get balance of address
    pub const GET_BALANCE: &str = "blockchain_getBalance";
    
    /// Get transaction by hash
    pub const GET_TRANSACTION: &str = "blockchain_getTransaction";
    
    /// Send raw transaction
    pub const SEND_TRANSACTION: &str = "blockchain_sendRawTransaction";
    
    /// Get block by height
    pub const GET_BLOCK: &str = "blockchain_getBlock";
    
    /// Get network info
    pub const GET_NETWORK_INFO: &str = "blockchain_getNetworkInfo";
    
    /// Get mempool info
    pub const GET_MEMPOOL_INFO: &str = "blockchain_getMempoolInfo";
    
    /// Get mining info (for miners/nodes)
    pub const GET_MINING_INFO: &str = "blockchain_getMiningInfo";
    
    /// Get node sync status
    pub const GET_SYNC_STATUS: &str = "blockchain_getSyncStatus";
    
    /// Credit balance directly (for vouchers/airdrops)
    pub const CREDIT_BALANCE: &str = "blockchain_creditBalance";
}

pub mod client;
pub mod server;

// Re-exports for convenience
pub use client::RpcClient;
pub use server::{RpcServer, RpcServerConfig, BlockchainState};
