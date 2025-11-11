//! Minimal block module

use crate::{BlockchainError, Result, Hash256, BlockHeight, Timestamp};
use crate::transaction::{Transaction, TransactionInput, TransactionOutput, UTXO, UTXOSet};
use blockchain_ffi::types::Hash256Wrapper;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockHeader {
    pub version: u32,
    pub prev_block_hash: Hash256,
    pub merkle_root: Hash256,
    pub timestamp: u32,
    pub difficulty_target: u32,
    pub nonce: u32,
    pub height: u32,
}

impl BlockHeader {
    pub fn new(version: u32, prev_block_hash: Hash256, merkle_root: Hash256, difficulty_target: u32, height: u32) -> Self {
        Self {
            version,
            prev_block_hash,
            merkle_root,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as u32,
            difficulty_target,
            nonce: 0,
            height,
        }
    }
    
    pub fn calculate_hash(&self) -> Hash256 {
        let data = format!(
            "{}{}{}{}{}{}{}",
            self.version,
            hex::encode(self.prev_block_hash),
            hex::encode(self.merkle_root),
            self.timestamp,
            self.difficulty_target,
            self.nonce,
            self.height
        );
        crate::utils::double_sha256(data.as_bytes())
    }
    
    pub fn get_hex_hash(&self) -> String {
        hex::encode(self.calculate_hash())
    }
    
    pub fn meets_difficulty_target(&self) -> bool {
        let hash = self.calculate_hash();
        let leading_zeros = (32 - self.difficulty_target.leading_zeros() / 4) as usize;
        let hash_hex = hex::encode(hash);
        hash_hex.starts_with(&"0".repeat(leading_zeros))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub header: BlockHeader,
    pub transactions: Vec<Transaction>,
    cached_hash: Option<Hash256>,
}

impl Block {
    pub fn new(header: BlockHeader, transactions: Vec<Transaction>) -> Self {
        Self {
            header,
            transactions,
            cached_hash: None,
        }
    }
    
    pub fn get_hash(&self) -> Hash256 {
        if let Some(hash) = self.cached_hash {
            hash
        } else {
            self.header.calculate_hash()
        }
    }
    
    pub fn get_hex_hash(&self) -> String {
        hex::encode(self.get_hash())
    }
    
    pub fn calculate_merkle_root(&self) -> Hash256 {
        if self.transactions.is_empty() {
            return [0u8; 32];
        }
        
        let tx_hashes: Vec<Hash256> = self.transactions
            .iter()
            .map(|tx| {
                let txid_hex = tx.get_txid();
                let txid_bytes = hex::decode(&txid_hex).unwrap_or_default();
                let mut hash = [0u8; 32];
                if txid_bytes.len() == 32 {
                    hash.copy_from_slice(&txid_bytes);
                }
                hash
            }).collect();
            
        Self::compute_merkle_root(tx_hashes)
    }

    /// Serialize block header for mining (without nonce)
    pub fn serialize_for_mining(&self) -> Result<Vec<u8>> {
        let mut data = Vec::new();
        
        // Version (4 bytes, little-endian)
        data.extend_from_slice(&self.header.version.to_le_bytes());
        
        // Previous block hash (32 bytes)
        data.extend_from_slice(&self.header.prev_block_hash);
        
        // Merkle root (32 bytes)
        data.extend_from_slice(&self.header.merkle_root);
        
        // Timestamp (8 bytes, little-endian)
        data.extend_from_slice(&self.header.timestamp.to_le_bytes());
        
        // Difficulty target (4 bytes, little-endian)
        data.extend_from_slice(&self.header.difficulty_target.to_le_bytes());
        
        // Block height (4 bytes, little-endian)
        data.extend_from_slice(&self.header.height.to_le_bytes());
        
        // Note: Nonce is NOT included here - it will be added by the mining process
        
        Ok(data)
    }
    
    fn compute_merkle_root(mut hashes: Vec<Hash256>) -> Hash256 {
        if hashes.is_empty() {
            return [0u8; 32];
        }
        
        while hashes.len() > 1 {
            let mut next_level = Vec::new();
            
            for chunk in hashes.chunks(2) {
                let left = &chunk[0];
                let right = chunk.get(1).unwrap_or(&chunk[0]);
                
                let combined_data = format!("{}{}", hex::encode(left), hex::encode(right));
                next_level.push(crate::utils::double_sha256(combined_data.as_bytes()));
            }
            
            hashes = next_level;
        }
        
        hashes.into_iter().next().unwrap_or([0u8; 32])
    }
    
    pub fn create_genesis_block(genesis_message: &str) -> Self {
        // Create genesis mining address
        let genesis_address = crate::script_utils::ScriptBuilder::generate_mining_address("genesis");
        let genesis_script = crate::script_utils::ScriptBuilder::create_coinbase_script(&genesis_address)
            .unwrap_or_else(|_| vec![0x76, 0xa9, 0x14, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x88, 0xac]);
        
        let coinbase_tx = Transaction::new(
            1,
            vec![TransactionInput::new(Hash256Wrapper::from_hash256(&[0u8; 32]), u32::MAX, genesis_message.as_bytes().to_vec())],
            vec![TransactionOutput::new(5_000_000_000, genesis_script)],
        );
        
        let transactions = vec![coinbase_tx];
        let merkle_root = {
            let mut block = Block::new(
                BlockHeader::new(1, [0u8; 32], [0u8; 32], 0x207FFFFF, 0), // Genesis block height = 0
                transactions.clone()
            );
            block.calculate_merkle_root()
        };
        
        let header = BlockHeader::new(1, [0u8; 32], merkle_root, 0x207FFFFF, 0); // Genesis block height = 0
        
        Block::new(header, transactions)
    }
    
    pub fn mine(&mut self, max_iterations: u64) -> bool {
        for i in 0..max_iterations {
            self.header.nonce = i as u32;
            if self.header.meets_difficulty_target() {
                self.cached_hash = Some(self.header.calculate_hash());
                return true;
            }
        }
        false
    }
}

#[derive(Debug, Default)]
pub struct Blockchain {
    pub blocks: Vec<Block>,
    pub utxo_set: UTXOSet,
    pub current_difficulty: u32,
}

impl Blockchain {
    pub fn new() -> Self {
        let mut blockchain = Self {
            blocks: Vec::new(),
            utxo_set: UTXOSet::new(),
            current_difficulty: 0x207FFFFF, // Easy difficulty for testing
        };
        
        let genesis = Block::create_genesis_block("Genesis Block");
        blockchain.add_block(genesis).unwrap_or_default();
        
        blockchain
    }
    
    pub fn add_block(&mut self, block: Block) -> Result<()> {
        // Basic validation
        if !block.transactions.is_empty() {
            for tx in &block.transactions {
                if !tx.is_valid() {
                    return Err(BlockchainError::InvalidTransaction("Invalid transaction structure".to_string()));
                }
            }
        }
        
        self.blocks.push(block);
        Ok(())
    }
    
    pub fn get_latest_block(&self) -> Option<&Block> {
        self.blocks.last()
    }
    
    pub fn get_block_height(&self) -> BlockHeight {
        self.blocks.len() as BlockHeight
    }
    
    pub fn is_valid(&self) -> bool {
        !self.blocks.is_empty()
    }
}
