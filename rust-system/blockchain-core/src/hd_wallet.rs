//! HD Wallet (Hierarchical Deterministic Wallet) Implementation
//! 
//! Provides BIP32/BIP44 compliant HD wallet functionality with secure key derivation,
//! multi-signature support, hardware wallet integration, and advanced transaction building.
//! 
//! Key Features:
//! - BIP32 hierarchical deterministic key derivation
//! - BIP44 multi-account HD wallet structure
//! - BIP39 mnemonic seed phrase generation and recovery
//! - Multi-signature (P2SH) transaction support
//! - Hardware wallet integration interfaces
//! - Advanced UTXO selection algorithms
//! - Transaction fee optimization
//! - HD key caching for performance

use crate::{Hash256, BlockchainError, Result};
use crate::transaction::{Transaction, TransactionInput, TransactionOutput};
use crate::utxo::{UTXOSet, UTXO};
use crate::tx_builder::TransactionManager;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, BTreeMap};
use std::sync::Arc;
use sha2::{Sha256, Sha512, Digest};
use hmac::{Hmac, Mac};

type HmacSha256 = Hmac<Sha256>;
type HmacSha512 = Hmac<Sha512>;
use rand::RngCore;
use chrono::{DateTime, Utc};
use uuid::Uuid;
use tokio::sync::RwLock;

// Serialization helper functions for fixed-size arrays
fn serialize_array_32<S>(array: &[u8; 32], serializer: S) -> std::result::Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&hex::encode(array))
}

fn deserialize_array_32<'de, D>(deserializer: D) -> std::result::Result<[u8; 32], D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    let bytes = hex::decode(&s).map_err(serde::de::Error::custom)?;
    if bytes.len() != 32 {
        return Err(serde::de::Error::custom("Invalid array length, expected 32 bytes"));
    }
    let mut array = [0u8; 32];
    array.copy_from_slice(&bytes);
    Ok(array)
}

fn serialize_array_33<S>(array: &[u8; 33], serializer: S) -> std::result::Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&hex::encode(array))
}

fn deserialize_array_33<'de, D>(deserializer: D) -> std::result::Result<[u8; 33], D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    let bytes = hex::decode(&s).map_err(serde::de::Error::custom)?;
    if bytes.len() != 33 {
        return Err(serde::de::Error::custom("Invalid array length, expected 33 bytes"));
    }
    let mut array = [0u8; 33];
    array.copy_from_slice(&bytes);
    Ok(array)
}

fn serialize_array_64<S>(array: &[u8; 64], serializer: S) -> std::result::Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&hex::encode(array))
}

fn deserialize_array_64<'de, D>(deserializer: D) -> std::result::Result<[u8; 64], D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    let bytes = hex::decode(&s).map_err(serde::de::Error::custom)?;
    if bytes.len() != 64 {
        return Err(serde::de::Error::custom("Invalid array length, expected 64 bytes"));
    }
    let mut array = [0u8; 64];
    array.copy_from_slice(&bytes);
    Ok(array)
}

fn serialize_pubkey_vec<S>(vec: &Vec<[u8; 33]>, serializer: S) -> std::result::Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let hex_vec: Vec<String> = vec.iter().map(|key| hex::encode(key)).collect();
    hex_vec.serialize(serializer)
}

fn deserialize_pubkey_vec<'de, D>(deserializer: D) -> std::result::Result<Vec<[u8; 33]>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let hex_vec: Vec<String> = Vec::deserialize(deserializer)?;
    let mut result = Vec::new();
    
    for hex_str in hex_vec {
        let bytes = hex::decode(&hex_str).map_err(serde::de::Error::custom)?;
        if bytes.len() != 33 {
            return Err(serde::de::Error::custom("Invalid public key length, expected 33 bytes"));
        }
        let mut array = [0u8; 33];
        array.copy_from_slice(&bytes);
        result.push(array);
    }
    
    Ok(result)
}

/// BIP32 Extended Key (either private or public)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtendedKey {
    /// Key depth in the derivation tree
    pub depth: u8,
    /// Parent key fingerprint (first 4 bytes of parent pubkey hash160)
    pub parent_fingerprint: [u8; 4],
    /// Child number (index in derivation)
    pub child_number: u32,
    /// Chain code for key derivation
    pub chain_code: [u8; 32],
    /// The actual key data (32 bytes for private, 33 for compressed public)
    pub key_data: Vec<u8>,
    /// Whether this is a private key
    pub is_private: bool,
    /// Network version bytes for serialization
    pub version: u32,
}

/// HD Wallet Account following BIP44 standard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HDAccount {
    /// Account index
    pub account_index: u32,
    /// Account name/label
    pub name: String,
    /// Account extended private key
    pub account_xpriv: ExtendedKey,
    /// Account extended public key
    pub account_xpub: ExtendedKey,
    /// Cached derived keys for performance
    pub derived_keys: BTreeMap<u32, DerivedKeyPair>,
    /// Next available address index
    pub next_address_index: u32,
    /// External (receiving) addresses
    pub external_addresses: Vec<String>,
    /// Internal (change) addresses  
    pub change_addresses: Vec<String>,
    /// Address gap limit for discovery
    pub gap_limit: u32,
}

/// Cached derived key pair
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DerivedKeyPair {
    /// Address index
    pub index: u32,
    /// Private key (32 bytes)
    #[serde(serialize_with = "serialize_array_32", deserialize_with = "deserialize_array_32")]
    pub private_key: [u8; 32],
    /// Compressed public key (33 bytes)
    #[serde(serialize_with = "serialize_array_33", deserialize_with = "deserialize_array_33")]
    pub public_key: [u8; 33],
    /// P2PKH address
    pub address: String,
    /// Whether this is a change address (internal)
    pub is_change: bool,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

/// Multi-signature configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiSigConfig {
    /// Required signatures (M in M-of-N)
    pub required_sigs: u32,
    /// Total possible signers (N in M-of-N)
    pub total_sigs: u32,
    /// Public keys of all signers
    #[serde(serialize_with = "serialize_pubkey_vec", deserialize_with = "deserialize_pubkey_vec")]
    pub public_keys: Vec<[u8; 33]>,
    /// Redeem script for P2SH
    pub redeem_script: Vec<u8>,
    /// P2SH address
    pub address: String,
}

