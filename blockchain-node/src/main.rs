//! Blockchain Node - Headless blockchain daemon
//! 
//! This binary runs a full blockchain node that:
//! - Validates and mines blocks
//! - Participates in P2P network
//! - Exposes RPC API for clients
//! - Earns mining rewards
//! 
//! This is what node operators run to support the network.

use blockchain_rpc::server::{RpcServer, RpcServerConfig};
use blockchain_network::NetworkConfig;
use anyhow::Result;
use clap::Parser;
use tracing::{info, error};
use jsonrpc_core::{IoHandler, Params, Value};
use serde_json::json;
use std::sync::Arc;
use std::net::SocketAddr;
use std::path::PathBuf;

mod blockchain;
mod miner;
mod treasury;

use blockchain::BlockchainBackend;
use miner::MiningDaemon;
use treasury::TreasuryManager;

/// Blockchain Node CLI
#[derive(Parser)]
#[command(name = "blockchain-node")]
#[command(about = "EduNet Blockchain Node - Full node daemon", long_about = None)]
struct Cli {
    /// RPC server host
    #[arg(long, default_value = "0.0.0.0")]
    rpc_host: String,
    
    /// RPC server port
    #[arg(long, default_value_t = 8545)]
    rpc_port: u16,
    
    /// P2P network port
    #[arg(long, default_value_t = 9000)]
    p2p_port: u16,
    
    /// Data directory for blockchain storage
    #[arg(long, default_value = "./blockchain-data")]
    data_dir: PathBuf,
    
    /// Bootstrap peers (comma-separated host:port)
    #[arg(long)]
    bootstrap_peers: Option<String>,
    
    /// Enable mining
    #[arg(long)]
    mining: bool,
    
    /// Validator address for mining rewards
    #[arg(long)]
    validator_address: Option<String>,
}

