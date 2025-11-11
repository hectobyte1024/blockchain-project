    // ========================================================================
    // BLOCKCHAIN RPC METHODS
    // ========================================================================

    /// Get current block count
    async fn get_block_count(&self) -> Result<Value> {
        let chain_state = self.consensus.get_chain_state().await;
        Ok(json!(chain_state.height))
    }

    /// Get best block hash
    async fn get_best_block_hash(&self) -> Result<Value> {
        let chain_state = self.consensus.get_chain_state().await;
        Ok(json!(hex::encode(chain_state.best_block_hash)))
    }

    /// Get block by hash or height
    async fn get_block(&self, params: Option<Value>) -> Result<Value> {
        let params = params.ok_or_else(|| 
            BlockchainError::ApiError("Missing block hash or height parameter".to_string()))?;
        
        // Handle both hash (string) and height (number)
        let block = if let Some(hash_str) = params.as_str() {
            // Get by hash
            let hash = hex::decode(hash_str)
                .map_err(|_| BlockchainError::ApiError("Invalid block hash format".to_string()))?;
            if hash.len() != 32 {
                return Err(BlockchainError::ApiError("Block hash must be 32 bytes".to_string()));
            }
            let mut hash_array = [0u8; 32];
            hash_array.copy_from_slice(&hash);
            self.storage.get_block(&hash_array).await?
        } else if let Some(height) = params.as_u64() {
            // Get by height - would need to implement height-to-hash lookup
            return Err(BlockchainError::ApiError("Get block by height not implemented yet".to_string()));
        } else {
            return Err(BlockchainError::ApiError("Invalid block parameter format".to_string()));
        };

        if let Some(block) = block {
            Ok(json!({
                "hash": hex::encode(block.hash()),
                "height": block.header.height,
                "timestamp": block.header.timestamp,
                "previous_hash": hex::encode(block.header.previous_hash),
                "merkle_root": hex::encode(block.header.merkle_root),
                "transactions": block.transactions.iter().map(|tx| hex::encode(tx.hash())).collect::<Vec<_>>(),
                "transaction_count": block.transactions.len(),
                "size": block.transactions.len() * 250 // Approximate size
            }))
        } else {
            Err(BlockchainError::ApiError("Block not found".to_string()))
        }
    }

    /// Get transaction by hash
    async fn get_transaction(&self, params: Option<Value>) -> Result<Value> {
        let params = params.ok_or_else(|| 
            BlockchainError::ApiError("Missing transaction hash parameter".to_string()))?;
        
        let hash_str = params.as_str()
            .ok_or_else(|| BlockchainError::ApiError("Transaction hash must be a string".to_string()))?;
        
        let hash = hex::decode(hash_str)
            .map_err(|_| BlockchainError::ApiError("Invalid transaction hash format".to_string()))?;
        if hash.len() != 32 {
            return Err(BlockchainError::ApiError("Transaction hash must be 32 bytes".to_string()));
        }
        let mut hash_array = [0u8; 32];
        hash_array.copy_from_slice(&hash);

        if let Some(tx) = self.storage.get_transaction(&hash_array).await? {
            Ok(json!({
                "txid": hex::encode(tx.hash()),
                "version": tx.version,
                "locktime": tx.locktime,
                "inputs": tx.inputs.iter().map(|input| json!({
                    "previous_output": {
                        "txid": hex::encode(input.previous_output.txid),
                        "vout": input.previous_output.vout
                    },
                    "script_sig": hex::encode(&input.script_sig),
                    "sequence": input.sequence
                })).collect::<Vec<_>>(),
                "outputs": tx.outputs.iter().enumerate().map(|(i, output)| json!({
                    "n": i,
                    "value": output.value,
                    "script_pubkey": hex::encode(&output.script_pubkey)
                })).collect::<Vec<_>>(),
                "size": 250, // Approximate size
                "fee": 1000, // Would calculate actual fee
                "confirmations": 6 // Would get from blockchain
            }))
        } else {
            Err(BlockchainError::ApiError("Transaction not found".to_string()))
        }
    }

    /// Get balance for address or wallet
    async fn get_balance(&self, params: Option<Value>) -> Result<Value> {
        let params = params.unwrap_or(json!({}));
        
        if let Some(address) = params.get("address").and_then(|v| v.as_str()) {
            // Get balance for specific address
            let utxos = self.storage.get_utxos_by_address(address).await?;
            let balance: u64 = utxos.iter().map(|utxo| utxo.output.value).sum();
            
            Ok(json!({
                "address": address,
                "balance": balance,
                "utxo_count": utxos.len()
            }))
        } else if let Some(wallet_id) = params.get("wallet_id").and_then(|v| v.as_str()) {
            // Get balance for wallet
            let wallet_uuid = Uuid::parse_str(wallet_id)
                .map_err(|_| BlockchainError::ApiError("Invalid wallet ID format".to_string()))?;
            
            let wallet_manager = self.wallet_manager.lock().await;
            // Would implement wallet balance calculation
            Ok(json!({
                "wallet_id": wallet_id,
                "balance": 100000, // Placeholder
                "pending_balance": 5000
            }))
        } else {
            Err(BlockchainError::ApiError("Must specify either 'address' or 'wallet_id'".to_string()))
        }
    }

    // ========================================================================
    // WALLET RPC METHODS
    // ========================================================================

    /// Create a new wallet
    async fn create_wallet(&self, params: Option<Value>) -> Result<Value> {
        let params = params.ok_or_else(|| 
            BlockchainError::ApiError("Missing wallet name parameter".to_string()))?;
        
        let name = params.get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| BlockchainError::ApiError("Wallet name is required".to_string()))?;
        
        let passphrase = params.get("passphrase").and_then(|v| v.as_str());
        
        let mut wallet_manager = self.wallet_manager.lock().await;
        let entropy = if let Some(entropy_hex) = params.get("entropy").and_then(|v| v.as_str()) {
            let entropy_bytes = hex::decode(entropy_hex)
                .map_err(|_| BlockchainError::ApiError("Invalid entropy format".to_string()))?;
            if entropy_bytes.len() != 32 {
                return Err(BlockchainError::ApiError("Entropy must be 32 bytes".to_string()));
            }
            let mut entropy_array = [0u8; 32];
            entropy_array.copy_from_slice(&entropy_bytes);
            Some(entropy_array)
        } else {
            None
        };

        let wallet_id = wallet_manager.create_hd_wallet(name.to_string(), entropy)?;
        
        Ok(json!({
            "wallet_id": wallet_id,
            "name": name,
            "created_at": Utc::now().to_rfc3339(),
            "has_passphrase": passphrase.is_some()
        }))
    }

    /// List all wallets
    async fn list_wallets(&self) -> Result<Value> {
        let wallet_manager = self.wallet_manager.lock().await;
        
        // Would implement wallet listing
        Ok(json!([
            {
                "wallet_id": Uuid::new_v4(),
                "name": "Main Wallet",
                "created_at": Utc::now().to_rfc3339(),
                "balance": 150000,
                "account_count": 2
            },
            {
                "wallet_id": Uuid::new_v4(),
                "name": "Trading Wallet", 
                "created_at": Utc::now().to_rfc3339(),
                "balance": 75000,
                "account_count": 1
            }
        ]))
    }

    /// Generate new address
    async fn get_new_address(&self, params: Option<Value>) -> Result<Value> {
        let params = params.ok_or_else(|| 
            BlockchainError::ApiError("Missing wallet_id parameter".to_string()))?;
        
        let wallet_id = params.get("wallet_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| BlockchainError::ApiError("wallet_id is required".to_string()))?;
        
        let account_index = params.get("account_index")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32);
        
        let wallet_uuid = Uuid::parse_str(wallet_id)
            .map_err(|_| BlockchainError::ApiError("Invalid wallet ID format".to_string()))?;
        
        let mut wallet_manager = self.wallet_manager.lock().await;
        let address = wallet_manager.generate_address(wallet_uuid, account_index)?;
        
        Ok(json!({
            "address": address,
            "wallet_id": wallet_id,
            "account_index": account_index,
            "path": format!("m/44'/0'/{}'/{}/{}", 
                account_index.unwrap_or(0), 0, 0), // Simplified path
            "created_at": Utc::now().to_rfc3339()
        }))
    }

    /// Send to address
    async fn send_to_address(&self, params: Option<Value>) -> Result<Value> {
        let params = params.ok_or_else(|| 
            BlockchainError::ApiError("Missing send parameters".to_string()))?;
        
        let to_address = params.get("address")
            .and_then(|v| v.as_str())
            .ok_or_else(|| BlockchainError::ApiError("Destination address is required".to_string()))?;
        
        let amount = params.get("amount")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| BlockchainError::ApiError("Amount is required".to_string()))?;
        
        let wallet_id = params.get("wallet_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| BlockchainError::ApiError("Wallet ID is required".to_string()))?;
        
        // For demo purposes, create a mock transaction
        let tx_hash = format!("{:x}", md5::compute(format!("{}{}{}", wallet_id, to_address, amount)));
        
        Ok(json!({
            "txid": tx_hash,
            "amount": amount,
            "to_address": to_address,
            "fee": 1000,
            "confirmations": 0,
            "status": "pending",
            "created_at": Utc::now().to_rfc3339()
        }))
    }

    // ========================================================================
    // MEMPOOL RPC METHODS
    // ========================================================================

    /// Get mempool information
    async fn get_mempool_info(&self) -> Result<Value> {
        let mempool = self.mempool.read().await;
        let stats = mempool.get_stats();
        
        Ok(json!({
            "size": stats.transaction_count,
            "bytes": stats.total_size,
            "usage": stats.memory_usage,
            "max_mempool": stats.max_size,
            "mempoolminfee": 1000, // 1000 satoshis per byte
            "minrelaytxfee": 100,  // 100 satoshis per byte
            "unbroadcastcount": 0
        }))
    }

    /// Get raw mempool
    async fn get_raw_mempool(&self) -> Result<Value> {
        let mempool = self.mempool.read().await;
        let transactions = mempool.get_all_transactions();
        
        let txids: Vec<String> = transactions
            .iter()
            .map(|tx| hex::encode(tx.hash()))
            .collect();
        
        Ok(json!(txids))
    }

    /// Send raw transaction
    async fn send_raw_transaction(&self, params: Option<Value>) -> Result<Value> {
        let params = params.ok_or_else(|| 
            BlockchainError::ApiError("Missing raw transaction parameter".to_string()))?;
        
        let raw_tx = params.as_str()
            .ok_or_else(|| BlockchainError::ApiError("Raw transaction must be hex string".to_string()))?;
        
        // Decode and validate transaction
        let tx_bytes = hex::decode(raw_tx)
            .map_err(|_| BlockchainError::ApiError("Invalid hex format".to_string()))?;
        
        // For demo purposes, generate a mock transaction hash
        let tx_hash = format!("{:x}", md5::compute(&tx_bytes));
        
        // In production, would deserialize and add to mempool
        let mut mempool = self.mempool.write().await;
        // mempool.add_transaction(transaction).await?;
        
        Ok(json!({
            "txid": tx_hash,
            "size": tx_bytes.len(),
            "status": "accepted",
            "added_at": Utc::now().to_rfc3339()
        }))
    }

    // ========================================================================
    // NETWORK RPC METHODS
    // ========================================================================

    /// Get network information
    async fn get_network_info(&self) -> Result<Value> {
        let network_stats = self.network.get_stats().await;
        
        Ok(json!({
            "version": "1.0.0",
            "protocol_version": 70016,
            "connections": network_stats.connected_peers,
            "networks": [
                {
                    "name": "ipv4",
                    "limited": false,
                    "reachable": true,
                    "proxy": "",
                    "proxy_randomize_credentials": false
                }
            ],
            "relay_fee": 1000,
            "incremental_fee": 100,
            "local_addresses": [],
            "warnings": ""
        }))
    }

    /// Get peer information
    async fn get_peer_info(&self) -> Result<Value> {
        let peers = self.network.get_connected_peers().await;
        
        let peer_info: Vec<Value> = peers.iter().enumerate().map(|(i, peer)| json!({
            "id": i,
            "addr": format!("{}:{}", peer.address, peer.port),
            "services": "0000000000000409",
            "lastsend": Utc::now().timestamp(),
            "lastrecv": Utc::now().timestamp(),
            "bytessent": peer.bytes_sent,
            "bytesrecv": peer.bytes_received,
            "conntime": peer.connected_since.timestamp(),
            "timeoffset": 0,
            "pingtime": 0.05,
            "version": peer.protocol_version,
            "subver": "/EduBlockchain:1.0.0/",
            "inbound": peer.direction == crate::network::ConnectionDirection::Inbound,
            "addnode": false,
            "startingheight": peer.latest_block_height,
            "banscore": 0,
            "synced_headers": peer.latest_block_height,
            "synced_blocks": peer.latest_block_height,
            "whitelisted": false
        })).collect();
        
        Ok(json!(peer_info))
    }

    /// Add node
    async fn add_node(&self, params: Option<Value>) -> Result<Value> {
        let params = params.ok_or_else(|| 
            BlockchainError::ApiError("Missing node address parameter".to_string()))?;
        
        let node_address = params.get("address")
            .and_then(|v| v.as_str())
            .ok_or_else(|| BlockchainError::ApiError("Node address is required".to_string()))?;
        
        let action = params.get("action")
            .and_then(|v| v.as_str())
            .unwrap_or("add");
        
        match action {
            "add" => {
                self.network.connect_to_peer(node_address).await?;
                Ok(json!({
                    "result": "success",
                    "action": "add",
                    "address": node_address,
                    "message": "Node added successfully"
                }))
            }
            "remove" => {
                self.network.disconnect_from_peer(node_address).await?;
                Ok(json!({
                    "result": "success", 
                    "action": "remove",
                    "address": node_address,
                    "message": "Node removed successfully"
                }))
            }
            _ => Err(BlockchainError::ApiError("Invalid action. Use 'add' or 'remove'".to_string()))
        }
    }

    // ========================================================================
    // MINING RPC METHODS  
    // ========================================================================

    /// Get mining information
    async fn get_mining_info(&self) -> Result<Value> {
        let chain_state = self.consensus.get_chain_state().await;
        
        Ok(json!({
            "blocks": chain_state.height,
            "difficulty": chain_state.difficulty_target,
            "networkhashps": 1000000000.0, // 1 GH/s estimated
            "pooledtx": 0,
            "chain": "main",
            "warnings": ""
        }))
    }

    // ========================================================================
    // WEBSOCKET MANAGEMENT
    // ========================================================================

    /// Add WebSocket connection
    pub async fn add_websocket_connection(
        &self, 
        connection_id: Uuid,
        authenticated: bool,
        api_key: Option<String>
    ) -> Result<()> {
        let connection = WebSocketConnection {
            id: connection_id,
            subscriptions: Vec::new(),
            authenticated,
            api_key,
            connected_at: Utc::now(),
        };

        let mut connections = self.websocket_connections.write().await;
        connections.insert(connection_id, connection);
        
        let mut metrics = self.metrics.write().await;
        metrics.websocket_connections += 1;

        println!("📺 WebSocket connection established: {}", connection_id);
        Ok(())
    }

    /// Remove WebSocket connection
    pub async fn remove_websocket_connection(&self, connection_id: &Uuid) -> Result<()> {
        let mut connections = self.websocket_connections.write().await;
        if let Some(connection) = connections.remove(connection_id) {
            let mut metrics = self.metrics.write().await;
            metrics.websocket_connections -= 1;
            metrics.active_subscriptions -= connection.subscriptions.len() as u32;
            
            println!("📺 WebSocket connection closed: {}", connection_id);
        }
        Ok(())
    }

    /// Subscribe to events
    pub async fn subscribe_to_events(
        &self,
        connection_id: &Uuid,
        subscription_type: SubscriptionType
    ) -> Result<()> {
        let mut connections = self.websocket_connections.write().await;
        if let Some(connection) = connections.get_mut(connection_id) {
            if !connection.subscriptions.contains(&subscription_type) {
                connection.subscriptions.push(subscription_type.clone());
                
                let mut metrics = self.metrics.write().await;
                metrics.active_subscriptions += 1;
                
                println!("📺 WebSocket subscription added: {:?} for {}", subscription_type, connection_id);
            }
        }
        Ok(())
    }

    /// Broadcast event to subscribers
    pub async fn broadcast_event(&self, event_type: &SubscriptionType, data: Value) -> Result<()> {
        let connections = self.websocket_connections.read().await;
        let mut subscribers = Vec::new();
        
        for (id, connection) in connections.iter() {
            if connection.subscriptions.contains(event_type) {
                subscribers.push(*id);
            }
        }
        
        // In production, would send WebSocket messages to subscribers
        if !subscribers.is_empty() {
            println!("📡 Broadcasting {:?} to {} subscribers", event_type, subscribers.len());
        }
        
        Ok(())
    }

    // ========================================================================
    // METRICS & MONITORING
    // ========================================================================

    /// Get API metrics
    pub async fn get_metrics(&self) -> ApiMetrics {
        self.metrics.read().await.clone()
    }

    /// Reset metrics
    pub async fn reset_metrics(&self) -> Result<()> {
        let mut metrics = self.metrics.write().await;
        *metrics = ApiMetrics {
            last_reset: Utc::now(),
            ..Default::default()
        };
        Ok(())
    }

    /// Get server status
    pub async fn get_server_status(&self) -> Result<Value> {
        let metrics = self.get_metrics().await;
        let is_running = *self.is_running.read().await;
        
        Ok(json!({
            "status": if is_running { "running" } else { "stopped" },
            "uptime_seconds": Utc::now().signed_duration_since(metrics.last_reset).num_seconds(),
            "total_requests": metrics.total_requests,
            "successful_requests": metrics.successful_requests,
            "failed_requests": metrics.failed_requests,
            "success_rate": if metrics.total_requests > 0 {
                metrics.successful_requests as f64 / metrics.total_requests as f64 * 100.0
            } else { 0.0 },
            "average_response_time_ms": metrics.average_response_time,
            "websocket_connections": metrics.websocket_connections,
            "active_subscriptions": metrics.active_subscriptions,
            "requests_by_method": metrics.requests_by_method,
            "errors_by_code": metrics.errors_by_code,
            "memory_usage": std::mem::size_of_val(&metrics), // Simplified
            "config": {
                "bind_address": self.config.bind_address,
                "port": self.config.port,
                "max_connections": self.config.max_connections,
                "request_timeout_seconds": self.config.request_timeout.as_secs(),
                "enable_auth": self.config.enable_auth,
                "enable_metrics": self.config.enable_metrics,
            }
        }))
    }
}

// ========================================================================
// ERROR HANDLING & UTILITIES
// ========================================================================

impl std::fmt::Display for JsonRpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "JSON-RPC Error {}: {}", self.code, self.message)
    }
}

impl std::error::Error for JsonRpcError {}

// Helper function to validate API permissions
pub fn has_permission(permissions: &[ApiPermission], required: ApiPermission) -> bool {
    permissions.contains(&required) || permissions.contains(&ApiPermission::Admin)
}

// Helper function to format blockchain amounts
pub fn format_amount(satoshis: u64) -> f64 {
    satoshis as f64 / 100_000_000.0 // Convert satoshis to coins
}

// Helper function to parse amount from coins to satoshis
pub fn parse_amount(coins: f64) -> Result<u64> {
    if coins < 0.0 {
        return Err(BlockchainError::ApiError("Amount cannot be negative".to_string()));
    }
    Ok((coins * 100_000_000.0) as u64)
}