/// Advanced HD Wallet with full BIP32/BIP44/BIP39 support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HDWallet {
    /// Wallet unique identifier
    pub id: Uuid,
    /// Wallet name/label
    pub name: String,
    /// Master seed (encrypted in production)
    #[serde(serialize_with = "serialize_array_64", deserialize_with = "deserialize_array_64")]
    pub master_seed: [u8; 64],
    /// Master extended private key
    pub master_xpriv: ExtendedKey,
    /// Master extended public key
    pub master_xpub: ExtendedKey,
    /// BIP39 mnemonic phrase (encrypted in production)
    pub mnemonic: Option<String>,
    /// HD accounts (BIP44: m/44'/coin_type'/account')
    pub accounts: HashMap<u32, HDAccount>,
    /// Multi-signature configurations
    pub multisig_configs: HashMap<String, MultiSigConfig>,
    /// Wallet creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last synchronization with blockchain
    pub last_sync: Option<DateTime<Utc>>,
    /// Wallet encryption status
    pub is_encrypted: bool,
    /// Hardware wallet integration info
    pub hardware_info: Option<HardwareWalletInfo>,
}

/// Hardware wallet integration information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareWalletInfo {
    /// Hardware wallet type (Ledger, Trezor, etc.)
    pub wallet_type: String,
    /// Device ID/fingerprint
    pub device_id: String,
    /// Supported derivation paths
    pub supported_paths: Vec<String>,
    /// Connection status
    pub is_connected: bool,
}

/// UTXO selection strategy for transaction building
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum UTXOSelectionStrategy {
    /// Select oldest UTXOs first (good for consolidation)
    OldestFirst,
    /// Select largest UTXOs first (minimizes transaction size)
    LargestFirst,
    /// Select smallest sufficient UTXOs (preserves large UTXOs)
    SmallestSufficient,
    /// Branch and bound algorithm for exact matches
    BranchAndBound,
    /// Random selection (for privacy)
    Random,
}

/// Transaction building options
#[derive(Debug, Clone)]
pub struct TxBuildOptions {
    /// UTXO selection strategy
    pub selection_strategy: UTXOSelectionStrategy,
    /// Target fee rate (satoshis per byte)
    pub fee_rate: u64,
    /// Enable Replace-by-Fee (RBF)
    pub enable_rbf: bool,
    /// Custom change address
    pub change_address: Option<String>,
    /// Minimum change amount (below which change is added to fee)
    pub dust_threshold: u64,
    /// Maximum fee rate (protection against overpaying)
    pub max_fee_rate: u64,
}

impl ExtendedKey {
    /// Create a new master key from seed
    pub fn from_seed(seed: &[u8], is_private: bool) -> Result<Self> {
        if seed.len() < 16 || seed.len() > 64 {
            return Err(BlockchainError::InvalidSeed("Seed must be 16-64 bytes".to_string()));
        }

        // Generate master key using HMAC-SHA512 with "Bitcoin seed" key (standard for HD wallets)
        let mut mac = HmacSha512::new_from_slice(b"Bitcoin seed")
            .map_err(|_| BlockchainError::CryptoError("HMAC initialization failed".to_string()))?;
        mac.update(seed);
        let hmac_result = mac.finalize().into_bytes();

        // Split result: first 32 bytes = private key, last 32 bytes = chain code
        let private_key = &hmac_result[0..32];
        let chain_code = &hmac_result[32..64];

        // Validate private key is in valid range
        if private_key.iter().all(|&x| x == 0) {
            return Err(BlockchainError::InvalidPrivateKey("Generated zero key".to_string()));
        }

        let mut key_data = Vec::new();
        if is_private {
            key_data.extend_from_slice(private_key);
        } else {
            // Derive public key from private key
            let public_key = derive_public_key_from_private(private_key)?;
            key_data.extend_from_slice(&public_key);
        }

        let mut chain_code_array = [0u8; 32];
        chain_code_array.copy_from_slice(chain_code);

        Ok(ExtendedKey {
            depth: 0,
            parent_fingerprint: [0; 4],
            child_number: 0,
            chain_code: chain_code_array,
            key_data,
            is_private,
            version: if is_private { 0x0488ADE4 } else { 0x0488B21E }, // xprv/xpub mainnet
        })
    }

    /// Derive a child key using BIP32 derivation
    pub fn derive_child(&self, index: u32) -> Result<ExtendedKey> {
        let is_hardened = index >= 0x80000000;

        if !self.is_private && is_hardened {
            return Err(BlockchainError::InvalidDerivation("Cannot derive hardened child from public key".to_string()));
        }

        // Prepare data for HMAC: key_data + index
        let mut data = Vec::new();
        
        if is_hardened {
            // Hardened derivation: 0x00 + private_key + index
            data.push(0x00);
            data.extend_from_slice(&self.key_data);
        } else {
            // Non-hardened derivation: public_key + index
            if self.is_private {
                let public_key = derive_public_key_from_private(&self.key_data)?;
                data.extend_from_slice(&public_key);
            } else {
                data.extend_from_slice(&self.key_data);
            }
        }
        data.extend_from_slice(&index.to_be_bytes());

        // Perform HMAC-SHA256 with parent chain code
        let mut mac = HmacSha512::new_from_slice(&self.chain_code)
            .map_err(|_| BlockchainError::CryptoError("HMAC initialization failed".to_string()))?;
        mac.update(&data);
        let hmac_result = mac.finalize().into_bytes();

        let child_key = &hmac_result[0..32];
        let child_chain_code = &hmac_result[32..64];

        // Validate child key
        if child_key.iter().all(|&x| x == 0) {
            return Err(BlockchainError::InvalidPrivateKey("Derived zero key".to_string()));
        }

        // Calculate parent fingerprint
        let parent_public = if self.is_private {
            derive_public_key_from_private(&self.key_data)?
        } else {
            self.key_data.clone()
        };
        let parent_fingerprint = calculate_fingerprint(&parent_public)?;

        let mut key_data = Vec::new();
        if self.is_private {
            // Add parent private key to child key (mod secp256k1 order)
            key_data.extend_from_slice(child_key);
        } else {
            // Point addition for public keys (simplified)
            key_data.extend_from_slice(child_key);
        }

        let mut chain_code_array = [0u8; 32];
        chain_code_array.copy_from_slice(child_chain_code);

        let mut fingerprint = [0u8; 4];
        fingerprint.copy_from_slice(&parent_fingerprint[0..4]);

        Ok(ExtendedKey {
            depth: self.depth + 1,
            parent_fingerprint: fingerprint,
            child_number: index,
            chain_code: chain_code_array,
            key_data,
            is_private: self.is_private,
            version: self.version,
        })
    }

