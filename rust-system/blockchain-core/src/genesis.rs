use crate::{
    block::{Block, BlockHeader}, 
    transaction::{Transaction, TransactionInput, TransactionOutput}, 
    utxo::UTXOSet,
    BlockchainError, Result
};
use serde::{Deserialize, Serialize};
use chrono::Utc;

/// Genesis block and initial state configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisConfig {
    pub initial_accounts: Vec<GenesisAccount>,
    pub genesis_timestamp: i64,
    pub network_id: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisAccount {
    pub address: String,
    pub balance: u64, // In satoshis (1e8 = 1 EDU)
    pub description: String,
}

impl Default for GenesisConfig {
    fn default() -> Self {
        Self {
            initial_accounts: vec![
                // Total Supply: 2,000,000 EDU (200,000,000,000,000 satoshis)
                GenesisAccount {
                    address: "edu1qGenesis00000000000000000000".to_string(),
                    balance: 10000000000000, // 100,000 EDU - Genesis distribution pool
                    description: "Genesis Distribution Pool".to_string(),
                },
                GenesisAccount {
                    address: "edu1qMiner000000000000000000000".to_string(),
                    balance: 50000000000000, // 500,000 EDU - Mining rewards pool
                    description: "Mining Rewards Pool".to_string(),
                },
                GenesisAccount {
                    address: "edu1qTreasury00000000000000000000".to_string(),
                    balance: 30000000000000, // 300,000 EDU - Platform treasury
                    description: "Platform Treasury".to_string(),
                },
                GenesisAccount {
                    address: "edu1qLoanPool00000000000000000000".to_string(),
                    balance: 60000000000000, // 600,000 EDU - Student loan pool
                    description: "Student Loan Pool".to_string(),
                },
                GenesisAccount {
                    address: "edu1qInvestment000000000000000000".to_string(),
                    balance: 40000000000000, // 400,000 EDU - Education investment fund
                    description: "Education Investment Fund".to_string(),
                },
                GenesisAccount {
                    address: "edu1qFoundation000000000000000000".to_string(),
                    balance: 10000000000000, // 100,000 EDU - Foundation reserves
                    description: "EduNet Foundation Reserves".to_string(),
                },
            ],
            genesis_timestamp: Utc::now().timestamp(),
            network_id: 0x45444e45, // "EDNE" in hex
        }
    }
}

/// Genesis block creator
pub struct GenesisCreator {
    config: GenesisConfig,
}

impl GenesisCreator {
    pub fn new(config: Option<GenesisConfig>) -> Self {
        Self {
            config: config.unwrap_or_default(),
        }
    }

    /// Create the genesis block with initial UTXOs
    pub fn create_genesis_block(&self) -> Result<Block> {
        // Create coinbase transaction that mints initial tokens
        let genesis_tx = self.create_genesis_transaction()?;
        
        let genesis_header = BlockHeader {
            version: 1,
            prev_block_hash: [0u8; 32], // Genesis has no previous block
            merkle_root: genesis_tx.get_hash()?,
            timestamp: self.config.genesis_timestamp as u32,
            difficulty_target: 0x1d00ffff, // Initial difficulty
            nonce: 0,
            height: 0, // Genesis block height = 0
        };

        let genesis_block = Block::new(
            genesis_header,
            vec![genesis_tx],
        );

        Ok(genesis_block)
    }

    /// Create the genesis coinbase transaction that distributes initial funds
    fn create_genesis_transaction(&self) -> Result<Transaction> {
        // Create outputs for each initial account
        let mut outputs = Vec::new();
        
        for account in &self.config.initial_accounts {
            let output = TransactionOutput::create_p2pkh(
                account.balance,
                &account.address
            )?;
            outputs.push(output);
        }

        // Create coinbase input (special case for genesis)
        let coinbase_input = TransactionInput::create_coinbase(
            format!("Edunet Genesis Block - {}", self.config.genesis_timestamp).into_bytes()
        );

        let genesis_tx = Transaction::new(
            1, // version
            vec![coinbase_input],
            outputs,
        );

        Ok(genesis_tx)
    }

