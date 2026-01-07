//! RPC client - for web applications to connect to blockchain nodes

use crate::{RpcRequest, RpcResponse, methods};
use anyhow::{Result, Context};
use serde_json::json;

/// RPC client for connecting to blockchain node
pub struct RpcClient {
    endpoint: String,
    client: reqwest::Client,
    request_id: std::sync::atomic::AtomicU64,
}

impl RpcClient {
    /// Create new RPC client
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            client: reqwest::Client::new(),
            request_id: std::sync::atomic::AtomicU64::new(1),
        }
    }
    
    /// Make RPC call
    async fn call(&self, method: &str, params: serde_json::Value) -> Result<serde_json::Value> {
        let id = self.request_id.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        
        let request = RpcRequest {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params,
            id,
        };
        
        let response: RpcResponse = self.client
            .post(&self.endpoint)
            .json(&request)
            .send()
            .await
            .context("Failed to send RPC request")?
            .json()
            .await
            .context("Failed to parse RPC response")?;
        
        if let Some(error) = response.error {
            anyhow::bail!("RPC error {}: {}", error.code, error.message);
        }
        
        response.result.context("Missing result in RPC response")
    }
    
    /// Get current block height
    pub async fn get_block_height(&self) -> Result<u64> {
        let result = self.call(methods::GET_BLOCK_HEIGHT, json!([])).await?;
        Ok(serde_json::from_value(result)?)
    }
    
    /// Get balance of address
    pub async fn get_balance(&self, address: &str) -> Result<u64> {
        let result = self.call(methods::GET_BALANCE, json!([address])).await?;
        Ok(serde_json::from_value(result)?)
    }
    
    /// Send raw transaction (returns transaction hash)
    pub async fn send_transaction(&self, signed_tx_hex: &str) -> Result<String> {
        let result = self.call(methods::SEND_TRANSACTION, json!([signed_tx_hex])).await?;
        Ok(serde_json::from_value(result)?)
    }
    
    /// Get transaction by hash
    pub async fn get_transaction(&self, tx_hash: &str) -> Result<serde_json::Value> {
        self.call(methods::GET_TRANSACTION, json!([tx_hash])).await
    }
    
    /// Get block by height
    pub async fn get_block(&self, height: u64) -> Result<serde_json::Value> {
        self.call(methods::GET_BLOCK, json!([height])).await
    }
    
    /// Get network info
    pub async fn get_network_info(&self) -> Result<serde_json::Value> {
        self.call(methods::GET_NETWORK_INFO, json!([])).await
    }
    
    /// Get mempool info
    pub async fn get_mempool_info(&self) -> Result<serde_json::Value> {
        self.call(methods::GET_MEMPOOL_INFO, json!([])).await
    }
    
    /// Get mining info (for miners)
    pub async fn get_mining_info(&self) -> Result<serde_json::Value> {
        self.call(methods::GET_MINING_INFO, json!([])).await
    }
    
    /// Get sync status
    pub async fn get_sync_status(&self) -> Result<serde_json::Value> {
        self.call(methods::GET_SYNC_STATUS, json!([])).await
    }
    
    /// Credit balance directly (for vouchers/airdrops)
    pub async fn credit_balance(&self, address: &str, amount: u64) -> Result<serde_json::Value> {
        self.call(methods::CREDIT_BALANCE, json!([address, amount])).await
    }
    
    /// Check if node is reachable
    pub async fn is_connected(&self) -> bool {
        self.get_block_height().await.is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_client_creation() {
        let client = RpcClient::new("http://localhost:8545");
        assert_eq!(client.endpoint, "http://localhost:8545");
    }
}