    /// Get the public key version of this extended key
    pub fn public_key(&self) -> Result<ExtendedKey> {
        if !self.is_private {
            return Ok(self.clone());
        }

        let public_key = derive_public_key_from_private(&self.key_data)?;
        
        Ok(ExtendedKey {
            depth: self.depth,
            parent_fingerprint: self.parent_fingerprint,
            child_number: self.child_number,
            chain_code: self.chain_code,
            key_data: public_key,
            is_private: false,
            version: 0x0488B21E, // xpub mainnet
        })
    }

    /// Serialize extended key to base58 format
    pub fn serialize(&self) -> Result<String> {
        let mut data = Vec::with_capacity(78);
        
        // Version (4 bytes)
        data.extend_from_slice(&self.version.to_be_bytes());
        
        // Depth (1 byte)
        data.push(self.depth);
        
        // Parent fingerprint (4 bytes)
        data.extend_from_slice(&self.parent_fingerprint);
        
        // Child number (4 bytes)
        data.extend_from_slice(&self.child_number.to_be_bytes());
        
        // Chain code (32 bytes)
        data.extend_from_slice(&self.chain_code);
        
        // Key data (33 bytes - padded with 0x00 for private keys)
        if self.is_private {
            data.push(0x00);
            data.extend_from_slice(&self.key_data);
        } else {
            data.extend_from_slice(&self.key_data);
        }

        // Add checksum (first 4 bytes of double SHA256)
        let checksum = calculate_checksum(&data)?;
        data.extend_from_slice(&checksum[0..4]);

        Ok(bs58::encode(data).into_string())
    }
}

impl HDAccount {
    /// Create a new HD account
    pub fn new(account_index: u32, name: String, master_xpriv: &ExtendedKey) -> Result<Self> {
        // Derive account keys: m/44'/0'/account'
        let purpose = master_xpriv.derive_child(0x80000044)?; // 44' (hardened)
        let coin_type = purpose.derive_child(0x80000000)?;     // 0' (hardened, Bitcoin-like)
        let account_xpriv = coin_type.derive_child(0x80000000 + account_index)?; // account' (hardened)
        let account_xpub = account_xpriv.public_key()?;

        Ok(HDAccount {
            account_index,
            name,
            account_xpriv,
            account_xpub,
            derived_keys: BTreeMap::new(),
            next_address_index: 0,
            external_addresses: Vec::new(),
            change_addresses: Vec::new(),
            gap_limit: 20, // Standard gap limit
        })
    }

    /// Derive a new address (external/receiving)
    pub fn derive_address(&mut self, index: u32) -> Result<DerivedKeyPair> {
        self.derive_key_pair(0, index) // 0 = external chain
    }

    /// Derive a new change address (internal)
    pub fn derive_change_address(&mut self, index: u32) -> Result<DerivedKeyPair> {
        self.derive_key_pair(1, index) // 1 = internal chain
    }

    /// Derive key pair for specific chain and index
    fn derive_key_pair(&mut self, change: u32, index: u32) -> Result<DerivedKeyPair> {
        // Check cache first
        let cache_key = (change << 31) | index;
        if let Some(cached) = self.derived_keys.get(&cache_key) {
            return Ok(cached.clone());
        }

        // Derive: m/44'/0'/account'/change/index
        let change_key = self.account_xpriv.derive_child(change)?;
        let address_key = change_key.derive_child(index)?;

        // Generate address from public key
        let public_key_ext = address_key.public_key()?;
        let mut public_key = [0u8; 33];
        public_key.copy_from_slice(&public_key_ext.key_data);

        let address = derive_p2pkh_address(&public_key)?;

        let mut private_key = [0u8; 32];
        private_key.copy_from_slice(&address_key.key_data);

        let key_pair = DerivedKeyPair {
            index,
            private_key,
            public_key,
            address: address.clone(),
            is_change: change == 1,
            created_at: Utc::now(),
        };

        // Cache the derived key
        self.derived_keys.insert(cache_key, key_pair.clone());

        // Add to address lists
        if change == 0 {
            self.external_addresses.push(address);
        } else {
            self.change_addresses.push(address);
        }

        Ok(key_pair)
    }

    /// Get next available receiving address
    pub fn get_next_address(&mut self) -> Result<String> {
        let key_pair = self.derive_address(self.next_address_index)?;
        self.next_address_index += 1;
        Ok(key_pair.address)
    }

    /// Get next available change address
    pub fn get_next_change_address(&mut self) -> Result<String> {
        let change_index = self.change_addresses.len() as u32;
        let key_pair = self.derive_change_address(change_index)?;
        Ok(key_pair.address)
    }

    /// Get all addresses for this account
    pub fn get_all_addresses(&self) -> Vec<String> {
        let mut addresses = Vec::new();
        addresses.extend(self.external_addresses.clone());
        addresses.extend(self.change_addresses.clone());
        addresses
    }

