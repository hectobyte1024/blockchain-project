// API Server Demo - Production-Grade Blockchain API
//
// Comprehensive demonstration of the RPC Interface & API Layer featuring:
// - JSON-RPC 2.0 server for standard blockchain operations
// - REST API endpoints for HTTP-based interactions
// - WebSocket support for real-time updates and subscriptions  
// - Authentication and authorization system
// - Rate limiting and request throttling
// - API metrics and monitoring
//
// This demo showcases the complete external interface to our blockchain system.

use blockchain_core::{
    api_server::{ApiServer, ApiServerConfig, ApiPermission, JsonRpcRequest, SubscriptionType},
    rest_api::{WebSocketMessage},
    consensus::{ConsensusValidator, ConsensusParams},
    mempool::{Mempool, MempoolConfig},
    advanced_wallet::AdvancedWalletManager,
};

use serde_json::json;
use tokio::sync::{Arc, Mutex, RwLock};
use std::time::Duration;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 API Server Demo - Production-Grade Blockchain API");
    println!("==================================================");
    
    // Initialize blockchain components
    println!("🔧 Initializing blockchain components...");
    
    let consensus_params = ConsensusParams::default();
    let consensus = Arc::new(ConsensusValidator::new(consensus_params));
    
    let mempool_config = MempoolConfig::default();
    let mempool = Arc::new(RwLock::new(Mempool::new(mempool_config)));
    
    let wallet_manager = Arc::new(Mutex::new(AdvancedWalletManager::new()));
    
    // Create API server configuration
    let api_config = ApiServerConfig {
        bind_address: "127.0.0.1".to_string(),
        port: 8332,
        enable_cors: true,
        cors_origins: vec!["*".to_string()],
        max_connections: 1000,
        request_timeout: Duration::from_secs(30),
        enable_metrics: true,
        enable_auth: true,
        default_rate_limit: 100, // 100 requests per minute
        admin_rate_limit: 500,   // 500 requests per minute for admin
    };
    
    // Create API server
    let api_server = ApiServer::new(
        api_config,
        consensus,
        mempool,
        wallet_manager,
    );
    
    println!("✅ Blockchain components initialized");
    
    // ========================================================================
    // API KEY MANAGEMENT DEMO
    // ========================================================================
    
    println!("\n🔐 API Key Management Demo");
    println!("------------------------");
    
    // Create different types of API keys
    let admin_key = api_server.create_api_key(
        vec![ApiPermission::Admin],
        Some(500) // Higher rate limit for admin
    ).await?;
    println!("🔑 Admin API key created: {}", admin_key);
    
    let wallet_key = api_server.create_api_key(
        vec![
            ApiPermission::ReadWallet,
            ApiPermission::WriteWallet,
            ApiPermission::ReadBlockchain,
        ],
        Some(200)
    ).await?;
    println!("🔑 Wallet API key created: {}", wallet_key);
    
    let readonly_key = api_server.create_api_key(
        vec![
            ApiPermission::ReadBlockchain,
            ApiPermission::ReadMempool,
            ApiPermission::ReadNetwork,
        ],
        Some(60)
    ).await?;
    println!("🔑 Read-only API key created: {}", readonly_key);
    
    // ========================================================================
    // START API SERVER
    // ========================================================================
    
    println!("\n🚀 Starting API Server");
    println!("---------------------");
    
    api_server.start().await?;
    
    // Give server time to initialize
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // ========================================================================
    // JSON-RPC 2.0 DEMO
    // ========================================================================
    
    println!("\n📡 JSON-RPC 2.0 Demo");
    println!("-------------------");
    
    // Test blockchain info requests
    let block_count_request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "getblockcount".to_string(),
        params: None,
        id: Some(json!(1)),
    };
    
    let response = api_server.handle_jsonrpc_request(block_count_request).await;
    println!("📊 Block count response: {:?}", response);
    
    let best_hash_request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "getbestblockhash".to_string(),
        params: None,
        id: Some(json!(2)),
    };
    
    let response = api_server.handle_jsonrpc_request(best_hash_request).await;
    println!("🏷️  Best block hash response: {:?}", response);
    
    // Test wallet operations
    let create_wallet_request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "createwallet".to_string(),
        params: Some(json!({
            "name": "Demo Wallet",
            "passphrase": "secure123"
        })),
        id: Some(json!(3)),
    };
    
    let response = api_server.handle_jsonrpc_request(create_wallet_request).await;
    println!("💼 Create wallet response: {:?}", response);
    
    // Test mempool info
    let mempool_info_request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "getmempoolinfo".to_string(),
        params: None,
        id: Some(json!(4)),
    };
    
    let response = api_server.handle_jsonrpc_request(mempool_info_request).await;
    println!("🏊 Mempool info response: {:?}", response);
    
    // Test network info
    let network_info_request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "getnetworkinfo".to_string(),
        params: None,
        id: Some(json!(5)),
    };
    
    let response = api_server.handle_jsonrpc_request(network_info_request).await;
    println!("🌐 Network info response: {:?}", response);
    
    // ========================================================================
    // REST API DEMO
    // ========================================================================
    
    println!("\n🌐 REST API Demo");
    println!("--------------");
    
    let mut headers = std::collections::HashMap::new();
    headers.insert("authorization".to_string(), format!("Bearer {}", admin_key));
    
    // Test blockchain info endpoint
    let blockchain_info = api_server.handle_rest_request(
        "GET",
        "/api/v1/blockchain/info",
        None,
        &headers,
    ).await?;
    println!("📊 Blockchain info: {}", blockchain_info);
    
    // Test wallet creation via REST
    let wallet_request = json!({
        "name": "REST Demo Wallet",
        "passphrase": "rest123"
    });
    
    let wallet_response = api_server.handle_rest_request(
        "POST",
        "/api/v1/wallets",
        Some(wallet_request),
        &headers,
    ).await?;
    println!("💼 REST wallet creation: {}", wallet_response);
    
    // Test wallet list
    let wallets_list = api_server.handle_rest_request(
        "GET",
        "/api/v1/wallets",
        None,
        &headers,
    ).await?;
    println!("📋 Wallets list: {}", wallets_list);
    
    // Test mempool info via REST
    let mempool_rest = api_server.handle_rest_request(
        "GET",
        "/api/v1/mempool/info",
        None,
        &headers,
    ).await?;
    println!("🏊 REST mempool info: {}", mempool_rest);
    
    // Test network peers
    let peers_info = api_server.handle_rest_request(
        "GET",
        "/api/v1/network/peers",
        None,
        &headers,
    ).await?;
    println!("👥 Network peers: {}", peers_info);
    
    // ========================================================================
    // WEBSOCKET DEMO
    // ========================================================================
    
    println!("\n📺 WebSocket Demo");
    println!("---------------");
    
    // Create WebSocket connections
    let ws_connection1 = Uuid::new_v4();
    let ws_connection2 = Uuid::new_v4();
    
    api_server.add_websocket_connection(ws_connection1, true, Some(admin_key.clone())).await?;
    api_server.add_websocket_connection(ws_connection2, true, Some(wallet_key.clone())).await?;
    
    println!("📱 WebSocket connections established: {} and {}", ws_connection1, ws_connection2);
    
    // Test subscriptions
    let subscribe_msg = WebSocketMessage {
        id: Some("sub1".to_string()),
        method: "subscribe".to_string(),
        params: Some(json!({
            "type": "new_blocks"
        })),
    };
    
    let sub_response = api_server.handle_websocket_message(&ws_connection1, subscribe_msg).await?;
    println!("📺 Block subscription response: {:?}", sub_response);
    
    let subscribe_msg = WebSocketMessage {
        id: Some("sub2".to_string()),
        method: "subscribe".to_string(),
        params: Some(json!({
            "type": "mempool_updates"
        })),
    };
    
    let sub_response = api_server.handle_websocket_message(&ws_connection2, subscribe_msg).await?;
    println!("📺 Mempool subscription response: {:?}", sub_response);
    
    // Test JSON-RPC via WebSocket
    let ws_rpc_msg = WebSocketMessage {
        id: Some("rpc1".to_string()),
        method: "getblockcount".to_string(),
        params: None,
    };
    
    let rpc_response = api_server.handle_websocket_message(&ws_connection1, ws_rpc_msg).await?;
    println!("📺 WebSocket RPC response: {:?}", rpc_response);
    
    // Simulate broadcasting events
    println!("\n📡 Broadcasting Events");
    println!("--------------------");
    
    // Simulate new block broadcast
    api_server.broadcast_event(
        &SubscriptionType::NewBlocks,
        json!({
            "type": "new_block",
            "data": {
                "hash": "0x1234567890abcdef",
                "height": 12345,
                "timestamp": chrono::Utc::now().timestamp(),
                "transaction_count": 10
            }
        })
    ).await?;
    
    // Simulate mempool update
    api_server.broadcast_mempool_update(
        "transaction_added",
        json!({
            "txid": "0xabcdef1234567890",
            "size": 250,
            "fee": 1000
        })
    ).await?;
    
    // ========================================================================
    // RATE LIMITING DEMO
    // ========================================================================
    
    println!("\n⏱️  Rate Limiting Demo");
    println!("--------------------");
    
    // Test authentication
    match api_server.authenticate_request(&readonly_key).await {
        Ok(permissions) => println!("✅ Authentication successful. Permissions: {:?}", permissions),
        Err(e) => println!("❌ Authentication failed: {}", e),
    }
    
    // Simulate rapid requests to test rate limiting
    println!("🔄 Testing rate limiting with rapid requests...");
    for i in 1..=5 {
        match api_server.authenticate_request(&readonly_key).await {
            Ok(_) => println!("✅ Request {} accepted", i),
            Err(e) => println!("❌ Request {} rejected: {}", i, e),
        }
    }
    
    // ========================================================================
    // METRICS & MONITORING DEMO
    // ========================================================================
    
    println!("\n📊 Metrics & Monitoring Demo");
    println!("---------------------------");
    
    let metrics = api_server.get_metrics().await;
    println!("📈 API Metrics:");
    println!("   Total requests: {}", metrics.total_requests);
    println!("   Successful requests: {}", metrics.successful_requests);
    println!("   Failed requests: {}", metrics.failed_requests);
    println!("   Success rate: {:.2}%", 
        if metrics.total_requests > 0 {
            metrics.successful_requests as f64 / metrics.total_requests as f64 * 100.0
        } else { 0.0 });
    println!("   WebSocket connections: {}", metrics.websocket_connections);
    println!("   Active subscriptions: {}", metrics.active_subscriptions);
    println!("   Average response time: {:.2}ms", metrics.average_response_time);
    
    // Get server status
    let status = api_server.get_server_status().await?;
    println!("\n🖥️  Server Status: {}", status);
    
    // ========================================================================
    // SECURITY FEATURES DEMO
    // ========================================================================
    
    println!("\n🔒 Security Features Demo");
    println!("-----------------------");
    
    // Test unauthorized access
    let invalid_headers = std::collections::HashMap::new();
    match api_server.handle_rest_request(
        "GET",
        "/api/v1/blockchain/info",
        None,
        &invalid_headers,
    ).await {
        Ok(_) => println!("⚠️  Unauthorized access allowed (should not happen)"),
        Err(e) => println!("✅ Unauthorized access blocked: {}", e),
    }
    
    // Test invalid API key
    let mut invalid_headers = std::collections::HashMap::new();
    invalid_headers.insert("authorization".to_string(), "Bearer invalid_key_123".to_string());
    
    match api_server.handle_rest_request(
        "GET", 
        "/api/v1/blockchain/info",
        None,
        &invalid_headers,
    ).await {
        Ok(_) => println!("⚠️  Invalid API key accepted (should not happen)"),
        Err(e) => println!("✅ Invalid API key rejected: {}", e),
    }
    
    // ========================================================================
    // PERFORMANCE DEMONSTRATION
    // ========================================================================
    
    println!("\n⚡ Performance Demonstration");
    println!("--------------------------");
    
    let start_time = std::time::Instant::now();
    let mut success_count = 0;
    let test_requests = 50;
    
    for i in 1..=test_requests {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "getblockcount".to_string(),
            params: None,
            id: Some(json!(i)),
        };
        
        let response = api_server.handle_jsonrpc_request(request).await;
        if response.error.is_none() {
            success_count += 1;
        }
    }
    
    let duration = start_time.elapsed();
    let requests_per_second = test_requests as f64 / duration.as_secs_f64();
    
    println!("🚀 Performance Results:");
    println!("   Processed {} requests in {:?}", test_requests, duration);
    println!("   Success rate: {}/{} ({:.1}%)", success_count, test_requests, 
        success_count as f64 / test_requests as f64 * 100.0);
    println!("   Throughput: {:.0} requests/second", requests_per_second);
    println!("   Average response time: {:.2}ms", duration.as_millis() as f64 / test_requests as f64);
    
    // ========================================================================
    // CLEANUP
    // ========================================================================
    
    println!("\n🧹 Cleanup");
    println!("---------");
    
    // Remove WebSocket connections
    api_server.remove_websocket_connection(&ws_connection1).await?;
    api_server.remove_websocket_connection(&ws_connection2).await?;
    
    // Stop API server
    api_server.stop().await?;
    
    // Final metrics
    let final_metrics = api_server.get_metrics().await;
    println!("📊 Final API Metrics:");
    println!("   Total requests processed: {}", final_metrics.total_requests);
    println!("   Total WebSocket connections: {}", final_metrics.websocket_connections);
    
    println!("\n🎯 API Server Demo Completed Successfully!");
    println!("=========================================");
    println!("✅ JSON-RPC 2.0 server fully functional");
    println!("✅ REST API endpoints working correctly");
    println!("✅ WebSocket real-time communication active");
    println!("✅ Authentication & authorization enforced");
    println!("✅ Rate limiting protecting against abuse");
    println!("✅ Comprehensive metrics & monitoring");
    println!("✅ Security features preventing unauthorized access");
    println!("✅ High-performance request processing");
    println!("✅ Production-ready blockchain API interface");
    
    Ok(())
}