    /// Initialize the UTXO set with genesis block UTXOs
    pub fn initialize_utxo_set(&self, genesis_block: &Block) -> Result<UTXOSet> {
        let mut utxo_set = UTXOSet::new();
        
        // Add genesis transaction UTXOs
        if let Some(genesis_tx) = genesis_block.transactions.first() {
            utxo_set.add_transaction(genesis_tx, 0)?;
        }
        
        Ok(utxo_set)
    }

    /// Get the complete genesis state
    pub fn create_genesis_state(&self) -> Result<GenesisState> {
        let genesis_block = self.create_genesis_block()?;
        let utxo_set = self.initialize_utxo_set(&genesis_block)?;
        
        Ok(GenesisState {
            genesis_block,
            utxo_set,
            config: self.config.clone(),
        })
    }
}

/// Complete genesis state including block and UTXO set
#[derive(Debug)]
pub struct GenesisState {
    pub genesis_block: Block,
    pub utxo_set: UTXOSet,
    pub config: GenesisConfig,
}

impl GenesisState {
    /// Verify that the genesis state is valid
    pub fn verify(&self) -> Result<()> {
        // Verify genesis block
        if !self.genesis_block.header.prev_block_hash.iter().all(|&b| b == 0) {
            return Err(BlockchainError::InvalidBlock("Genesis block must have zero previous hash".to_string()).into());
        }

        // Verify genesis transaction is coinbase
        let genesis_tx = self.genesis_block.transactions.first()
            .ok_or_else(|| BlockchainError::InvalidBlock("Genesis block must have at least one transaction".to_string()))?;
        
        if !genesis_tx.is_coinbase() {
            return Err(BlockchainError::InvalidTransaction("Genesis transaction must be coinbase".to_string()).into());
        }

        // Verify output count matches config
        if genesis_tx.outputs.len() != self.config.initial_accounts.len() {
            return Err(BlockchainError::InvalidTransaction("Genesis outputs don't match config".to_string()).into());
        }

        // Verify balances match
        for (i, account) in self.config.initial_accounts.iter().enumerate() {
            if genesis_tx.outputs[i].value != account.balance {
                return Err(BlockchainError::InvalidTransaction(
                    format!("Genesis output {} has wrong value", i)
                ).into());
            }
        }

        Ok(())
    }

    /// Get account information
    pub fn get_account_info(&self, address: &str) -> Option<&GenesisAccount> {
        self.config.initial_accounts.iter().find(|acc| acc.address == address)
    }

    /// Get total initial supply
    pub fn get_total_supply(&self) -> u64 {
        self.config.initial_accounts.iter().map(|acc| acc.balance).sum()
    }

    /// Convert supply to EDU tokens (divide by 1e8)
    pub fn get_total_supply_edu(&self) -> f64 {
        self.get_total_supply() as f64 / 1e8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_genesis_creation() {
        let creator = GenesisCreator::new(None);
        let genesis_state = creator.create_genesis_state().unwrap();
        
        assert!(genesis_state.verify().is_ok());
        assert_eq!(genesis_state.genesis_block.transactions.len(), 1);
        assert!(genesis_state.genesis_block.transactions[0].is_coinbase());
        
        // Check that UTXOs were created correctly
        for account in &genesis_state.config.initial_accounts {
            let balance = genesis_state.utxo_set.get_balance(&account.address);
            assert_eq!(balance, account.balance);
        }
    }

    #[test]
    fn test_custom_genesis_config() {
        let custom_config = GenesisConfig {
            initial_accounts: vec![
                GenesisAccount {
                    address: "test_address_1".to_string(),
                    balance: 1000000000, // 10 EDU
                    description: "Test Account 1".to_string(),
                },
                GenesisAccount {
                    address: "test_address_2".to_string(),
                    balance: 2000000000, // 20 EDU
                    description: "Test Account 2".to_string(),
                },
            ],
            genesis_timestamp: 1640995200, // 2022-01-01
            network_id: 0x54455354, // "TEST"
        };

        let creator = GenesisCreator::new(Some(custom_config));
        let genesis_state = creator.create_genesis_state().unwrap();
        
        assert!(genesis_state.verify().is_ok());
        assert_eq!(genesis_state.get_total_supply_edu(), 30.0); // 10 + 20 EDU
    }
}