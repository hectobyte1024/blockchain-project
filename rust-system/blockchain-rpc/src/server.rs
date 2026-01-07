//! RPC server - exposes blockchain node functionality via JSON-RPC

use crate::{RpcRequest, RpcResponse, RpcError, methods};
use jsonrpc_http_server::{Server, ServerBuilder, DomainsValidation};
use jsonrpc_core::{IoHandler, Params, Value};
use std::sync::{Arc, Mutex};
use serde_json::json;
use blockchain_core::{block::Block, transaction::Transaction};

/// RPC server configuration
pub struct RpcServerConfig {
    pub host: String,
    pub port: u16,
}

impl Default for RpcServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 8545,
        }
    }
}

/// Blockchain state shared by RPC handlers
pub struct BlockchainState {
    pub block_height: Arc<Mutex<u64>>,
    pub balances: Arc<Mutex<std::collections::HashMap<String, u64>>>,
    pub transactions: Arc<Mutex<std::collections::HashMap<String, serde_json::Value>>>,
    pub tx_sender: Option<tokio::sync::mpsc::Sender<String>>,
}

impl Default for BlockchainState {
    fn default() -> Self {
        Self {
            block_height: Arc::new(Mutex::new(0)),
            balances: Arc::new(Mutex::new(std::collections::HashMap::new())),
            transactions: Arc::new(Mutex::new(std::collections::HashMap::new())),
            tx_sender: None,
        }
    }
}

impl BlockchainState {
    pub fn with_tx_sender(mut self, sender: tokio::sync::mpsc::Sender<String>) -> Self {
        self.tx_sender = Some(sender);
        self
    }
}

/// RPC server that exposes blockchain functionality
pub struct RpcServer {
    config: RpcServerConfig,
    handler: IoHandler,
    state: Arc<BlockchainState>,
}

impl RpcServer {
    /// Create new RPC server
    pub fn new(config: RpcServerConfig) -> Self {
        let state = Arc::new(BlockchainState::default());
        let mut handler = IoHandler::new();
        
        // Register all RPC methods
        Self::register_methods(&mut handler, state.clone());
        
        Self { config, handler, state }
    }
    
    /// Create new RPC server with custom state
    pub fn with_state(config: RpcServerConfig, state: Arc<BlockchainState>) -> Self {
        let mut handler = IoHandler::new();
        
        // Register all RPC methods
        Self::register_methods(&mut handler, state.clone());
        
        Self { config, handler, state }
    }
    
    /// Create new RPC server with custom handler (for blockchain integration)
    pub fn with_custom_handler(config: RpcServerConfig, handler: IoHandler) -> Self {
        let state = Arc::new(BlockchainState::default());
        Self { config, handler, state }
    }
    
