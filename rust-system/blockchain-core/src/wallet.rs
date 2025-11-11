//! Wallet functionality for the Edunet blockchain
//! 
//! Provides secure wallet generation, address creation, transaction signing,
//! and key management for the EDU cryptocurrency.

use crate::{BlockchainError, Result};
use crate::utxo::UTXOSet;
use crate::tx_builder::{TransactionBuilder, TransactionManager};
use crate::transaction::Transaction;
use serde::{Deserialize, Serialize, Serializer, Deserializer};
use std::collections::HashMap;
use std::string::FromUtf8Error;
use std::sync::Arc;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use rand::RngCore;
use sha2::{Sha256, Digest};
use tokio::sync::RwLock;

/// A cryptographic private key (32 bytes)
pub type PrivateKey = [u8; 32];

/// A cryptographic public key (33 bytes compressed)
pub type PublicKey = [u8; 33];

/// A blockchain address string (e.g., "edu1q...")
pub type Address = String;

// Helper functions for serializing byte arrays
fn serialize_private_key<S: Serializer>(key: &PrivateKey, serializer: S) -> std::result::Result<S::Ok, S::Error> {
    serializer.serialize_str(&hex::encode(key))
}

fn deserialize_private_key<'de, D: Deserializer<'de>>(deserializer: D) -> std::result::Result<PrivateKey, D::Error> {
    let s = String::deserialize(deserializer)?;
    let bytes = hex::decode(&s).map_err(serde::de::Error::custom)?;
    if bytes.len() != 32 {
        return Err(serde::de::Error::custom("Invalid private key length"));
    }
    let mut array = [0u8; 32];
    array.copy_from_slice(&bytes);
    Ok(array)
}

fn serialize_public_key<S: Serializer>(key: &PublicKey, serializer: S) -> std::result::Result<S::Ok, S::Error> {
    serializer.serialize_str(&hex::encode(key))
}

fn deserialize_public_key<'de, D: Deserializer<'de>>(deserializer: D) -> std::result::Result<PublicKey, D::Error> {
    let s = String::deserialize(deserializer)?;
    let bytes = hex::decode(&s).map_err(serde::de::Error::custom)?;
    if bytes.len() != 33 {
        return Err(serde::de::Error::custom("Invalid public key length"));
    }
    let mut array = [0u8; 33];
    array.copy_from_slice(&bytes);
    Ok(array)
}

/// Wallet containing keys and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wallet {
    pub id: Uuid,
    pub name: String,
    #[serde(serialize_with = "serialize_private_key", deserialize_with = "deserialize_private_key")]
    pub private_key: PrivateKey,
    #[serde(serialize_with = "serialize_public_key", deserialize_with = "deserialize_public_key")]
    pub public_key: PublicKey,
    pub address: Address,
    pub created_at: DateTime<Utc>,
    pub balance: u64, // in satoshis (1 EDU = 100,000,000 satoshis)
}

/// Payment request for QR codes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentRequest {
    pub address: Address,
    pub amount: Option<u64>,
    pub label: Option<String>,
    pub message: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
}

/// Transaction for sending between wallets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletTransaction {
    pub id: Uuid,
    pub from_address: Address,
    pub to_address: Address,
    pub amount: u64,
    pub fee: u64,
    pub message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub blockchain_tx_id: Option<String>,
    pub confirmations: u32,
    pub status: TransactionStatus,
}

/// Status of a wallet transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionStatus {
    Pending,      // Created but not broadcast
    Broadcasting, // Being sent to network
    Confirmed,    // Included in blockchain
    Failed,       // Transaction failed
}

/// Wallet manager for handling multiple wallets with blockchain integration
#[derive(Debug)]
pub struct WalletManager {
    wallets: HashMap<Uuid, Wallet>,
    address_to_id: HashMap<String, Uuid>,
    transactions: Vec<WalletTransaction>,
    transaction_manager: Option<Arc<RwLock<TransactionManager>>>,
}

impl Wallet {
    /// Create a new wallet with random keys
    pub fn new(name: String) -> Result<Self> {
        let private_key = Self::generate_private_key()?;
        let public_key = Self::derive_public_key(&private_key)?;
        let address = Self::derive_address(&public_key)?;
        
        Ok(Wallet {
            id: Uuid::new_v4(),
            name,
            private_key,
            public_key,
            address,
            created_at: Utc::now(),
            balance: 0,
        })
    }
    
    /// Generate a cryptographically secure private key
    fn generate_private_key() -> Result<PrivateKey> {
        let mut key = [0u8; 32];
        let mut rng = rand::thread_rng();
        rng.fill_bytes(&mut key);
        
        // Ensure key is valid (not zero, not too large)
        if key == [0u8; 32] {
            return Err(BlockchainError::InvalidPrivateKey("Generated zero key".to_string()));
        }
        
        Ok(key)
    }
    
