// REST API Endpoints & WebSocket Handler
//
// This module provides REST API endpoints for HTTP-based interactions
// and WebSocket handlers for real-time communication.

use crate::api_server::{ApiServer, JsonRpcRequest, JsonRpcResponse, SubscriptionType};
use crate::{BlockchainError, Result};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use uuid::Uuid;

// REST API Request/Response Types
#[derive(Debug, Deserialize)]
pub struct CreateWalletRequest {
    pub name: String,
    pub passphrase: Option<String>,
    pub entropy: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SendTransactionRequest {
    pub wallet_id: String,
    pub to_address: String,
    pub amount: u64,
    pub fee_rate: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct GenerateAddressRequest {
    pub wallet_id: String,
    pub account_index: Option<u32>,
    pub is_change: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub timestamp: String,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }
}

// WebSocket Message Types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketMessage {
    pub id: Option<String>,
    pub method: String,
    pub params: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketResponse {
    pub id: Option<String>,
    pub result: Option<Value>,
    pub error: Option<String>,
    pub event_type: Option<String>,
}

impl ApiServer {
    // ========================================================================
    // REST API ENDPOINTS
    // ========================================================================

    /// Handle REST API requests
    pub async fn handle_rest_request(
        &self,
        method: &str,
        path: &str,
        body: Option<Value>,
        headers: &HashMap<String, String>,
    ) -> Result<Value> {
        // Check authentication if enabled
        if self.config.enable_auth {
            let auth_header = headers.get("authorization")
                .or_else(|| headers.get("x-api-key"))
                .ok_or_else(|| BlockchainError::ApiError("Missing API key".to_string()))?;
            
            let api_key = auth_header.strip_prefix("Bearer ")
                .unwrap_or(auth_header);
            
            self.authenticate_request(api_key).await?;
        }

        match (method, path) {
            // Wallet endpoints
            ("POST", "/api/v1/wallets") => self.rest_create_wallet(body).await,
            ("GET", "/api/v1/wallets") => self.rest_list_wallets().await,
            ("GET", path) if path.starts_with("/api/v1/wallets/") => {
                let wallet_id = path.strip_prefix("/api/v1/wallets/").unwrap();
                self.rest_get_wallet(wallet_id).await
            }
            ("POST", path) if path.starts_with("/api/v1/wallets/") && path.ends_with("/addresses") => {
                let wallet_id = path.strip_prefix("/api/v1/wallets/")
                    .unwrap().strip_suffix("/addresses").unwrap();
                self.rest_generate_address(wallet_id, body).await
            }
            ("POST", path) if path.starts_with("/api/v1/wallets/") && path.ends_with("/send") => {
                let wallet_id = path.strip_prefix("/api/v1/wallets/")
                    .unwrap().strip_suffix("/send").unwrap();
                self.rest_send_transaction(wallet_id, body).await
            }

            // Blockchain endpoints
            ("GET", "/api/v1/blockchain/info") => self.rest_get_blockchain_info().await,
            ("GET", "/api/v1/blockchain/blocks/latest") => self.rest_get_latest_block().await,
            ("GET", path) if path.starts_with("/api/v1/blockchain/blocks/") => {
                let block_id = path.strip_prefix("/api/v1/blockchain/blocks/").unwrap();
                self.rest_get_block(block_id).await
            }
            ("GET", path) if path.starts_with("/api/v1/blockchain/transactions/") => {
                let tx_id = path.strip_prefix("/api/v1/blockchain/transactions/").unwrap();
                self.rest_get_transaction(tx_id).await
            }

            // Mempool endpoints
            ("GET", "/api/v1/mempool/info") => self.rest_get_mempool_info().await,
            ("GET", "/api/v1/mempool/transactions") => self.rest_get_mempool_transactions().await,
            ("POST", "/api/v1/mempool/transactions") => self.rest_submit_transaction(body).await,

            // Network endpoints
            ("GET", "/api/v1/network/info") => self.rest_get_network_info().await,
            ("GET", "/api/v1/network/peers") => self.rest_get_peers().await,
            ("POST", "/api/v1/network/peers") => self.rest_add_peer(body).await,

            // Status and metrics
            ("GET", "/api/v1/status") => self.get_server_status().await,
            ("GET", "/api/v1/metrics") => Ok(json!(self.get_metrics().await)),

            _ => Err(BlockchainError::ApiError(format!("Unknown endpoint: {} {}", method, path))),
        }
    }