    /// Register all blockchain RPC methods
    fn register_methods(io: &mut IoHandler, state: Arc<BlockchainState>) {
        // Get block height
        let state_clone = state.clone();
        io.add_sync_method(methods::GET_BLOCK_HEIGHT, move |_params: Params| {
            let height = state_clone.block_height.lock().unwrap();
            Ok(Value::Number((*height).into()))
        });
        
        // Get balance
        let state_clone = state.clone();
        io.add_sync_method(methods::GET_BALANCE, move |params: Params| {
            let parsed: Vec<String> = params.parse()?;
            if parsed.is_empty() {
                return Err(jsonrpc_core::Error::invalid_params("Missing address"));
            }
            let address = &parsed[0];
            
            let balances = state_clone.balances.lock().unwrap();
            let balance = balances.get(address).copied().unwrap_or(0);
            
            Ok(Value::Number(balance.into()))
        });
        
        // Send transaction
        let state_clone = state.clone();
        io.add_sync_method(methods::SEND_TRANSACTION, move |params: Params| {
            let parsed: Vec<String> = params.parse()?;
            if parsed.is_empty() {
                return Err(jsonrpc_core::Error::invalid_params("Missing transaction hex"));
            }
            let tx_hex = &parsed[0];
            
            // Decode transaction from hex
            let tx_bytes = hex::decode(tx_hex)
                .map_err(|_| jsonrpc_core::Error::invalid_params("Invalid hex"))?;
            let tx_json: serde_json::Value = serde_json::from_slice(&tx_bytes)
                .map_err(|_| jsonrpc_core::Error::invalid_params("Invalid transaction JSON"))?;
            
            // Extract transaction details
            let tx_obj = tx_json.get("transaction")
                .ok_or_else(|| jsonrpc_core::Error::invalid_params("Missing transaction field"))?;
            
            let from = tx_obj.get("from").and_then(|v| v.as_str())
                .ok_or_else(|| jsonrpc_core::Error::invalid_params("Missing from address"))?;
            let to = tx_obj.get("to").and_then(|v| v.as_str())
                .ok_or_else(|| jsonrpc_core::Error::invalid_params("Missing to address"))?;
            let amount = tx_obj.get("amount").and_then(|v| v.as_u64())
                .ok_or_else(|| jsonrpc_core::Error::invalid_params("Missing amount"))?;
            
            // Generate transaction hash
            use sha2::{Digest, Sha256};
            let tx_hash = format!("0x{}", hex::encode(Sha256::digest(tx_hex.as_bytes())));
            
            // Store transaction with details
            let mut transactions = state_clone.transactions.lock().unwrap();
            transactions.insert(tx_hash.clone(), json!({
                "hash": tx_hash.clone(),
                "from": from,
                "to": to,
                "amount": amount,
                "status": "pending",
                "timestamp": chrono::Utc::now().timestamp()
            }));
            
            // Send to miner if tx_sender exists
            if let Some(ref tx_sender) = state_clone.tx_sender {
                let _ = tx_sender.try_send(tx_hash.clone());
            }
            
            Ok(Value::String(tx_hash))
        });
        
        // Get transaction
        let state_clone = state.clone();
        io.add_sync_method(methods::GET_TRANSACTION, move |params: Params| {
            let parsed: Vec<String> = params.parse()?;
            if parsed.is_empty() {
                return Err(jsonrpc_core::Error::invalid_params("Missing transaction hash"));
            }
            let tx_hash = &parsed[0];
            
            let transactions = state_clone.transactions.lock().unwrap();
            if let Some(tx) = transactions.get(tx_hash) {
                Ok(tx.clone())
            } else {
                Ok(json!({
                    "error": "Transaction not found"
                }))
            }
        });
        
        // Get block
        let state_clone = state.clone();
        io.add_sync_method(methods::GET_BLOCK, move |params: Params| {
            let parsed: Vec<u64> = params.parse()?;
            if parsed.is_empty() {
                return Err(jsonrpc_core::Error::invalid_params("Missing block height"));
            }
            let height = parsed[0];
            
            let current_height = *state_clone.block_height.lock().unwrap();
            if height > current_height {
                return Ok(json!({
                    "error": "Block not found"
                }));
            }
            
            Ok(json!({
                "height": height,
                "hash": format!("0x{:064x}", height),
                "prev_hash": format!("0x{:064x}", height.saturating_sub(1)),
                "timestamp": chrono::Utc::now().timestamp(),
                "transactions": []
            }))
        });
        
        // Get network info
        io.add_sync_method(methods::GET_NETWORK_INFO, |_params: Params| {
            Ok(json!({
                "peers": 0,
                "network": "edunet",
                "version": "1.0.0",
                "protocol_version": 1
            }))
        });
        
        // Get mempool info
        let state_clone = state.clone();
        io.add_sync_method(methods::GET_MEMPOOL_INFO, move |_params: Params| {
            let transactions = state_clone.transactions.lock().unwrap();
            let pending_count = transactions.values()
                .filter(|tx| tx.get("status").and_then(|s| s.as_str()) == Some("pending"))
                .count();
            
            Ok(json!({
                "size": pending_count,
                "bytes": pending_count * 250, // Estimate
                "usage": pending_count * 250
            }))
        });
        
        // Get mining info
        let state_clone = state.clone();
        io.add_sync_method(methods::GET_MINING_INFO, move |_params: Params| {
            let height = *state_clone.block_height.lock().unwrap();
            Ok(json!({
                "mining": true,
                "hashrate": 0,
                "difficulty": 4,
                "blocks_mined": height,
                "network_hashrate": 0
            }))
        });
        
        // Get sync status
        let state_clone = state.clone();
        io.add_sync_method(methods::GET_SYNC_STATUS, move |_params: Params| {
            let height = *state_clone.block_height.lock().unwrap();
            Ok(json!({
                "syncing": false,
                "current_block": height,
                "highest_block": height,
                "peers": 0
            }))
        });
        
        // Credit balance directly (for vouchers/airdrops)
        let state_clone = state.clone();
        io.add_sync_method(methods::CREDIT_BALANCE, move |params: Params| {
            let parsed: Vec<serde_json::Value> = params.parse()?;
            if parsed.len() < 2 {
                return Err(jsonrpc_core::Error::invalid_params("Missing address or amount"));
            }
            
            let address = parsed[0].as_str()
                .ok_or_else(|| jsonrpc_core::Error::invalid_params("Invalid address"))?;
            let amount = parsed[1].as_u64()
                .ok_or_else(|| jsonrpc_core::Error::invalid_params("Invalid amount"))?;
            
            // Directly credit balance
            let mut balances = state_clone.balances.lock().unwrap();
            *balances.entry(address.to_string()).or_insert(0) += amount;
            
            Ok(json!({
                "success": true,
                "address": address,
                "credited": amount,
                "new_balance": balances.get(address).copied().unwrap_or(0)
            }))
        });
    }
    
    /// Start RPC server (blocking)
    pub fn start(self) -> Result<Server, String> {
        let addr = format!("{}:{}", self.config.host, self.config.port);
        
        println!("🚀 Starting RPC server on {}", addr);
        
        ServerBuilder::new(self.handler)
            .threads(4)
            .cors(DomainsValidation::AllowOnly(vec![
                jsonrpc_http_server::AccessControlAllowOrigin::Any
            ]))
            .start_http(&addr.parse().map_err(|e| format!("Invalid address: {}", e))?)
            .map_err(|e| format!("Failed to start RPC server: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_server_creation() {
        let config = RpcServerConfig::default();
        let server = RpcServer::new(config);
        assert_eq!(server.config.port, 8545);
    }
}