    /// Derive public key from private key (simplified ECDSA)
    fn derive_public_key(private_key: &PrivateKey) -> Result<PublicKey> {
        // Simplified public key derivation
        // In production, use secp256k1 curve operations
        let mut hasher = Sha256::new();
        hasher.update(private_key);
        hasher.update(b"edunet_public_key");
        let hash = hasher.finalize();
        
        let mut public_key = [0u8; 33];
        public_key[0] = 0x02; // Compressed public key prefix
        public_key[1..33].copy_from_slice(&hash[0..32]);
        
        Ok(public_key)
    }
    
    /// Derive blockchain address from public key
    fn derive_address(public_key: &PublicKey) -> Result<Address> {
        // Create EDU address format: edu1q + base58 encoded hash
        let mut hasher = Sha256::new();
        hasher.update(public_key);
        let hash = hasher.finalize();
        
        // Take first 20 bytes and encode as base58
        let address_bytes = &hash[0..20];
        let encoded = bs58::encode(address_bytes).into_string();
        
        Ok(format!("edu1q{}", encoded))
    }
    
    /// Convert EDU amount to satoshis
    pub fn edu_to_satoshis(edu_amount: f64) -> u64 {
        (edu_amount * 100_000_000.0) as u64
    }
    
    /// Convert satoshis to EDU amount
    pub fn satoshis_to_edu(satoshis: u64) -> f64 {
        satoshis as f64 / 100_000_000.0
    }
    
    /// Get formatted balance string
    pub fn get_balance_string(&self) -> String {
        format!("{:.8} EDU", Self::satoshis_to_edu(self.balance))
    }
    
    /// Generate a payment request for this wallet
    pub fn create_payment_request(&self, amount: Option<f64>, message: Option<String>) -> PaymentRequest {
        PaymentRequest {
            address: self.address.clone(),
            amount: amount.map(Self::edu_to_satoshis),
            label: Some(self.name.clone()),
            message,
            expires_at: Some(Utc::now() + chrono::Duration::hours(24)),
        }
    }
}

impl PaymentRequest {
    /// Convert payment request to QR code data string
    pub fn to_qr_string(&self) -> String {
        let mut parts = vec![format!("edunet:{}", self.address)];
        
        if let Some(amount) = self.amount {
            parts.push(format!("amount={}", Wallet::satoshis_to_edu(amount)));
        }
        
        if let Some(label) = &self.label {
            parts.push(format!("label={}", urlencoding::encode(label)));
        }
        
        if let Some(message) = &self.message {
            parts.push(format!("message={}", urlencoding::encode(message)));
        }
        
        if parts.len() == 1 {
            parts[0].clone()
        } else {
            format!("{}?{}", parts[0], parts[1..].join("&"))
        }
    }
    
    /// Parse QR code data string into payment request
    pub fn from_qr_string(qr_data: &str) -> Result<Self> {
        if !qr_data.starts_with("edunet:") {
            return Err(BlockchainError::InvalidAddress("Invalid QR code format".to_string()));
        }
        
        let url = qr_data.replace("edunet:", "");
        let parts: Vec<&str> = url.split('?').collect();
        let address = parts[0].to_string();
        
        let mut payment_request = PaymentRequest {
            address,
            amount: None,
            label: None,
            message: None,
            expires_at: None,
        };
        
        if parts.len() > 1 {
            for param in parts[1].split('&') {
                let kv: Vec<&str> = param.split('=').collect();
                if kv.len() == 2 {
                    match kv[0] {
                        "amount" => {
                            if let Ok(amount) = kv[1].parse::<f64>() {
                                payment_request.amount = Some(Wallet::edu_to_satoshis(amount));
                            }
                        }
                        "label" => {
                            payment_request.label = Some(urlencoding::decode(kv[1])?.into_owned());
                        }
                        "message" => {
                            payment_request.message = Some(urlencoding::decode(kv[1])?.into_owned());
                        }
                        _ => {} // Ignore unknown parameters
                    }
                }
            }
        }
        
        Ok(payment_request)
    }
}

impl WalletManager {
    /// Create a new wallet manager
    pub fn new() -> Self {
        Self {
            wallets: HashMap::new(),
            address_to_id: HashMap::new(),
            transactions: Vec::new(),
            transaction_manager: None,
        }
    }

