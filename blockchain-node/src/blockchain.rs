//! Blockchain Backend for Full Node
//! 
//! This module provides the complete blockchain implementation for the full node daemon.
//! Ported from edunet-web's working blockchain integration.

use blockchain_core::consensus::ConsensusValidator;
use blockchain_core::wallet::WalletManager;
use blockchain_core::genesis::{GenesisCreator, GenesisConfig};
use blockchain_core::transaction::{Transaction, TransactionInput, TransactionOutput};
use blockchain_core::block::Block;
use blockchain_core::utxo::{UTXOSet, UTXO};
use blockchain_core::mempool::{Mempool, MempoolConfig};
use blockchain_core::tx_builder::{TransactionBuilder, TransactionManager};
use blockchain_core::sync::{SyncEngine, SyncConfig};
use blockchain_core::contracts::{ContractExecutor, ExecutionResult};
use blockchain_core::{Hash256, Amount, Result as BlockchainResult};

use blockchain_network::NetworkConfig;
use blockchain_network::NetworkManager;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, error, warn};
use hex;

#[derive(Clone)]
pub struct BlockchainBackend {
    pub network: Arc<NetworkManager>,
    pub consensus: Arc<ConsensusValidator>,
    pub wallets: Arc<RwLock<WalletManager>>,
    pub mempool: Arc<RwLock<Mempool>>,
    pub utxo_set: Arc<RwLock<UTXOSet>>,
    pub tx_manager: Arc<TransactionManager>,
    pub sync_engine: Arc<SyncEngine>,
    pub contract_executor: Arc<ContractExecutor>,
}

impl BlockchainBackend {
    pub async fn new(network_config: NetworkConfig) -> Result<Self> {
        info!("🚀 Initializing blockchain backend for full node...");

        // Initialize UTXO set
        let utxo_set = Arc::new(RwLock::new(UTXOSet::new()));
        
        // Initialize disk storage for blocks
        let data_dir = std::path::PathBuf::from("./blockchain-data/blocks");
        let storage = Arc::new(
            blockchain_core::storage::DiskBlockStorage::new(&data_dir)
                .map_err(|e| anyhow::anyhow!("Failed to initialize block storage: {}", e))?
        );
        info!("💾 Block storage initialized at: {}", data_dir.display());
        
        // Initialize consensus with genesis block and storage
        let consensus_params = blockchain_core::consensus::ConsensusParams::default();
        let consensus = Arc::new(
            ConsensusValidator::new(consensus_params)
                .with_storage(storage.clone())
        );
        
        // Create genesis state
        let genesis_creator = GenesisCreator::new(Some(GenesisConfig::default()));
        let genesis_state = genesis_creator.create_genesis_state()
            .map_err(|e| anyhow::anyhow!("Failed to create genesis state: {}", e))?;
        
        let total_supply = genesis_state.get_total_supply_edu();
        info!("🎯 Genesis block created with {} EDU total supply", total_supply);
        
        // Initialize consensus with genesis
        consensus.initialize_with_genesis(genesis_state).await
            .map_err(|e| anyhow::anyhow!("Failed to initialize consensus with genesis: {}", e))?;
        
        // Sync UTXO set from consensus to backend
        {
            let consensus_utxo_set = consensus.get_utxo_set().await;
            let utxo_count = consensus_utxo_set.get_utxo_count();
            let mut backend_utxo_set = utxo_set.write().await;
            *backend_utxo_set = consensus_utxo_set;
            info!("✅ UTXO set synchronized with consensus ({} UTXOs)", utxo_count);
        }
        
        info!("✅ Genesis block initialized and persisted");

        // Initialize wallet manager
        let wallets = Arc::new(RwLock::new(WalletManager::new()));
        
        // Initialize mempool with consensus for proper fee calculation
        let mempool_config = MempoolConfig::default();
        let mut mempool_instance = Mempool::new(mempool_config);
        mempool_instance.set_consensus_validator(consensus.clone());
        let mempool = Arc::new(RwLock::new(mempool_instance));
        
        // Initialize transaction manager
        let utxo_clone = utxo_set.read().await.clone();
        let tx_manager = Arc::new(TransactionManager::new(utxo_clone));

        // Initialize network with consensus
        let network = NetworkManager::new(network_config, Some(consensus.clone()))?;

        // Initialize sync engine
        let sync_config = SyncConfig::default();
        let sync_engine = Arc::new(SyncEngine::new(sync_config, consensus.clone()));
        
        // Initialize contract executor with persistence
        let contract_executor = Arc::new(ContractExecutor::with_path("blockchain-data/contracts"));
        
        // Load existing contracts from disk
        if let Err(e) = contract_executor.load_contracts().await {
            warn!("Failed to load contracts from disk: {}", e);
        }
        
        info!("✅ Blockchain backend initialized");
        info!("🔐 Features: HMAC-SHA256 signatures, UTXO validation, PoW consensus, SQLite persistence, Smart Contracts (EVM)");

        Ok(Self {
            network: Arc::new(network),
            consensus,
            wallets,
            mempool,
            utxo_set,
            tx_manager,
            sync_engine,
            contract_executor,
        })
    }

    /// Get current blockchain height
    pub async fn get_height(&self) -> u64 {
        let chain_state = self.consensus.get_chain_state().await;
        chain_state.height
    }

    /// Get block by height
    pub async fn get_block_by_height(&self, height: u64) -> Option<Block> {
        self.consensus.get_block_by_height(height).await
    }

