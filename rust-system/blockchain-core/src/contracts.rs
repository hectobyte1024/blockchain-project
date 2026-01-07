//! Smart Contract Execution Engine
//! 
//! Integrates revm (Rust EVM) for Ethereum-compatible smart contracts

use crate::{BlockchainError, Result, Hash256, Address as BlockchainAddress};
use revm::{
    primitives::{Address, Bytecode, TransactTo, TxEnv, U256, B256, Bytes},
    Database, DatabaseCommit, InMemoryDB, Evm,
};
use serde::{Deserialize, Serialize, Serializer, Deserializer};
use std::collections::HashMap;
use std::sync::Arc;
use std::path::{Path, PathBuf};
use tokio::sync::RwLock;
use tokio::fs;

/// Hex serialization helper
mod hex_serde {
    use serde::{Deserialize, Deserializer, Serializer};
    
    pub fn serialize<S>(data: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&hex::encode(data))
    }
    
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        hex::decode(&s).map_err(serde::de::Error::custom)
    }
}

/// Ethereum address wrapper for serialization (20 bytes)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EthAddress([u8; 20]);

impl EthAddress {
    pub fn new(bytes: [u8; 20]) -> Self {
        Self(bytes)
    }
    
    pub fn from_address(addr: Address) -> Self {
        Self(addr.0 .0)
    }
    
    pub fn to_address(&self) -> Address {
        Address::from(self.0)
    }
    
    pub fn as_bytes(&self) -> &[u8; 20] {
        &self.0
    }
}

impl Serialize for EthAddress {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&hex::encode(self.0))
    }
}

impl<'de> Deserialize<'de> for EthAddress {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let bytes = hex::decode(&s).map_err(serde::de::Error::custom)?;
        if bytes.len() != 20 {
            return Err(serde::de::Error::custom("Invalid address length"));
        }
        let mut arr = [0u8; 20];
        arr.copy_from_slice(&bytes);
        Ok(EthAddress(arr))
    }
}

/// U256 wrapper for serialization
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SerU256(pub U256);

impl Serialize for SerU256 {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de> Deserialize<'de> for SerU256 {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let val = s.parse::<u128>().map_err(serde::de::Error::custom)?;
        Ok(SerU256(U256::from(val)))
    }
}

/// Contract account state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractAccount {
    /// Contract address (20 bytes Ethereum-style)
    pub address: EthAddress,
    /// Contract bytecode
    pub code: Vec<u8>,
    /// Contract storage (key-value pairs, serialized as strings)
    pub storage: HashMap<String, String>, // Store as hex strings for serialization
    /// Account balance (in wei/satoshis)
    pub balance: u64,
    /// Account nonce (for contract creation)
    pub nonce: u64,
    /// Block height when contract was deployed
    pub deployed_at: u64,
}

/// Contract execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// Whether execution succeeded
    pub success: bool,
    /// Gas used
    pub gas_used: u64,
    /// Return data from contract
    #[serde(with = "hex_serde")]
    pub return_data: Vec<u8>,
    /// Logs emitted
    pub logs: Vec<Log>,
    /// New contract address (if deployment)
    pub contract_address: Option<EthAddress>,
    /// Error message if failed
    pub error: Option<String>,
}

/// Contract event log
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Log {
    pub address: EthAddress,
    pub topics: Vec<String>, // Hex-encoded B256
    #[serde(with = "hex_serde")]
    pub data: Vec<u8>,
}

/// Smart contract executor using revm
pub struct ContractExecutor {
    /// Contract state storage
    contracts: Arc<RwLock<HashMap<EthAddress, ContractAccount>>>,
    /// Account balances (EDU address -> balance)
    balances: Arc<RwLock<HashMap<String, u64>>>,
    /// Storage path for contract persistence
    storage_path: PathBuf,
}

impl ContractExecutor {
    /// Create new contract executor
    pub fn new() -> Self {
        Self::with_path("contract-data")
    }
    
    /// Create new contract executor with custom storage path
    pub fn with_path<P: AsRef<Path>>(path: P) -> Self {
        Self {
            contracts: Arc::new(RwLock::new(HashMap::new())),
            balances: Arc::new(RwLock::new(HashMap::new())),
            storage_path: path.as_ref().to_path_buf(),
        }
    }
    
    /// Load all contracts from disk
    pub async fn load_contracts(&self) -> Result<()> {
        // Create storage directory if it doesn't exist
        if !self.storage_path.exists() {
            fs::create_dir_all(&self.storage_path).await
                .map_err(|e| BlockchainError::StorageError(e.to_string()))?;
            return Ok(());
        }
        
        // Read all contract files
        let mut entries = fs::read_dir(&self.storage_path).await
            .map_err(|e| BlockchainError::StorageError(e.to_string()))?;
        
        let mut loaded_count = 0;
        while let Some(entry) = entries.next_entry().await
            .map_err(|e| BlockchainError::StorageError(e.to_string()))? {
            let path = entry.path();
            
            // Only load .json files
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Ok(contract) = self.load_contract_from_file(&path).await {
                    self.contracts.write().await.insert(contract.address, contract);
                    loaded_count += 1;
                }
            }
        }
        