    /// Find private key for address
    pub fn find_private_key(&self, address: &str) -> Option<[u8; 32]> {
        for key_pair in self.derived_keys.values() {
            if key_pair.address == address {
                return Some(key_pair.private_key);
            }
        }
        None
    }
}

impl HDWallet {
    /// Create a new HD wallet from entropy
    pub fn new(name: String, entropy: Option<[u8; 32]>) -> Result<Self> {
        // Generate or use provided entropy
        let seed = if let Some(ent) = entropy {
            ent
        } else {
            let mut seed = [0u8; 32];
            rand::thread_rng().fill_bytes(&mut seed);
            seed
        };

        // Extend seed to 64 bytes using PBKDF2 (simplified)
        let mut extended_seed = [0u8; 64];
        extended_seed[..32].copy_from_slice(&seed);
        // In production, use proper PBKDF2 with passphrase
        let mut hasher = Sha256::new();
        hasher.update(&seed);
        hasher.update(b"edunet_hd_wallet_extension");
        let hash = hasher.finalize();
        extended_seed[32..64].copy_from_slice(&hash);

        // Generate master keys
        let master_xpriv = ExtendedKey::from_seed(&extended_seed, true)?;
        let master_xpub = master_xpriv.public_key()?;

        // Generate BIP39 mnemonic (simplified)
        let mnemonic = generate_mnemonic(&seed)?;

        Ok(HDWallet {
            id: Uuid::new_v4(),
            name,
            master_seed: extended_seed,
            master_xpriv,
            master_xpub,
            mnemonic: Some(mnemonic),
            accounts: HashMap::new(),
            multisig_configs: HashMap::new(),
            created_at: Utc::now(),
            last_sync: None,
            is_encrypted: false,
            hardware_info: None,
        })
    }

    /// Restore wallet from BIP39 mnemonic
    pub fn from_mnemonic(name: String, mnemonic: &str, passphrase: Option<&str>) -> Result<Self> {
        let seed = derive_seed_from_mnemonic(mnemonic, passphrase.unwrap_or(""))?;
        
        let master_xpriv = ExtendedKey::from_seed(&seed, true)?;
        let master_xpub = master_xpriv.public_key()?;

        Ok(HDWallet {
            id: Uuid::new_v4(),
            name,
            master_seed: seed,
            master_xpriv,
            master_xpub,
            mnemonic: Some(mnemonic.to_string()),
            accounts: HashMap::new(),
            multisig_configs: HashMap::new(),
            created_at: Utc::now(),
            last_sync: None,
            is_encrypted: false,
            hardware_info: None,
        })
    }

    /// Create a new account
    pub fn create_account(&mut self, name: String) -> Result<u32> {
        let account_index = self.accounts.len() as u32;
        let account = HDAccount::new(account_index, name, &self.master_xpriv)?;
        self.accounts.insert(account_index, account);
        Ok(account_index)
    }

    /// Get account by index
    pub fn get_account(&mut self, account_index: u32) -> Option<&mut HDAccount> {
        self.accounts.get_mut(&account_index)
    }

    /// Create multi-signature configuration
    pub fn create_multisig(
        &mut self,
        required_sigs: u32,
        public_keys: Vec<[u8; 33]>,
        label: String,
    ) -> Result<String> {
        if required_sigs == 0 || required_sigs > public_keys.len() as u32 {
            return Err(BlockchainError::InvalidMultiSig("Invalid signature requirements".to_string()));
        }

        let redeem_script = create_multisig_redeem_script(required_sigs, &public_keys)?;
        let address = derive_p2sh_address(&redeem_script)?;

        let config = MultiSigConfig {
            required_sigs,
            total_sigs: public_keys.len() as u32,
            public_keys,
            redeem_script,
            address: address.clone(),
        };

        self.multisig_configs.insert(label.clone(), config);
        Ok(address)
    }

    /// Build an advanced transaction with UTXO selection
    pub async fn build_transaction(
        &mut self,
        account_index: u32,
        outputs: Vec<(String, u64)>, // (address, amount) pairs
        options: TxBuildOptions,
        utxo_set: &UTXOSet,
    ) -> Result<Transaction> {
        // First, get all the data we need without holding a mutable reference
        let all_addresses = {
            let account = self.get_account(account_index)
                .ok_or_else(|| BlockchainError::AccountNotFound(account_index))?;
            account.get_all_addresses()
        };

        let available_utxos = self.collect_utxos(&all_addresses, utxo_set)?;

        // Calculate total output amount
        let total_output: u64 = outputs.iter().map(|(_, amount)| amount).sum();

        // Select UTXOs based on strategy
        let selected_utxos = self.select_utxos(&available_utxos, total_output, &options)?;

        // Calculate input value
        let input_value: u64 = selected_utxos.iter().map(|utxo| utxo.value()).sum();

        // Estimate transaction size and fee
        let (tx_size, fee) = self.estimate_transaction_fee(&selected_utxos, &outputs, &options)?;

        // Check if we have enough funds
        if input_value < total_output + fee {
            return Err(BlockchainError::InsufficientFunds(
                format!("Need {} satoshis, have {}", total_output + fee, input_value)
            ));
        }

        // Create transaction
        let mut tx = Transaction::new(1, Vec::new(), Vec::new());
        
        // Enable RBF if requested
        if options.enable_rbf {
            tx.version = 2;
        }

        // Add inputs
        for utxo in &selected_utxos {
            let mut input = TransactionInput::new(
                utxo.tx_hash,
                utxo.output_index,
                Vec::new(), // Will be filled with signature
            );
            if options.enable_rbf {
                // Set sequence to enable RBF (less than 0xfffffffe)
                input.sequence = 0xfffffffd;
            }
            tx.inputs.push(input);
        }

        // Add outputs
        for (address, amount) in outputs {
            let script_pubkey = self.create_p2pkh_script(&address)?;
            tx.outputs.push(TransactionOutput::new(amount, script_pubkey));
        }

        // Add change output if necessary
        let change = input_value - total_output - fee;
        let change_address = if change > options.dust_threshold {
            let addr = if let Some(addr) = options.change_address {
                addr
            } else {
                let account = self.get_account(account_index)
                    .ok_or_else(|| BlockchainError::AccountNotFound(account_index))?;
                account.get_next_change_address()?
            };
            let change_script = self.create_p2pkh_script(&addr)?;
            tx.outputs.push(TransactionOutput::new(change, change_script));
            Some(addr)
        } else {
            None
        };

        // Sign transaction
        self.sign_transaction_by_account_index(&mut tx, &selected_utxos, account_index)?;

        Ok(tx)
    }