    /// Create a new wallet manager with blockchain integration
    pub fn with_blockchain(transaction_manager: Arc<RwLock<TransactionManager>>) -> Self {
        Self {
            wallets: HashMap::new(),
            address_to_id: HashMap::new(),
            transactions: Vec::new(),
            transaction_manager: Some(transaction_manager),
        }
    }

    /// Set the transaction manager for blockchain operations
    pub fn set_transaction_manager(&mut self, transaction_manager: Arc<RwLock<TransactionManager>>) {
        self.transaction_manager = Some(transaction_manager);
    }
    
    /// Create and add a new wallet
    pub fn create_wallet(&mut self, name: String) -> Result<&Wallet> {
        let wallet = Wallet::new(name)?;
        let wallet_id = wallet.id;
        let address = wallet.address.clone();
        
        self.address_to_id.insert(address, wallet_id);
        self.wallets.insert(wallet_id, wallet);
        
        Ok(self.wallets.get(&wallet_id).unwrap())
    }
    
    /// Get wallet by ID
    pub fn get_wallet(&self, id: &Uuid) -> Option<&Wallet> {
        self.wallets.get(id)
    }
    
    /// Get wallet by address
    pub fn get_wallet_by_address(&self, address: &str) -> Option<&Wallet> {
        self.address_to_id.get(address)
            .and_then(|id| self.wallets.get(id))
    }
    
    /// List all wallets
    pub fn list_wallets(&self) -> Vec<&Wallet> {
        self.wallets.values().collect()
    }
    
    /// Update wallet balance from blockchain UTXO set
    pub async fn update_balance(&mut self, address: &str) -> Result<u64> {
        if let Some(tx_manager) = &self.transaction_manager {
            let tx_manager = tx_manager.read().await;
            let balance = tx_manager.get_balance(address);
            
            // Update wallet balance
            if let Some(wallet_id) = self.address_to_id.get(address) {
                if let Some(wallet) = self.wallets.get_mut(wallet_id) {
                    wallet.balance = balance;
                }
            }
            
            Ok(balance)
        } else {
            // Fallback to stored balance if no blockchain integration
            if let Some(wallet_id) = self.address_to_id.get(address) {
                if let Some(wallet) = self.wallets.get(wallet_id) {
                    return Ok(wallet.balance);
                }
            }
            Ok(0)
        }
    }

    /// Update all wallet balances from blockchain
    pub async fn update_all_balances(&mut self) -> Result<()> {
        let addresses: Vec<String> = self.address_to_id.keys().cloned().collect();
        
        for address in addresses {
            self.update_balance(&address).await?;
        }
        
        Ok(())
    }
    
    /// Create a real blockchain transaction between wallets
    pub async fn create_transaction(
        &mut self,
        from_address: &str,
        to_address: &str,
        amount: u64,
        message: Option<String>,
    ) -> Result<WalletTransaction> {
        // Get sender wallet
        let sender = self.get_wallet_by_address(from_address)
            .ok_or_else(|| BlockchainError::WalletNotFound(from_address.to_string()))?
            .clone();
            
        // Validate recipient address format
        if !to_address.starts_with("edu1q") {
            return Err(BlockchainError::InvalidAddress(to_address.to_string()));
        }
        
        // Create blockchain transaction if manager is available
        let blockchain_tx = if let Some(tx_manager) = &self.transaction_manager {
            let mut tx_manager = tx_manager.write().await;
            
            // Update sender balance from blockchain first
            let current_balance = tx_manager.get_balance(&sender.address);
            
            // Verify sufficient balance before creating transaction
            if current_balance < amount {
                return Err(BlockchainError::InsufficientFunds(
                    format!("Balance {} insufficient for amount {}", current_balance, amount)
                ));
            }
            
            // Create the actual blockchain transaction
            let blockchain_tx = tx_manager.create_transaction(
                &sender,
                to_address,
                amount,
                Some(1000), // Default fee rate
            )?;
            
            // Calculate actual fee from the transaction
            let actual_fee = tx_manager.get_utxo_set().calculate_fee(&blockchain_tx)?;
            
            // Add to pending transactions
            let pending_id = tx_manager.add_pending_transaction(blockchain_tx.clone())?;
            
            Some((blockchain_tx, actual_fee, pending_id))
        } else {
            None
        };
        
        // Create wallet transaction record
        let (fee, blockchain_tx_id) = if let Some((tx, fee, _)) = blockchain_tx.as_ref() {
            let tx_hash = hex::encode(tx.get_hash()?);
            (fee.clone(), Some(tx_hash))
        } else {
            (self.calculate_transaction_fee(amount), None)
        };
        
        let wallet_transaction = WalletTransaction {
            id: Uuid::new_v4(),
            from_address: from_address.to_string(),
            to_address: to_address.to_string(),
            amount,
            fee,
            message,
            created_at: Utc::now(),
            blockchain_tx_id,
            confirmations: 0,
            status: if blockchain_tx.is_some() {
                TransactionStatus::Broadcasting
            } else {
                TransactionStatus::Pending
            },
        };
        
        self.transactions.push(wallet_transaction.clone());
        
        // Update balances after transaction
        self.update_balance(from_address).await?;
        
        Ok(wallet_transaction)
    }

