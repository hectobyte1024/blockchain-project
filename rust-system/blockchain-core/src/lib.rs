//! Minimal blockchain core that compiles

use serde::{Deserialize, Serialize};

/// 256-bit hash type used throughout the blockchain
pub type Hash256 = [u8; 32];

/// Amount type with satoshi precision (1e-8)  
pub type Amount = u64;

/// Block height type
pub type BlockHeight = u64;

/// Timestamp type (Unix timestamp)
pub type Timestamp = u64;

/// Private key type (32 bytes)
pub type PrivateKey = [u8; 32];

/// Public key type (compressed, 33 bytes)
pub type PublicKey = Vec<u8>;

/// Signature type (DER-encoded ECDSA signature)
pub type Signature = Vec<u8>;

/// Address type (base58-encoded string)
pub type Address = String;

/// Helper trait for Hash256 operations
pub trait Hash256Ext {
    fn to_hex(&self) -> String;
    fn from_hex(hex: &str) -> Result<Hash256>;
    fn is_zero(&self) -> bool;
}

impl Hash256Ext for Hash256 {
    fn to_hex(&self) -> String {
        hex::encode(self)
    }
    
    fn from_hex(hex: &str) -> Result<Hash256> {
        let bytes = hex::decode(hex).map_err(|e| BlockchainError::InvalidInput(format!("Invalid hex: {}", e)))?;
        if bytes.len() != 32 {
            return Err(BlockchainError::InvalidInput(format!("Hash must be 32 bytes, got {}", bytes.len())));
        }
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&bytes);
        Ok(hash)
    }
    
    fn is_zero(&self) -> bool {
        self.iter().all(|&b| b == 0)
    }
}

/// Transaction outpoint reference
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OutPoint {
    pub txid: Hash256,
    pub vout: u32,
}

impl OutPoint {
    pub fn new(txid: Hash256, vout: u32) -> Self {
        Self { txid, vout }
    }
    
    pub fn is_null(&self) -> bool {
        self.txid == [0u8; 32] && self.vout == u32::MAX
    }
}

/// Error types for blockchain operations
#[derive(Debug, thiserror::Error, serde::Serialize)]
pub enum BlockchainError {
    #[error("Invalid transaction: {0}")]
    InvalidTransaction(String),
    
    #[error("Invalid block: {0}")]
    InvalidBlock(String),
    
    #[error("Invalid private key: {0}")]
    InvalidPrivateKey(String),
    
    #[error("Invalid address: {0}")]
    InvalidAddress(String),
    
    #[error("Wallet not found: {0}")]
    WalletNotFound(String),
    
    #[error("Insufficient funds: {0}")]
    InsufficientFunds(String),
    
    #[error("Serialization error: {0}")]
    SerializationError(String),
    
    #[error("UTF-8 error: {0}")]
    Utf8Error(String),
    
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    
    #[error("Consensus error: {0}")]
    ConsensusError(String),
    
    #[error("Crypto error: {0}")]
    CryptoError(String),
    
    #[error("Invalid seed: {0}")]
    InvalidSeed(String),
    
    #[error("Invalid derivation: {0}")]
    InvalidDerivation(String),
    
    #[error("Invalid signature: {0}")]
    InvalidSignature(String),
    
    #[error("Account not found: {0}")]
    AccountNotFound(u32),
    
    #[error("Wallet error: {0}")]
    WalletError(String),
    
    #[error("API error: {0}")]
    ApiError(String),
    
    #[error("Invalid multi-signature configuration: {0}")]
    InvalidMultiSig(String),
    
    #[error("Signing error: {0}")]
    SigningError(String),
    
    #[error("Invalid script: {0}")]
    InvalidScript(String),
    
    #[error("Synchronization error: {0}")]
    SyncError(String),
    
    #[error("Orphan block")]
    OrphanBlock,
    
    #[error("Contract execution failed: {0}")]
    ContractExecutionFailed(String),
    
    #[error("Contract not found: {0}")]
    ContractNotFound(String),
    
    #[error("Storage error: {0}")]
    StorageError(String),
}

impl From<std::string::FromUtf8Error> for BlockchainError {
    fn from(err: std::string::FromUtf8Error) -> Self {
        BlockchainError::Utf8Error(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, BlockchainError>;

/// Utility functions
pub mod utils {
    use super::*;
    use sha2::{Digest, Sha256};
    
    pub fn double_sha256(data: &[u8]) -> Hash256 {
        let first_hash = Sha256::digest(data);
        let second_hash = Sha256::digest(&first_hash);
        second_hash.into()
    }
    
    pub fn bytes_to_hex(bytes: &[u8]) -> String {
        hex::encode(bytes)
    }
    
    pub fn hex_to_bytes(hex: &str) -> Result<Vec<u8>> {
        hex::decode(hex).map_err(|e| BlockchainError::SerializationError(e.to_string()))
    }
}

pub mod transaction;
pub mod block;
pub mod consensus;
pub mod crypto;  // Real secp256k1 ECDSA crypto
pub mod wallet;
pub mod utxo;
pub mod tx_builder;
pub mod genesis;
// Mining implementation is in blockchain-node/src/miner.rs
pub mod mempool;
pub mod hd_wallet;
pub mod advanced_wallet;
pub mod api_server;
pub mod rest_api;
pub mod script_utils;
pub mod sync;
pub mod storage;
pub mod contracts;  // Smart contract execution (EVM)
pub mod event_indexer;  // Event indexing and filtering