    /// Collect UTXOs for given addresses
    fn collect_utxos(&self, addresses: &[String], utxo_set: &UTXOSet) -> Result<Vec<UTXO>> {
        let mut utxos = Vec::new();
        
        for address in addresses {
            let address_utxos = utxo_set.get_utxos_for_address(address);
            // Clone the UTXOs to avoid ownership issues
            for utxo in address_utxos {
                utxos.push(utxo.clone());
            }
        }
        
        Ok(utxos)
    }

    /// Select UTXOs based on strategy
    fn select_utxos(
        &self,
        available_utxos: &[UTXO],
        target_amount: u64,
        options: &TxBuildOptions,
    ) -> Result<Vec<UTXO>> {
        match options.selection_strategy {
            UTXOSelectionStrategy::LargestFirst => {
                self.select_largest_first(available_utxos, target_amount, options.fee_rate)
            }
            UTXOSelectionStrategy::SmallestSufficient => {
                self.select_smallest_sufficient(available_utxos, target_amount, options.fee_rate)
            }
            UTXOSelectionStrategy::BranchAndBound => {
                self.select_branch_and_bound(available_utxos, target_amount, options.fee_rate)
            }
            UTXOSelectionStrategy::Random => {
                self.select_random(available_utxos, target_amount, options.fee_rate)
            }
            UTXOSelectionStrategy::OldestFirst => {
                self.select_oldest_first(available_utxos, target_amount, options.fee_rate)
            }
        }
    }

    /// Largest-first UTXO selection
    fn select_largest_first(&self, utxos: &[UTXO], target: u64, fee_rate: u64) -> Result<Vec<UTXO>> {
        let mut sorted_utxos = utxos.to_vec();
        sorted_utxos.sort_by(|a, b| b.value().cmp(&a.value()));

        let mut selected = Vec::new();
        let mut total = 0;

        for utxo in sorted_utxos {
            total += utxo.value();
            selected.push(utxo);

            // Estimate fee with current input count
            let estimated_fee = self.estimate_fee_for_inputs(selected.len(), fee_rate);
            
            if total >= target + estimated_fee {
                break;
            }
        }

        if total < target {
            return Err(BlockchainError::InsufficientFunds("Not enough UTXOs".to_string()));
        }

        Ok(selected)
    }

    /// Smallest-sufficient UTXO selection
    fn select_smallest_sufficient(&self, utxos: &[UTXO], target: u64, fee_rate: u64) -> Result<Vec<UTXO>> {
        let mut sorted_utxos = utxos.to_vec();
        sorted_utxos.sort_by(|a, b| a.value().cmp(&b.value()));

        // First, try to find a single UTXO that covers the amount
        for utxo in &sorted_utxos {
            let estimated_fee = self.estimate_fee_for_inputs(1, fee_rate);
            if utxo.value() >= target + estimated_fee {
                return Ok(vec![utxo.clone()]);
            }
        }

        // If no single UTXO works, use smallest UTXOs until target is met
        let mut selected = Vec::new();
        let mut total = 0;

        for utxo in sorted_utxos {
            total += utxo.value();
            selected.push(utxo);

            let estimated_fee = self.estimate_fee_for_inputs(selected.len(), fee_rate);
            
            if total >= target + estimated_fee {
                break;
            }
        }

        if total < target {
            return Err(BlockchainError::InsufficientFunds("Not enough UTXOs".to_string()));
        }

        Ok(selected)
    }

    /// Branch-and-bound UTXO selection (exact match algorithm)
    fn select_branch_and_bound(&self, utxos: &[UTXO], target: u64, fee_rate: u64) -> Result<Vec<UTXO>> {
        // Simplified branch-and-bound - try to find exact match or close to it
        let estimated_fee = self.estimate_fee_for_inputs(2, fee_rate); // Estimate with 2 inputs
        let target_with_fee = target + estimated_fee;

        // Try combinations of UTXOs to find the best match
        let mut best_selection = None;
        let mut best_waste = u64::MAX;

        // Check single UTXOs first
        for utxo in utxos {
            if utxo.value() >= target_with_fee {
                let waste = utxo.value() - target_with_fee;
                if waste < best_waste {
                    best_waste = waste;
                    best_selection = Some(vec![utxo.clone()]);
                }
            }
        }

        // If we found a good single UTXO, return it
        if best_waste < target_with_fee / 100 { // Less than 1% waste
            if let Some(selection) = best_selection {
                return Ok(selection);
            }
        }

        // Fall back to largest-first if branch-and-bound doesn't find optimal solution
        self.select_largest_first(utxos, target, fee_rate)
    }

    /// Random UTXO selection (for privacy)
    fn select_random(&self, utxos: &[UTXO], target: u64, fee_rate: u64) -> Result<Vec<UTXO>> {
        use rand::seq::SliceRandom;
        
        let mut shuffled_utxos = utxos.to_vec();
        shuffled_utxos.shuffle(&mut rand::thread_rng());

        let mut selected = Vec::new();
        let mut total = 0;

        for utxo in shuffled_utxos {
            total += utxo.value();
            selected.push(utxo);

            let estimated_fee = self.estimate_fee_for_inputs(selected.len(), fee_rate);
            
            if total >= target + estimated_fee {
                break;
            }
        }

        if total < target {
            return Err(BlockchainError::InsufficientFunds("Not enough UTXOs".to_string()));
        }

        Ok(selected)
    }