    // Wallet REST endpoints
    async fn rest_create_wallet(&self, body: Option<Value>) -> Result<Value> {
        let req: CreateWalletRequest = serde_json::from_value(
            body.ok_or_else(|| BlockchainError::ApiError("Missing request body".to_string()))?
        ).map_err(|e| BlockchainError::ApiError(format!("Invalid request: {}", e)))?;

        let params = json!({
            "name": req.name,
            "passphrase": req.passphrase,
            "entropy": req.entropy
        });

        let result = self.create_wallet(Some(params)).await?;
        Ok(json!(ApiResponse::success(result)))
    }

    async fn rest_list_wallets(&self) -> Result<Value> {
        let result = self.list_wallets().await?;
        Ok(json!(ApiResponse::success(result)))
    }

    async fn rest_get_wallet(&self, wallet_id: &str) -> Result<Value> {
        let _wallet_uuid = Uuid::parse_str(wallet_id)
            .map_err(|_| BlockchainError::ApiError("Invalid wallet ID format".to_string()))?;

        // Mock wallet data
        let wallet_data = json!({
            "wallet_id": wallet_id,
            "name": "Sample Wallet",
            "balance": 150000,
            "pending_balance": 5000,
            "account_count": 2,
            "created_at": chrono::Utc::now().to_rfc3339()
        });

        Ok(json!(ApiResponse::success(wallet_data)))
    }

    async fn rest_generate_address(&self, wallet_id: &str, body: Option<Value>) -> Result<Value> {
        let req: GenerateAddressRequest = if let Some(body) = body {
            serde_json::from_value(body)
                .map_err(|e| BlockchainError::ApiError(format!("Invalid request: {}", e)))?
        } else {
            GenerateAddressRequest {
                wallet_id: wallet_id.to_string(),
                account_index: None,
                is_change: None,
            }
        };

        let params = json!({
            "wallet_id": req.wallet_id,
            "account_index": req.account_index
        });

        let result = self.get_new_address(Some(params)).await?;
        Ok(json!(ApiResponse::success(result)))
    }

    async fn rest_send_transaction(&self, wallet_id: &str, body: Option<Value>) -> Result<Value> {
        let req: SendTransactionRequest = serde_json::from_value(
            body.ok_or_else(|| BlockchainError::ApiError("Missing request body".to_string()))?
        ).map_err(|e| BlockchainError::ApiError(format!("Invalid request: {}", e)))?;

        let params = json!({
            "wallet_id": wallet_id,
            "address": req.to_address,
            "amount": req.amount,
            "fee_rate": req.fee_rate
        });

        let result = self.send_to_address(Some(params)).await?;
        Ok(json!(ApiResponse::success(result)))
    }

    // Blockchain REST endpoints
    async fn rest_get_blockchain_info(&self) -> Result<Value> {
        let block_count = self.get_block_count().await?;
        let best_hash = self.get_best_block_hash().await?;
        
        let info = json!({
            "chain": "main",
            "blocks": block_count,
            "best_block_hash": best_hash,
            "difficulty": 1.0,
            "median_time": chrono::Utc::now().timestamp(),
            "verification_progress": 1.0,
            "chain_work": "0000000000000000000000000000000000000000000000000000000000000000",
            "size_on_disk": 1000000,
            "pruned": false
        });

        Ok(json!(ApiResponse::success(info)))
    }

    async fn rest_get_latest_block(&self) -> Result<Value> {
        let best_hash = self.get_best_block_hash().await?;
        let result = self.get_block(Some(best_hash)).await?;
        Ok(json!(ApiResponse::success(result)))
    }

    async fn rest_get_block(&self, block_id: &str) -> Result<Value> {
        let result = self.get_block(Some(json!(block_id))).await?;
        Ok(json!(ApiResponse::success(result)))
    }

    async fn rest_get_transaction(&self, tx_id: &str) -> Result<Value> {
        let result = self.get_transaction(Some(json!(tx_id))).await?;
        Ok(json!(ApiResponse::success(result)))
    }