    /// Create a transaction without blockchain integration (fallback)
    pub fn create_simple_transaction(
        &mut self,
        from_address: &str,
        to_address: &str,
        amount: u64,
        message: Option<String>,
    ) -> Result<WalletTransaction> {
        // Validate sender has sufficient balance
        let sender = self.get_wallet_by_address(from_address)
            .ok_or_else(|| BlockchainError::WalletNotFound(from_address.to_string()))?;
            
        let fee = self.calculate_transaction_fee(amount);
        if sender.balance < amount + fee {
            return Err(BlockchainError::InsufficientFunds(
                format!("Need {} satoshis, have {}", amount + fee, sender.balance)
            ));
        }
        
        // Validate recipient address exists or is valid format
        if !to_address.starts_with("edu1q") {
            return Err(BlockchainError::InvalidAddress(to_address.to_string()));
        }
        
        let transaction = WalletTransaction {
            id: Uuid::new_v4(),
            from_address: from_address.to_string(),
            to_address: to_address.to_string(),
            amount,
            fee,
            message,
            created_at: Utc::now(),
            blockchain_tx_id: None,
            confirmations: 0,
            status: TransactionStatus::Pending,
        };
        
        self.transactions.push(transaction.clone());
        Ok(transaction)
    }
    
    /// Calculate transaction fee (simple fee calculation)
    pub fn calculate_transaction_fee(&self, amount: u64) -> u64 {
        // Simple fee: 0.1% of amount, minimum 1000 satoshis (0.00001 EDU)
        let percentage_fee = amount / 1000;
        std::cmp::max(percentage_fee, 1000)
    }
    
    /// Get transaction history for a wallet
    pub fn get_wallet_transactions(&self, address: &str) -> Vec<&WalletTransaction> {
        self.transactions.iter()
            .filter(|tx| tx.from_address == address || tx.to_address == address)
            .collect()
    }
    
    /// Get all pending transactions
    pub fn get_pending_transactions(&self) -> Vec<&WalletTransaction> {
        self.transactions.iter()
            .filter(|tx| matches!(tx.status, TransactionStatus::Pending))
            .collect()
    }
    
    /// Update transaction status
    pub fn update_transaction_status(&mut self, tx_id: &Uuid, status: TransactionStatus, blockchain_tx_id: Option<String>) {
        if let Some(tx) = self.transactions.iter_mut().find(|tx| tx.id == *tx_id) {
            tx.status = status;
            if let Some(btx_id) = blockchain_tx_id {
                tx.blockchain_tx_id = Some(btx_id);
            }
        }
    }
}

impl Default for WalletManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_wallet_creation() {
        let wallet = Wallet::new("Test Wallet".to_string()).unwrap();
        assert_eq!(wallet.name, "Test Wallet");
        assert!(wallet.address.starts_with("edu1q"));
        assert_eq!(wallet.balance, 0);
    }
    
    #[test]
    fn test_payment_request_qr() {
        let wallet = Wallet::new("Test".to_string()).unwrap();
        let request = wallet.create_payment_request(Some(1.5), Some("Coffee".to_string()));
        let qr_string = request.to_qr_string();
        
        assert!(qr_string.contains(&wallet.address));
        assert!(qr_string.contains("amount=1.5"));
        assert!(qr_string.contains("message=Coffee"));
        
        let parsed = PaymentRequest::from_qr_string(&qr_string).unwrap();
        assert_eq!(parsed.address, wallet.address);
        assert_eq!(parsed.amount, Some(Wallet::edu_to_satoshis(1.5)));
    }
    
    #[test]
    fn test_wallet_manager() {
        let mut manager = WalletManager::new();
        
        let wallet1 = manager.create_wallet("Alice".to_string()).unwrap();
        let wallet1_address = wallet1.address.clone();
        let wallet2 = manager.create_wallet("Bob".to_string()).unwrap();
        let wallet2_address = wallet2.address.clone();
        
        assert_eq!(manager.list_wallets().len(), 2);
        assert_eq!(manager.get_wallet_by_address(&wallet1_address).unwrap().name, "Alice");
        assert_eq!(manager.get_wallet_by_address(&wallet2_address).unwrap().name, "Bob");
    }
}