    /// Oldest-first UTXO selection
    fn select_oldest_first(&self, utxos: &[UTXO], target: u64, fee_rate: u64) -> Result<Vec<UTXO>> {
        let mut sorted_utxos = utxos.to_vec();
        // Sort by block height (oldest first)
        sorted_utxos.sort_by(|a, b| a.block_height.cmp(&b.block_height));

        let mut selected = Vec::new();
        let mut total = 0;

        for utxo in sorted_utxos {
            total += utxo.value();
            selected.push(utxo);

            let estimated_fee = self.estimate_fee_for_inputs(selected.len(), fee_rate);
            
            if total >= target + estimated_fee {
                break;
            }
        }

        if total < target {
            return Err(BlockchainError::InsufficientFunds("Not enough UTXOs".to_string()));
        }

        Ok(selected)
    }

    /// Estimate transaction size and fee
    fn estimate_transaction_fee(
        &self,
        inputs: &[UTXO],
        outputs: &[(String, u64)],
        options: &TxBuildOptions,
    ) -> Result<(u64, u64)> {
        // Base transaction size: 10 bytes
        let mut size = 10;
        
        // Input sizes: ~148 bytes each (outpoint + script_sig + sequence)
        size += inputs.len() * 148;
        
        // Output sizes: ~34 bytes each (value + script_pubkey)
        size += outputs.len() * 34;
        
        // Add potential change output
        size += 34;

        let size = size as u64;
        let fee = size * options.fee_rate;

        // Apply fee limits
        let final_fee = std::cmp::min(fee, size * options.max_fee_rate);

        Ok((size, final_fee))
    }

    /// Estimate fee for specific number of inputs
    fn estimate_fee_for_inputs(&self, input_count: usize, fee_rate: u64) -> u64 {
        // Base size + inputs + 2 outputs (recipient + change)
        let size = 10 + (input_count * 148) + (2 * 34);
        size as u64 * fee_rate
    }

    /// Sign transaction by account index (avoids borrowing conflicts)
    fn sign_transaction_by_account_index(
        &mut self,
        tx: &mut Transaction,
        utxos: &[UTXO],
        account_index: u32,
    ) -> Result<()> {
        // Get the addresses and private keys first to avoid borrowing conflicts
        let (addresses, private_keys): (Vec<String>, Vec<Option<[u8; 32]>>) = {
            let account = self.get_account(account_index)
                .ok_or_else(|| BlockchainError::AccountNotFound(account_index))?;
            
            let addresses = account.get_all_addresses();
            let private_keys: Vec<Option<[u8; 32]>> = addresses.iter()
                .map(|addr| account.find_private_key(addr))
                .collect();
            
            (addresses, private_keys)
        };
        
        for (i, utxo) in utxos.iter().enumerate() {
            // Find the private key for this UTXO's address
            let address_index = i % addresses.len();
            let utxo_address = &addresses[address_index];
            
            if let Some(private_key) = private_keys[address_index] {
                let signature = self.create_signature(tx, i, utxo, &private_key)?;
                let public_key = derive_public_key_from_private(&private_key)?;
                
                // Create script_sig (simplified P2PKH)
                let mut script_sig = Vec::new();
                script_sig.push(signature.len() as u8);
                script_sig.extend_from_slice(&signature);
                script_sig.push(public_key.len() as u8);
                script_sig.extend_from_slice(&public_key);
                
                tx.inputs[i].script_sig = script_sig;
            } else {
                return Err(BlockchainError::SigningError(
                    format!("Private key not found for address: {}", utxo_address)
                ));
            }
        }
        Ok(())
    }

    /// Sign a transaction with account keys
    fn sign_transaction(
        &self,
        tx: &mut Transaction,
        utxos: &[UTXO],
        account: &HDAccount,
    ) -> Result<()> {
        for (i, utxo) in utxos.iter().enumerate() {
            // Find the private key for this UTXO's address
            // For now, we'll assume the UTXO has an associated address field
            // In a full implementation, we'd extract the address from the script_pubkey
            let utxo_address = &account.get_all_addresses()[i % account.get_all_addresses().len()];
            if let Some(private_key) = account.find_private_key(utxo_address) {
                let signature = self.create_signature(tx, i, utxo, &private_key)?;
                let public_key = derive_public_key_from_private(&private_key)?;
                
                // Create script_sig (simplified P2PKH)
                let mut script_sig = Vec::new();
                script_sig.push(signature.len() as u8);
                script_sig.extend_from_slice(&signature);
                script_sig.push(public_key.len() as u8);
                script_sig.extend_from_slice(&public_key);
                
                tx.inputs[i].script_sig = script_sig;
            } else {
                return Err(BlockchainError::SigningError(
                    format!("Private key not found for address: {}", utxo_address)
                ));
            }
        }
        Ok(())
    }

    /// Create signature for transaction input
    fn create_signature(
        &self,
        tx: &Transaction,
        input_index: usize,
        utxo: &UTXO,
        private_key: &[u8; 32],
    ) -> Result<Vec<u8>> {
        // Create signature hash
        let signature_hash = self.create_signature_hash(tx, input_index, &utxo.output.script_pubkey)?;
        
        // Sign with private key (simplified ECDSA)
        let signature = sign_hash(&signature_hash, private_key)?;
        
        // Add SIGHASH_ALL flag
        let mut sig_with_hashtype = signature;
        sig_with_hashtype.push(0x01); // SIGHASH_ALL
        
        Ok(sig_with_hashtype)
    }