    // Mempool REST endpoints
    async fn rest_get_mempool_info(&self) -> Result<Value> {
        let result = self.get_mempool_info().await?;
        Ok(json!(ApiResponse::success(result)))
    }

    async fn rest_get_mempool_transactions(&self) -> Result<Value> {
        let result = self.get_raw_mempool().await?;
        Ok(json!(ApiResponse::success(result)))
    }

    async fn rest_submit_transaction(&self, body: Option<Value>) -> Result<Value> {
        let raw_tx = body
            .and_then(|b| b.get("raw_transaction").cloned())
            .ok_or_else(|| BlockchainError::ApiError("Missing raw_transaction field".to_string()))?;

        let result = self.send_raw_transaction(Some(raw_tx)).await?;
        Ok(json!(ApiResponse::success(result)))
    }

    // Network REST endpoints
    async fn rest_get_network_info(&self) -> Result<Value> {
        let result = self.get_network_info().await?;
        Ok(json!(ApiResponse::success(result)))
    }

    async fn rest_get_peers(&self) -> Result<Value> {
        let result = self.get_peer_info().await?;
        Ok(json!(ApiResponse::success(result)))
    }

    async fn rest_add_peer(&self, body: Option<Value>) -> Result<Value> {
        let address = body
            .and_then(|b| b.get("address").cloned())
            .ok_or_else(|| BlockchainError::ApiError("Missing address field".to_string()))?;

        let params = json!({ "address": address, "action": "add" });
        let result = self.add_node(Some(params)).await?;
        Ok(json!(ApiResponse::success(result)))
    }

    // ========================================================================
    // WEBSOCKET HANDLERS
    // ========================================================================

    /// Handle WebSocket message
    pub async fn handle_websocket_message(
        &self,
        connection_id: &Uuid,
        message: WebSocketMessage,
    ) -> Result<WebSocketResponse> {
        match message.method.as_str() {
            "subscribe" => {
                self.handle_subscription(connection_id, message.params).await?;
                Ok(WebSocketResponse {
                    id: message.id,
                    result: Some(json!({ "status": "subscribed" })),
                    error: None,
                    event_type: None,
                })
            }
            "unsubscribe" => {
                self.handle_unsubscription(connection_id, message.params).await?;
                Ok(WebSocketResponse {
                    id: message.id,
                    result: Some(json!({ "status": "unsubscribed" })),
                    error: None,
                    event_type: None,
                })
            }
            "ping" => {
                Ok(WebSocketResponse {
                    id: message.id,
                    result: Some(json!({ "pong": chrono::Utc::now().timestamp() })),
                    error: None,
                    event_type: None,
                })
            }
            _ => {
                // Forward to JSON-RPC handler
                let rpc_request = JsonRpcRequest {
                    jsonrpc: "2.0".to_string(),
                    method: message.method,
                    params: message.params,
                    id: message.id.map(|s| json!(s)),
                };

                let rpc_response = self.handle_jsonrpc_request(rpc_request).await;
                
                Ok(WebSocketResponse {
                    id: rpc_response.id.and_then(|v| v.as_str().map(|s| s.to_string())),
                    result: rpc_response.result,
                    error: rpc_response.error.map(|e| e.message),
                    event_type: None,
                })
            }
        }
    }