/// Create RPC server wired to blockchain backend and treasury
fn create_blockchain_rpc_server(
    config: RpcServerConfig, 
    blockchain: Arc<BlockchainBackend>,
    treasury: Arc<TreasuryManager>
) -> RpcServer {
    let mut handler = IoHandler::new();
    
    // Get block height
    {
        let bc = blockchain.clone();
        handler.add_sync_method("blockchain_getBlockHeight", move |_params: Params| {
            let bc = bc.clone();
            let height = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    bc.get_height().await
                })
            });
            Ok(Value::Number(height.into()))
        });
    }
    
    // Get block by height
    {
        let bc = blockchain.clone();
        handler.add_sync_method("blockchain_getBlock", move |params: Params| {
            let bc = bc.clone();
            let parsed: Vec<u64> = params.parse()?;
            if parsed.is_empty() {
                return Err(jsonrpc_core::Error::invalid_params("Missing block height"));
            }
            
            let block = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    bc.get_block_by_height(parsed[0]).await
                })
            });
            
            match block {
                Some(b) => Ok(json!({
                    "height": b.header.height,
                    "hash": hex::encode(b.header.calculate_hash()),
                    "prev_hash": hex::encode(b.header.prev_block_hash),
                    "timestamp": b.header.timestamp,
                    "transactions_count": b.transactions.len()
                })),
                None => Ok(json!({"error": "Block not found"}))
            }
        });
    }
    
    // Get balance
    {
        let bc = blockchain.clone();
        handler.add_sync_method("wallet_getBalance", move |params: Params| {
            let bc = bc.clone();
            let parsed: Vec<String> = params.parse()?;
            if parsed.is_empty() {
                return Err(jsonrpc_core::Error::invalid_params("Missing address"));
            }
            
            let result = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    bc.get_balance(&parsed[0]).await
                })
            });
            
            match result {
                Ok(balance) => Ok(Value::Number(balance.into())),
                Err(e) => Ok(json!({"error": format!("Failed to get balance: {}", e)}))
            }
        });
    }
    
    // List wallets
    {
        let bc = blockchain.clone();
        handler.add_sync_method("wallet_list", move |_params: Params| {
            let bc = bc.clone();
            let wallets = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    bc.list_wallets().await
                })
            });
            
            let wallet_list: Vec<_> = wallets.into_iter().map(|(name, address, balance)| {
                json!({
                    "name": name,
                    "address": address,
                    "balance": balance
                })
            }).collect();
            Ok(Value::Array(wallet_list))
        });
    }
    
    // Debug: List all addresses with UTXOs
    {
        let bc = blockchain.clone();
        handler.add_sync_method("debug_listAddresses", move |_params: Params| {
            let bc = bc.clone();
            let addresses = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    let utxo_set = bc.utxo_set.read().await;
                    let all_addresses = utxo_set.get_all_addresses();
                    all_addresses.into_iter().map(|addr| {
                        let balance = utxo_set.get_balance(&addr);
                        json!({
                            "address": addr,
                            "balance": balance
                        })
                    }).collect::<Vec<_>>()
                })
            });
            Ok(Value::Array(addresses))
        });
    }
    
    // Get status
    {
        let bc = blockchain.clone();
        handler.add_sync_method("blockchain_getStatus", move |_params: Params| {
            let bc = bc.clone();
            let status = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    bc.get_status().await
                })
            });
            Ok(status)
        });
    }
    
    // Treasury: Get current price
    {
        let tr = treasury.clone();
        handler.add_sync_method("treasury_getPrice", move |_params: Params| {
            let tr = tr.clone();
            let price = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    tr.get_price().await
                })
            });
            Ok(json!({
                "price_cents": price,
                "price_usd": format!("${:.2}", price as f64 / 100.0)
            }))
        });
    }
    
    // Treasury: Set price (admin only in production)
    {
        let tr = treasury.clone();
        handler.add_sync_method("treasury_setPrice", move |params: Params| {
            let tr = tr.clone();
            let parsed: Vec<u64> = params.parse()?;
            if parsed.is_empty() {
                return Err(jsonrpc_core::Error::invalid_params("Missing price_cents"));
            }
            
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    tr.set_price(parsed[0]).await;
                })
            });
            Ok(json!({"success": true}))
        });
    }
    
    // Treasury: Sell coins (after receiving cash payment)
    {
        let tr = treasury.clone();
        handler.add_sync_method("treasury_sellCoins", move |params: Params| {
            let tr = tr.clone();
            let parsed: serde_json::Map<String, Value> = params.parse()?;
            
            let buyer_address = parsed.get("buyer_address")
                .and_then(|v| v.as_str())
                .ok_or_else(|| jsonrpc_core::Error::invalid_params("Missing buyer_address"))?;
            let amount = parsed.get("amount")
                .and_then(|v| v.as_u64())
                .ok_or_else(|| jsonrpc_core::Error::invalid_params("Missing amount"))?;
            let payment_method = parsed.get("payment_method")
                .and_then(|v| v.as_str())
                .unwrap_or("cash");
            let payment_proof = parsed.get("payment_proof")
                .and_then(|v| v.as_str())
                .unwrap_or("no receipt");
            
            let sale = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    tr.sell_coins(
                        buyer_address.to_string(),
                        amount,
                        payment_method.to_string(),
                        payment_proof.to_string(),
                    ).await
                })
            });
            
            match sale {
                Ok(s) => Ok(serde_json::to_value(s).unwrap()),
                Err(e) => {
                    error!("Treasury sale failed: {}", e);
                    Err(jsonrpc_core::Error::internal_error())
                }
            }
        });
    }
    
    // Treasury: Get statistics
    {
        let tr = treasury.clone();
        handler.add_sync_method("treasury_getStats", move |_params: Params| {
            let tr = tr.clone();
            let stats = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    tr.get_stats().await
                })
            });
            Ok(serde_json::to_value(stats).unwrap())
        });
    }
    
    // Treasury: List all sales
    {
        let tr = treasury.clone();
        handler.add_sync_method("treasury_getSales", move |_params: Params| {
            let tr = tr.clone();
            let sales = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    tr.get_sales().await
                })
            });
            Ok(serde_json::to_value(sales).unwrap())
        });
    }
    
    // Debug: Dump UTXO set
    {
        let bc = blockchain.clone();
        handler.add_sync_method("debug_dumpUtxos", move |_params: Params| {
            let bc = bc.clone();
            let result = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    let utxo_set = bc.utxo_set.read().await;
                    let count = utxo_set.get_utxo_count();
                    let addresses = utxo_set.get_all_addresses();
                    
                    json!({
                        "total_utxos": count,
                        "addresses": addresses
                    })
                })
            });
            Ok(result)
        });
    }
    
    // Contract: Deploy
    {
        let bc = blockchain.clone();
        handler.add_sync_method("contract_deploy", move |params: Params| {
            let bc = bc.clone();
            let parsed: serde_json::Map<String, Value> = params.parse()?;
            
            let deployer = parsed.get("deployer")
                .and_then(|v| v.as_str())
                .ok_or_else(|| jsonrpc_core::Error::invalid_params("Missing deployer"))?;
            let bytecode_hex = parsed.get("bytecode")
                .and_then(|v| v.as_str())
                .ok_or_else(|| jsonrpc_core::Error::invalid_params("Missing bytecode"))?;
            let value = parsed.get("value")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let gas_limit = parsed.get("gas_limit")
                .and_then(|v| v.as_u64())
                .unwrap_or(1000000);
                
            let bytecode = hex::decode(bytecode_hex)
                .map_err(|e| jsonrpc_core::Error::invalid_params(format!("Invalid bytecode hex: {}", e)))?;
            
            let result = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    bc.deploy_contract(deployer, bytecode, value, gas_limit).await
                })
            }).map_err(|e| {
                error!("❌ Contract deployment error: {}", e);
                jsonrpc_core::Error {
                    code: jsonrpc_core::ErrorCode::ServerError(-32000),
                    message: format!("Contract deployment failed: {}", e),
                    data: None,
                }
            })?;
            
            Ok(serde_json::to_value(result).unwrap())
        });
    }
    
    // Contract: Call
    {
        let bc = blockchain.clone();
        handler.add_sync_method("contract_call", move |params: Params| {
            let bc = bc.clone();
            let parsed: serde_json::Map<String, Value> = params.parse()?;
            
            let caller = parsed.get("caller")
                .and_then(|v| v.as_str())
                .ok_or_else(|| jsonrpc_core::Error::invalid_params("Missing caller"))?;
            let contract_hex = parsed.get("contract")
                .and_then(|v| v.as_str())
                .ok_or_else(|| jsonrpc_core::Error::invalid_params("Missing contract address"))?;
            let calldata_hex = parsed.get("data")
                .and_then(|v| v.as_str())
                .ok_or_else(|| jsonrpc_core::Error::invalid_params("Missing calldata"))?;
            let value = parsed.get("value")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let gas_limit = parsed.get("gas_limit")
                .and_then(|v| v.as_u64())
                .unwrap_or(100000);
                
            let contract_bytes = hex::decode(contract_hex)
                .map_err(|_| jsonrpc_core::Error::invalid_params("Invalid contract address hex"))?;
            if contract_bytes.len() != 20 {
                return Err(jsonrpc_core::Error::invalid_params("Contract address must be 20 bytes"));
            }
            let mut contract_addr = [0u8; 20];
            contract_addr.copy_from_slice(&contract_bytes);
            let contract_address = blockchain_core::contracts::EthAddress::new(contract_addr);
            
            let calldata = hex::decode(calldata_hex)
                .map_err(|_| jsonrpc_core::Error::invalid_params("Invalid calldata hex"))?;
            
            let result = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    bc.call_contract(caller, contract_address, calldata, value, gas_limit).await
                })
            }).map_err(|e| jsonrpc_core::Error::internal_error())?;
            
            Ok(serde_json::to_value(result).unwrap())
        });
    }
    
    // Contract: Get code
    {
        let bc = blockchain.clone();
        handler.add_sync_method("contract_getCode", move |params: Params| {
            let bc = bc.clone();
            let parsed: serde_json::Map<String, Value> = params.parse()?;
            
            let contract_hex = parsed.get("contract")
                .and_then(|v| v.as_str())
                .ok_or_else(|| jsonrpc_core::Error::invalid_params("Missing contract address"))?;
                
            let contract_bytes = hex::decode(contract_hex)
                .map_err(|_| jsonrpc_core::Error::invalid_params("Invalid contract address hex"))?;
            if contract_bytes.len() != 20 {
                return Err(jsonrpc_core::Error::invalid_params("Contract address must be 20 bytes"));
            }
            let mut contract_addr = [0u8; 20];
            contract_addr.copy_from_slice(&contract_bytes);
            let contract_address = blockchain_core::contracts::EthAddress::new(contract_addr);
            
            let code = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    bc.get_contract_code(contract_address).await
                })
            });
            
            Ok(json!({
                "code": code.map(|c| hex::encode(c))
            }))
        });
    }
    
    info!("📋 Registered RPC methods: blockchain_getBlockHeight, blockchain_getBlock, wallet_getBalance, wallet_list, blockchain_getStatus, treasury_getPrice, treasury_setPrice, treasury_sellCoins, treasury_getStats, treasury_getSales, contract_deploy, contract_call, contract_getCode");
    RpcServer::with_custom_handler(config, handler)
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    
    let cli = Cli::parse();
    
    info!("🚀 Starting EduNet Blockchain Full Node");
    info!("📁 Data directory: {}", cli.data_dir.display());
    
    // Create data directory
    std::fs::create_dir_all(&cli.data_dir)?;
    
    // Parse bootstrap peers
    let seed_peers: Vec<SocketAddr> = if let Some(peers) = cli.bootstrap_peers {
        peers.split(',')
            .filter_map(|p| {
                p.trim().parse().ok().or_else(|| {
                    error!("Invalid peer address: {}", p);
                    None
                })
            })
            .collect()
    } else {
        Vec::new()
    };
    
    // Configure P2P network
    info!("🌐 Configuring P2P network on port {}...", cli.p2p_port);
    let network_config = NetworkConfig {
        listen_addr: format!("0.0.0.0:{}", cli.p2p_port).parse()?,
        listening_port: cli.p2p_port,
        seed_peers,
        dns_seeds: vec![],
        our_services: 1,
        max_peers: 58,
        connection_timeout: std::time::Duration::from_secs(30),
        heartbeat_interval: std::time::Duration::from_secs(30),
        max_message_size: 32 * 1024 * 1024,
        network_magic: 0xED000001,
    };
    
    // Initialize blockchain backend
    info!("💾 Initializing blockchain backend...");
    let blockchain = Arc::new(BlockchainBackend::new(network_config).await?);
    
    info!("✅ Blockchain initialized at height {}", blockchain.get_height().await);
    
    // Initialize treasury manager
    info!("💰 Initializing treasury manager...");
    let treasury = Arc::new(TreasuryManager::new(blockchain.clone())?);
    info!("✅ Treasury ready - Address: {}", treasury::TREASURY_ADDRESS);
    
    // Start RPC server
    info!("🔌 Starting RPC server on {}:{}...", cli.rpc_host, cli.rpc_port);
    let rpc_config = RpcServerConfig {
        host: cli.rpc_host.clone(),
        port: cli.rpc_port,
    };
    
    // Create RPC handler with blockchain and treasury access
    let rpc_server = create_blockchain_rpc_server(rpc_config, blockchain.clone(), treasury.clone());
    
    match rpc_server.start() {
        Ok(server) => {
            info!("✅ Blockchain full node is running!");
            info!("📡 RPC endpoint: http://{}:{}", cli.rpc_host, cli.rpc_port);
            info!("🌐 P2P listening on port: {}", cli.p2p_port);
            info!("⛓️  Block height: {}", blockchain.get_height().await);
            
            // Start mining daemon if requested
            let mining_handle = if cli.mining {
                let validator_addr = cli.validator_address
                    .unwrap_or_else(|| "default_validator".to_string());
                info!("⛏️  Mining enabled - Rewards to: {}", validator_addr);
                
                let mining_daemon = MiningDaemon::new(blockchain.clone(), validator_addr);
                Some(mining_daemon.start())
            } else {
                info!("💤 Mining disabled (use --mining to enable)");
                None
            };
            
            info!("🚀 Node is ready! Press Ctrl+C to stop");
            
            // Keep server running
            server.wait();
            
            // Clean up mining if it was started
            if let Some(handle) = mining_handle {
                handle.abort();
            }
            
            Ok(())
        }
        Err(e) => {
            error!("❌ Failed to start RPC server: {}", e);
            Err(anyhow::anyhow!(e))
        }
    }
}