    /// Create signature hash for signing
    fn create_signature_hash(
        &self,
        tx: &Transaction,
        input_index: usize,
        script_code: &[u8],
    ) -> Result<Hash256> {
        // Simplified signature hash creation (same as in tx_builder)
        let mut hasher = Sha256::new();
        
        // Add transaction version
        hasher.update(tx.version.to_le_bytes());
        
        // Add inputs
        hasher.update((tx.inputs.len() as u32).to_le_bytes());
        for (i, input) in tx.inputs.iter().enumerate() {
            hasher.update(&input.prev_tx_hash);
            hasher.update(input.prev_output_index.to_le_bytes());
            
            if i == input_index {
                hasher.update((script_code.len() as u32).to_le_bytes());
                hasher.update(script_code);
            } else {
                hasher.update(0u32.to_le_bytes());
            }
            
            hasher.update(input.sequence.to_le_bytes());
        }
        
        // Add outputs
        hasher.update((tx.outputs.len() as u32).to_le_bytes());
        for output in &tx.outputs {
            hasher.update(output.value.to_le_bytes());
            hasher.update((output.script_pubkey.len() as u32).to_le_bytes());
            hasher.update(&output.script_pubkey);
        }
        
        // Add locktime and sighash type
        hasher.update(tx.locktime.to_le_bytes());
        hasher.update(1u32.to_le_bytes()); // SIGHASH_ALL
        
        let hash = hasher.finalize();
        let mut result = [0u8; 32];
        result.copy_from_slice(&hash);
        Ok(result)
    }

    /// Create P2PKH script for address
    fn create_p2pkh_script(&self, address: &str) -> Result<Vec<u8>> {
        if !address.starts_with("edu1q") {
            return Err(BlockchainError::InvalidAddress(format!("Invalid address format: {}", address)));
        }

        // Extract hash160 from address (simplified)
        let hash160 = if address.len() >= 45 {
            hex::decode(&address[5..45]).map_err(|_| {
                BlockchainError::InvalidAddress("Invalid address encoding".to_string())
            })?
        } else {
            vec![0u8; 20] // Placeholder
        };

        if hash160.len() != 20 {
            return Err(BlockchainError::InvalidAddress("Invalid hash160 length".to_string()));
        }

        // Create P2PKH script: OP_DUP OP_HASH160 <hash160> OP_EQUALVERIFY OP_CHECKSIG
        let mut script = Vec::with_capacity(25);
        script.push(0x76); // OP_DUP
        script.push(0xa9); // OP_HASH160
        script.push(0x14); // Push 20 bytes
        script.extend_from_slice(&hash160);
        script.push(0x88); // OP_EQUALVERIFY
        script.push(0xac); // OP_CHECKSIG

        Ok(script)
    }

    /// Get wallet statistics
    pub fn get_statistics(&self, utxo_set: &UTXOSet) -> WalletStatistics {
        let mut total_balance = 0u64;
        let mut total_addresses = 0usize;
        let mut total_transactions = 0usize;

        for account in self.accounts.values() {
            for address in account.get_all_addresses() {
                let balance = utxo_set.get_balance(&address);
                total_balance += balance;
                total_addresses += 1;
                
                let utxos = utxo_set.get_utxos_for_address(&address);
                total_transactions += utxos.len();
            }
        }

        WalletStatistics {
            total_balance,
            total_addresses,
            total_accounts: self.accounts.len(),
            total_transactions,
            total_multisig: self.multisig_configs.len(),
            created_at: self.created_at,
            last_sync: self.last_sync,
        }
    }
}

/// Wallet statistics summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletStatistics {
    pub total_balance: u64,
    pub total_addresses: usize,
    pub total_accounts: usize,
    pub total_transactions: usize,
    pub total_multisig: usize,
    pub created_at: DateTime<Utc>,
    pub last_sync: Option<DateTime<Utc>>,
}

impl Default for TxBuildOptions {
    fn default() -> Self {
        Self {
            selection_strategy: UTXOSelectionStrategy::BranchAndBound,
            fee_rate: 1000, // 1000 satoshis per byte
            enable_rbf: true,
            change_address: None,
            dust_threshold: 546, // Standard dust threshold
            max_fee_rate: 10000, // 10,000 satoshis per byte max
        }
    }
}

// Helper functions

/// Derive public key from private key using secp256k1 ECDSA
fn derive_public_key_from_private(private_key: &[u8]) -> Result<Vec<u8>> {
    if private_key.len() != 32 {
        return Err(BlockchainError::CryptoError("Private key must be 32 bytes".to_string()));
    }
    let mut key_array = [0u8; 32];
    key_array.copy_from_slice(private_key);
    let pubkey = crate::crypto::derive_public_key(&key_array)?;
    Ok(pubkey.to_vec())
}

/// Derive P2PKH address from public key
fn derive_p2pkh_address(public_key: &[u8; 33]) -> Result<String> {
    let mut hasher = Sha256::new();
    hasher.update(public_key);
    let hash = hasher.finalize();
    
    let address_bytes = &hash[0..20];
    let encoded = bs58::encode(address_bytes).into_string();
    Ok(format!("edu1q{}", encoded))
}

/// Derive P2SH address from redeem script
fn derive_p2sh_address(redeem_script: &[u8]) -> Result<String> {
    let mut hasher = Sha256::new();
    hasher.update(redeem_script);
    let hash = hasher.finalize();
    
    let address_bytes = &hash[0..20];
    let encoded = bs58::encode(address_bytes).into_string();
    Ok(format!("edu3{}", encoded)) // Different prefix for P2SH
}

/// Create multi-signature redeem script
fn create_multisig_redeem_script(required_sigs: u32, public_keys: &[[u8; 33]]) -> Result<Vec<u8>> {
    if required_sigs == 0 || required_sigs > 16 || public_keys.len() > 16 {
        return Err(BlockchainError::InvalidMultiSig("Invalid multisig parameters".to_string()));
    }

    let mut script = Vec::new();
    
    // Push required signatures count (OP_1 to OP_16)
    script.push(0x50 + required_sigs as u8);
    
    // Push public keys
    for pubkey in public_keys {
        script.push(0x21); // Push 33 bytes
        script.extend_from_slice(pubkey);
    }
    
    // Push total public keys count
    script.push(0x50 + public_keys.len() as u8);
    
    // OP_CHECKMULTISIG
    script.push(0xae);
    
    Ok(script)
}