    async fn handle_subscription(
        &self,
        connection_id: &Uuid,
        params: Option<Value>,
    ) -> Result<()> {
        let params = params.ok_or_else(|| 
            BlockchainError::ApiError("Missing subscription parameters".to_string()))?;
        
        let sub_type = params.get("type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| BlockchainError::ApiError("Missing subscription type".to_string()))?;

        let subscription = match sub_type {
            "new_blocks" => SubscriptionType::NewBlocks,
            "new_transactions" => SubscriptionType::NewTransactions,
            "mempool_updates" => SubscriptionType::MempoolUpdates,
            "network_updates" => SubscriptionType::NetworkUpdates,
            "wallet_updates" => {
                let wallet_id = params.get("wallet_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| BlockchainError::ApiError("Missing wallet_id for wallet updates".to_string()))?;
                let wallet_uuid = Uuid::parse_str(wallet_id)
                    .map_err(|_| BlockchainError::ApiError("Invalid wallet ID format".to_string()))?;
                SubscriptionType::WalletUpdates { wallet_id: wallet_uuid }
            }
            _ => return Err(BlockchainError::ApiError(format!("Unknown subscription type: {}", sub_type))),
        };

        self.subscribe_to_events(*connection_id, json!(subscription)).await?;
        Ok(())
    }

    async fn handle_unsubscription(
        &self,
        connection_id: &Uuid,
        _params: Option<Value>,
    ) -> Result<()> {
        // For simplicity, remove all subscriptions for this connection
        // In production, would handle specific subscription removal
        let mut connections = self.websocket_connections.write().await;
        if let Some(connection) = connections.get_mut(connection_id) {
            let sub_count = connection.subscriptions.len() as u32;
            connection.subscriptions.clear();
            
            let mut metrics = self.metrics.write().await;
            metrics.active_subscriptions -= sub_count;
        }
        Ok(())
    }

    /// Broadcast block event
    pub async fn broadcast_new_block(&self, block: &crate::block::Block) -> Result<()> {
        let event_data = json!({
            "type": "new_block",
            "data": {
                "hash": hex::encode(block.get_hash()),
                "height": 12345, // Demo height
                "timestamp": block.header.timestamp,
                "transaction_count": block.transactions.len(),
                "size": block.transactions.len() * 250 // Approximate
            }
        });

            self.broadcast_event(&json!("new_blocks"), event_data).await?;
        Ok(())
    }

    /// Broadcast transaction event
    pub async fn broadcast_new_transaction(&self, tx: &crate::transaction::Transaction) -> Result<()> {
        let event_data = json!({
            "type": "new_transaction", 
            "data": {
                "txid": hex::encode(tx.get_hash().unwrap_or([0u8; 32])),
                "size": 250, // Approximate
                "fee": 1000, // Would calculate actual fee
                "inputs": tx.inputs.len(),
                "outputs": tx.outputs.len()
            }
        });

            self.broadcast_event(&json!("new_transactions"), event_data).await?;
        Ok(())
    }

    /// Broadcast mempool event
    pub async fn broadcast_mempool_update(&self, update_type: &str, data: Value) -> Result<()> {
        let event_data = json!({
            "type": "mempool_update",
            "update_type": update_type,
            "data": data
        });

            self.broadcast_event(&json!("mempool_updates"), event_data).await?;
        Ok(())
    }

    /// Broadcast wallet event
    pub async fn broadcast_wallet_update(&self, wallet_id: Uuid, update_type: &str, data: Value) -> Result<()> {
        let event_data = json!({
            "type": "wallet_update",
            "wallet_id": wallet_id,
            "update_type": update_type,
            "data": data
        });

            self.broadcast_event(&json!("wallet_updates"), event_data).await?;
        Ok(())
    }

    /// Broadcast network event
    pub async fn broadcast_network_update(&self, update_type: &str, data: Value) -> Result<()> {
        let event_data = json!({
            "type": "network_update",
            "update_type": update_type,
            "data": data
        });

            self.broadcast_event(&json!("network_updates"), event_data).await?;
        Ok(())
    }
}

// ========================================================================
// HTTP SERVER CONFIGURATION
// ========================================================================

/// HTTP server configuration for production deployment
#[derive(Debug, Clone)]
pub struct HttpServerConfig {
    pub max_request_size: usize,
    pub max_concurrent_requests: usize,
    pub keep_alive_timeout: std::time::Duration,
    pub read_timeout: std::time::Duration,
    pub write_timeout: std::time::Duration,
    pub enable_compression: bool,
    pub enable_logging: bool,
    pub log_level: String,
}

impl Default for HttpServerConfig {
    fn default() -> Self {
        Self {
            max_request_size: 1024 * 1024, // 1MB
            max_concurrent_requests: 1000,
            keep_alive_timeout: std::time::Duration::from_secs(60),
            read_timeout: std::time::Duration::from_secs(30),
            write_timeout: std::time::Duration::from_secs(30),
            enable_compression: true,
            enable_logging: true,
            log_level: "info".to_string(),
        }
    }
}