    /// Get block by hash
    pub async fn get_block_by_hash(&self, hash: &Hash256) -> Option<Block> {
        // TODO: Implement get_block_by_hash in ConsensusValidator
        // For now, return None
        None
    }

    /// Submit transaction to mempool
    pub async fn submit_transaction(&self, tx: Transaction) -> BlockchainResult<Hash256> {
        let tx_hash: Hash256 = tx.get_hash()?;
        
        // Validate and add transaction
        let mut mempool = self.mempool.write().await;
        mempool.add_transaction(tx).await?;
        
        info!("✅ Transaction {} added to mempool", hex::encode(&tx_hash));
        
        Ok(tx_hash)
    }

    /// Get pending transactions from mempool
    pub async fn get_pending_transactions(&self) -> Vec<Transaction> {
        let mempool = self.mempool.read().await;
        // Get transactions from mempool
        mempool.get_transactions()
    }

    /// Get mempool stats
    pub async fn get_mempool_stats(&self) -> serde_json::Value {
        let mempool = self.mempool.read().await;
        
        serde_json::json!({
            "transactions": mempool.transaction_count(),
            "memory_usage": mempool.memory_usage(),
        })
    }

    /// Get network stats
    pub async fn get_network_stats(&self) -> serde_json::Value {
        let peers = self.network.get_connected_peers().await;
        
        serde_json::json!({
            "connected_peers": peers.len(),
            "peer_addresses": peers,
        })
    }

    /// Get full node status
    pub async fn get_status(&self) -> serde_json::Value {
        let chain_state = self.consensus.get_chain_state().await;
        let mempool = self.mempool.read().await;
        let mempool_stats = mempool.get_stats();
        let peers = self.network.get_connected_peers().await;
        
        serde_json::json!({
            "block_height": chain_state.height,
            "best_block_hash": hex::encode(&chain_state.best_block_hash),
            "total_work": chain_state.total_work,
            "difficulty": chain_state.next_difficulty,
            "mempool": {
                "transactions": mempool_stats.transaction_count,
                "memory_usage": mempool_stats.memory_usage,
            },
            "network": {
                "connected_peers": peers.len(),
            },
        })
    }

    /// Create a new wallet
    pub async fn create_wallet(&self, name: &str) -> Result<String> {
        let mut wallets = self.wallets.write().await;
        let wallet = wallets.create_wallet(name.to_string())
            .map_err(|e| anyhow::anyhow!("Failed to create wallet: {}", e))?;
        
        Ok(wallet.address.clone())
    }

    /// Get wallet balance
    pub async fn get_balance(&self, address: &str) -> Result<u64> {
        let utxo_set = self.utxo_set.read().await;
        let utxos = utxo_set.get_utxos_for_address(address);
        let balance: u64 = utxos.iter().map(|utxo| utxo.value()).sum();
        Ok(balance)
    }
    
    /// List all wallets with balances
    pub async fn list_wallets(&self) -> Vec<(String, String, u64)> {
        let wallets = self.wallets.read().await;
        let utxo_set = self.utxo_set.read().await;
        
        let wallet_list = wallets.list_wallets();
        wallet_list.iter().map(|wallet| {
            let utxos = utxo_set.get_utxos_for_address(&wallet.address);
            let balance: u64 = utxos.iter().map(|utxo| utxo.value()).sum();
            (wallet.name.clone(), wallet.address.clone(), balance)
        }).collect()
    }
    
    /// Deploy a smart contract
    pub async fn deploy_contract(
        &self,
        deployer: &str,
        bytecode: Vec<u8>,
        value: u64,
        gas_limit: u64,
    ) -> Result<ExecutionResult> {
        info!("📝 Deploying contract from {}", deployer);
        
        // Get deployer's actual balance from UTXO set
        let balance = self.get_balance(deployer).await?;
        
        // Set balance in contract executor so it has funds for gas
        self.contract_executor.set_balance(deployer, balance).await;
        
        let result = self.contract_executor.deploy_contract(
            deployer,
            bytecode,
            value,
            gas_limit
        ).await.map_err(|e| anyhow::anyhow!("Contract deployment failed: {}", e))?;
        
        if result.success {
            if let Some(addr) = result.contract_address {
                info!("✅ Contract deployed at {:?}", addr);
            }
        } else {
            warn!("❌ Contract deployment failed: {:?}", result.error);
        }
        
        Ok(result)
    }
    
    /// Call a smart contract function
    pub async fn call_contract(
        &self,
        caller: &str,
        contract_address: blockchain_core::contracts::EthAddress,
        calldata: Vec<u8>,
        value: u64,
        gas_limit: u64,
    ) -> Result<ExecutionResult> {
        info!("📞 Calling contract {:?} from {}", contract_address, caller);
        
        // Get caller's actual balance from UTXO set
        let balance = self.get_balance(caller).await?;
        
        // Set balance in contract executor so it has funds for gas
        self.contract_executor.set_balance(caller, balance).await;
        
        let result = self.contract_executor.call_contract(
            caller,
            contract_address,
            calldata,
            value,
            gas_limit
        ).await.map_err(|e| anyhow::anyhow!("Contract call failed: {}", e))?;
        
        if result.success {
            info!("✅ Contract call succeeded, gas used: {}", result.gas_used);
        } else {
            warn!("❌ Contract call failed: {:?}", result.error);
        }
        
        Ok(result)
    }
    
    /// Get contract code
    pub async fn get_contract_code(
        &self,
        contract_address: blockchain_core::contracts::EthAddress,
    ) -> Option<Vec<u8>> {
        self.contract_executor.get_contract(contract_address)
            .await
            .map(|contract| contract.code)
    }
}