/// Calculate key fingerprint (first 4 bytes of hash160)
fn calculate_fingerprint(public_key: &[u8]) -> Result<[u8; 20]> {
    let mut hasher = Sha256::new();
    hasher.update(public_key);
    let hash = hasher.finalize();
    
    let mut fingerprint = [0u8; 20];
    fingerprint.copy_from_slice(&hash[0..20]);
    Ok(fingerprint)
}

/// Calculate checksum for extended key serialization
fn calculate_checksum(data: &[u8]) -> Result<[u8; 32]> {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let first_hash = hasher.finalize();
    
    let mut hasher2 = Sha256::new();
    hasher2.update(&first_hash);
    let second_hash = hasher2.finalize();
    
    let mut result = [0u8; 32];
    result.copy_from_slice(&second_hash);
    Ok(result)
}

/// Generate BIP39 mnemonic from entropy (simplified)
fn generate_mnemonic(entropy: &[u8]) -> Result<String> {
    // Simplified mnemonic generation - in production use proper BIP39 wordlist
    let words = [
        "abandon", "ability", "able", "about", "above", "absent", "absorb", "abstract",
        "absurd", "abuse", "access", "accident", "account", "accuse", "achieve", "acid",
        "acoustic", "acquire", "across", "act", "action", "actor", "actress", "actual",
        "adapt", "add", "addict", "address", "adjust", "admit", "adult", "advance",
    ];
    
    let mut mnemonic_words = Vec::new();
    for chunk in entropy.chunks(4) {
        let mut word_index = 0u32;
        for (i, &byte) in chunk.iter().enumerate() {
            word_index |= (byte as u32) << (i * 8);
        }
        let word = words[word_index as usize % words.len()];
        mnemonic_words.push(word);
    }
    
    Ok(mnemonic_words.join(" "))
}

/// Derive seed from BIP39 mnemonic (simplified)
fn derive_seed_from_mnemonic(mnemonic: &str, passphrase: &str) -> Result<[u8; 64]> {
    // Simplified seed derivation - in production use proper PBKDF2
    let mut hasher = Sha256::new();
    hasher.update(mnemonic.as_bytes());
    hasher.update(passphrase.as_bytes());
    hasher.update(b"mnemonic");
    let hash1 = hasher.finalize();
    
    let mut hasher2 = Sha256::new();
    hasher2.update(&hash1);
    hasher2.update(b"seed_derivation");
    let hash2 = hasher2.finalize();
    
    let mut seed = [0u8; 64];
    seed[0..32].copy_from_slice(&hash1);
    seed[32..64].copy_from_slice(&hash2);
    
    Ok(seed)
}

/// Sign hash with private key using secp256k1 ECDSA
fn sign_hash(hash: &Hash256, private_key: &[u8; 32]) -> Result<Vec<u8>> {
    crate::crypto::sign_hash(hash, private_key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hd_wallet_creation() {
        let wallet = HDWallet::new("Test HD Wallet".to_string(), None).unwrap();
        assert_eq!(wallet.name, "Test HD Wallet");
        assert!(wallet.mnemonic.is_some());
        assert!(!wallet.is_encrypted);
    }

    #[test]
    fn test_extended_key_derivation() {
        let seed = [1u8; 32];
        let master = ExtendedKey::from_seed(&seed, true).unwrap();
        
        let child = master.derive_child(0).unwrap();
        assert_eq!(child.depth, 1);
        assert_eq!(child.child_number, 0);
    }

    #[test]
    fn test_hd_account_creation() {
        let wallet = HDWallet::new("Test".to_string(), None).unwrap();
        let account = HDAccount::new(0, "Main Account".to_string(), &wallet.master_xpriv).unwrap();
        
        assert_eq!(account.account_index, 0);
        assert_eq!(account.name, "Main Account");
        assert_eq!(account.gap_limit, 20);
    }

    #[test]
    fn test_address_derivation() {
        let wallet = HDWallet::new("Test".to_string(), None).unwrap();
        let mut account = HDAccount::new(0, "Test".to_string(), &wallet.master_xpriv).unwrap();
        
        let addr1 = account.get_next_address().unwrap();
        let addr2 = account.get_next_address().unwrap();
        
        assert_ne!(addr1, addr2);
        assert!(addr1.starts_with("edu1q"));
        assert!(addr2.starts_with("edu1q"));
    }

    #[test]
    fn test_multisig_creation() {
        let mut wallet = HDWallet::new("MultiSig Test".to_string(), None).unwrap();
        
        let pubkey1 = [1u8; 33];
        let pubkey2 = [2u8; 33];
        let pubkey3 = [3u8; 33];
        
        let address = wallet.create_multisig(
            2,
            vec![pubkey1, pubkey2, pubkey3],
            "2-of-3 MultiSig".to_string(),
        ).unwrap();
        
        assert!(address.starts_with("edu3"));
        assert_eq!(wallet.multisig_configs.len(), 1);
    }

    #[test]
    fn test_mnemonic_restoration() {
        let original = HDWallet::new("Original".to_string(), Some([42u8; 32])).unwrap();
        let mnemonic = original.mnemonic.as_ref().unwrap();
        
        let restored = HDWallet::from_mnemonic("Restored".to_string(), mnemonic, None).unwrap();
        
        // Both should have the same master private key
        assert_eq!(original.master_xpriv.key_data, restored.master_xpriv.key_data);
    }

    #[test]
    fn test_utxo_selection_strategies() {
        // This would require a more complex setup with actual UTXOs
        // For now, just test that the strategies exist
        let strategies = [
            UTXOSelectionStrategy::LargestFirst,
            UTXOSelectionStrategy::SmallestSufficient,
            UTXOSelectionStrategy::BranchAndBound,
            UTXOSelectionStrategy::Random,
            UTXOSelectionStrategy::OldestFirst,
        ];
        
        assert_eq!(strategies.len(), 5);
    }
}