        println!("Loaded {} contracts from disk", loaded_count);
        Ok(())
    }
    
    /// Load a single contract from file
    async fn load_contract_from_file(&self, path: &Path) -> Result<ContractAccount> {
        let data = fs::read_to_string(path).await
            .map_err(|e| BlockchainError::StorageError(e.to_string()))?;
        
        let contract: ContractAccount = serde_json::from_str(&data)
            .map_err(|e| BlockchainError::StorageError(e.to_string()))?;
        
        Ok(contract)
    }
    
    /// Save a contract to disk
    async fn save_contract(&self, contract: &ContractAccount) -> Result<()> {
        // Create storage directory if it doesn't exist
        if !self.storage_path.exists() {
            fs::create_dir_all(&self.storage_path).await
                .map_err(|e| BlockchainError::StorageError(e.to_string()))?;
        }
        
        // Generate filename from contract address
        let filename = format!("contract_{}.json", hex::encode(contract.address.as_bytes()));
        let path = self.storage_path.join(filename);
        
        // Serialize contract to JSON
        let data = serde_json::to_string_pretty(&contract)
            .map_err(|e| BlockchainError::StorageError(e.to_string()))?;
        
        // Write to file
        fs::write(&path, data).await
            .map_err(|e| BlockchainError::StorageError(e.to_string()))?;
        
        Ok(())
    }

    /// Deploy a new contract
    pub async fn deploy_contract(
        &self,
        deployer: &str,
        bytecode: Vec<u8>,
        value: u64,
        gas_limit: u64,
    ) -> Result<ExecutionResult> {
        // Convert EDU address to Ethereum address (use hash)
        let deployer_addr = self.edu_to_eth_address(deployer)?;
        
        // Get deployer balance from our tracking
        let deployer_balance = self.get_balance(deployer).await;
        
        // Create EVM with in-memory database
        let mut db = InMemoryDB::default();
        
        // Set deployer account with actual balance
        let deployer_info = revm::primitives::AccountInfo {
            balance: U256::from(deployer_balance + value + gas_limit * 10), // Ensure enough for gas + value
            nonce: 0,
            code_hash: revm::primitives::KECCAK_EMPTY,
            code: None,
        };
        db.insert_account_info(deployer_addr, deployer_info);
        
        // Configure transaction
        let mut evm = Evm::builder()
            .with_db(db)
            .modify_tx_env(|tx| {
                tx.caller = deployer_addr;
                tx.transact_to = TransactTo::Create;
                tx.data = Bytes::from(bytecode.clone());
                tx.value = U256::from(value);
                tx.gas_limit = gas_limit;
                tx.gas_price = U256::from(1);
            })
            .build();
        
        // Execute deployment
        let result = evm.transact().map_err(|e| {
            BlockchainError::ContractExecutionFailed(format!("Deployment failed: {:?}", e))
        })?;
        
        let execution_result = match result.result {
            revm::primitives::ExecutionResult::Success {
                reason: _,
                gas_used,
                gas_refunded: _,
                logs,
                output,
            } => {
                let contract_address = match output {
                    revm::primitives::Output::Create(_, addr) => addr,
                    _ => None,
                };
                
                // Store contract if deployment succeeded
                if let Some(addr) = contract_address {
                    let contract = ContractAccount {
                        address: EthAddress::from_address(addr),
                        code: bytecode,
                        storage: HashMap::new(),
                        balance: value,
                        nonce: 1,
                        deployed_at: 0, // Will be set by caller
                    };
                    
                    // Save to memory
                    self.contracts.write().await.insert(EthAddress::from_address(addr), contract.clone());
                    
                    // Persist to disk
                    if let Err(e) = self.save_contract(&contract).await {
                        eprintln!("Warning: Failed to persist contract to disk: {}", e);
                    }
                }
                
                ExecutionResult {
                    success: true,
                    gas_used,
                    return_data: Vec::new(),
                    logs: logs.into_iter().map(|log| Log {
                        address: EthAddress::from_address(log.address),
                        topics: log.topics().iter().map(|t| hex::encode(t)).collect(),
                        data: log.data.data.to_vec(),
                    }).collect(),
                    contract_address: contract_address.map(EthAddress::from_address),
                    error: None,
                }
            }
            revm::primitives::ExecutionResult::Revert { gas_used, output } => {
                ExecutionResult {
                    success: false,
                    gas_used,
                    return_data: output.to_vec(),
                    logs: Vec::new(),
                    contract_address: None,
                    error: Some("Contract reverted".to_string()),
                }
            }
            revm::primitives::ExecutionResult::Halt { reason, gas_used } => {
                ExecutionResult {
                    success: false,
                    gas_used,
                    return_data: Vec::new(),
                    logs: Vec::new(),
                    contract_address: None,
                    error: Some(format!("Execution halted: {:?}", reason)),
                }
            }
        };
        
        Ok(execution_result)
    }

    /// Call a contract function
    pub async fn call_contract(
        &self,
        caller: &str,
        contract_address: EthAddress,
        calldata: Vec<u8>,
        value: u64,
        gas_limit: u64,
    ) -> Result<ExecutionResult> {
        // Get contract code
        let contracts = self.contracts.read().await;
        let contract = contracts.get(&contract_address)
            .ok_or_else(|| BlockchainError::ContractNotFound(format!("{:?}", contract_address)))?;
        
        let caller_addr = self.edu_to_eth_address(caller)?;
        let eth_contract_addr = contract_address.to_address();
        
        // Get caller balance from our tracking
        let caller_balance = self.get_balance(caller).await;
        
        // Create EVM with contract state
        let mut db = InMemoryDB::default();
        
        // Set caller account with actual balance
        let caller_info = revm::primitives::AccountInfo {
            balance: U256::from(caller_balance + value + gas_limit * 10), // Ensure enough for gas + value
            nonce: 0,
            code_hash: revm::primitives::KECCAK_EMPTY,
            code: None,
        };
        db.insert_account_info(caller_addr, caller_info);
        
        // Set contract account
        let contract_info = revm::primitives::AccountInfo {
            balance: U256::from(contract.balance),
            nonce: contract.nonce,
            code_hash: revm::primitives::KECCAK_EMPTY, // Will be set by code
            code: Some(Bytecode::new_raw(Bytes::from(contract.code.clone()))),
        };
        db.insert_account_info(eth_contract_addr, contract_info);
        
        // Configure transaction
        let mut evm = Evm::builder()
            .with_db(db)
            .modify_tx_env(|tx| {
                tx.caller = caller_addr;
                tx.transact_to = TransactTo::Call(eth_contract_addr);
                tx.data = Bytes::from(calldata);
                tx.value = U256::from(value);
                tx.gas_limit = gas_limit;
                tx.gas_price = U256::from(1);
            })
            .build();
        
        // Execute call
        let result = evm.transact().map_err(|e| {
            BlockchainError::ContractExecutionFailed(format!("Call failed: {:?}", e))
        })?;
        
        let execution_result = match result.result {
            revm::primitives::ExecutionResult::Success {
                reason: _,
                gas_used,
                gas_refunded: _,
                logs,
                output,
            } => {
                let return_data = match output {
                    revm::primitives::Output::Call(data) => data.to_vec(),
                    _ => Vec::new(),
                };
                
                ExecutionResult {
                    success: true,
                    gas_used,
                    return_data,
                    logs: logs.into_iter().map(|log| Log {
                        address: EthAddress::from_address(log.address),
                        topics: log.topics().iter().map(|t| hex::encode(t)).collect(),
                        data: log.data.data.to_vec(),
                    }).collect(),
                    contract_address: None,
                    error: None,
                }
            }
            revm::primitives::ExecutionResult::Revert { gas_used, output } => {
                ExecutionResult {
                    success: false,
                    gas_used,
                    return_data: output.to_vec(),
                    logs: Vec::new(),
                    contract_address: None,
                    error: Some("Contract reverted".to_string()),
                }
            }
            revm::primitives::ExecutionResult::Halt { reason, gas_used } => {
                ExecutionResult {
                    success: false,
                    gas_used,
                    return_data: Vec::new(),
                    logs: Vec::new(),
                    contract_address: None,
                    error: Some(format!("Execution halted: {:?}", reason)),
                }
            }
        };
        
        Ok(execution_result)
    }

    /// Get contract by address
    pub async fn get_contract(&self, address: EthAddress) -> Option<ContractAccount> {
        self.contracts.read().await.get(&address).cloned()
    }

    /// Convert EDU address to Ethereum address (20 bytes)
    fn edu_to_eth_address(&self, edu_address: &str) -> Result<Address> {
        use sha2::{Sha256, Digest};
        
        // Hash the EDU address to get 32 bytes, then take first 20
        let mut hasher = Sha256::new();
        hasher.update(edu_address.as_bytes());
        let hash = hasher.finalize();
        
        let mut addr_bytes = [0u8; 20];
        addr_bytes.copy_from_slice(&hash[0..20]);
        
        Ok(Address::from(addr_bytes))
    }

    /// Set account balance (for testing)
    pub async fn set_balance(&self, address: &str, balance: u64) {
        self.balances.write().await.insert(address.to_string(), balance);
    }

    /// Get account balance
    pub async fn get_balance(&self, address: &str) -> u64 {
        *self.balances.read().await.get(address).unwrap_or(&0)
    }
}

impl Default for ContractExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_deploy_simple_contract() {
        let executor = ContractExecutor::new();
        
        // Simple contract bytecode (just returns)
        let bytecode = vec![0x60, 0x00, 0x60, 0x00, 0xf3]; // PUSH1 0 PUSH1 0 RETURN
        
        let result = executor.deploy_contract(
            "edu1qTestDeployer000000000000000",
            bytecode,
            0,
            100000
        ).await.unwrap();
        
        assert!(result.success);
        assert!(result.contract_address.is_some());
    }
}
