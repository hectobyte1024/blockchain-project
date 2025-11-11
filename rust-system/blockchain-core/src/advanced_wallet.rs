//! Advanced Wallet Manager
//! 
//! Integrates HD wallets with the blockchain infrastructure, providing
//! comprehensive wallet management, transaction creation, and key management.

use crate::{BlockchainError, Result, Hash256};
use crate::hd_wallet::{HDWallet, HDAccount, TxBuildOptions, UTXOSelectionStrategy, WalletStatistics};
use crate::wallet::{Wallet, WalletManager as SimpleWalletManager, WalletTransaction, TransactionStatus};
use crate::transaction::Transaction;
use crate::utxo::UTXOSet;
use crate::tx_builder::{TransactionManager, create_p2pkh_script};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, BTreeMap};
use std::sync::Arc;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use tokio::sync::RwLock;

/// Advanced wallet manager with HD wallet support
#[derive(Debug)]
pub struct AdvancedWalletManager {
    /// HD wallets (production-grade)
    hd_wallets: HashMap<Uuid, HDWallet>,
    /// Simple wallets (legacy/compatibility)
    simple_wallets: SimpleWalletManager,
    /// Transaction manager for blockchain operations
    transaction_manager: Option<Arc<RwLock<TransactionManager>>>,
    /// Wallet metadata and settings
    wallet_metadata: HashMap<Uuid, WalletMetadata>,
    /// Global settings
    settings: WalletManagerSettings,
}

/// Metadata for wallet management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletMetadata {
    /// Wallet ID
    pub wallet_id: Uuid,
    /// Wallet type
    pub wallet_type: WalletType,
    /// Whether wallet is the default
    pub is_default: bool,
    /// Last backup timestamp
    pub last_backup: Option<DateTime<Utc>>,
    /// Encryption status
    pub encryption_status: EncryptionStatus,
    /// Sync status
    pub sync_status: SyncStatus,
    /// Usage statistics
    pub usage_stats: UsageStatistics,
    /// Last known balance (cached for performance)
    pub last_balance: Option<u64>,
    /// Last sync block height
    pub last_sync_height: Option<u64>,
    /// Last sync timestamp
    pub last_sync_time: Option<DateTime<Utc>>,
}

/// Type of wallet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WalletType {
    /// Simple single-key wallet
    Simple,
    /// HD wallet with BIP32/44 support
    HD,
    /// Multi-signature wallet
    MultiSig,
    /// Hardware wallet
    Hardware,
    /// Watch-only wallet
    WatchOnly,
}

/// Encryption status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EncryptionStatus {
    /// Not encrypted
    None,
    /// Encrypted with passphrase
    Passphrase,
    /// Hardware-backed encryption
    Hardware,
}

/// Synchronization status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStatus {
    /// Last sync timestamp
    pub last_sync: Option<DateTime<Utc>>,
    /// Current block height synced to
    pub synced_height: u32,
    /// Whether sync is in progress
    pub is_syncing: bool,
    /// Sync progress percentage (0-100)
    pub sync_progress: u8,
}

/// Usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageStatistics {
    /// Total transactions created
    pub total_transactions: u64,
    /// Total amount transacted
    pub total_amount: u64,
    /// Average transaction fee
    pub average_fee: u64,
    /// Last transaction timestamp
    pub last_transaction: Option<DateTime<Utc>>,
    /// Addresses generated
    pub addresses_generated: u32,
}

/// Wallet manager settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletManagerSettings {
    /// Default fee rate (satoshis per byte)
    pub default_fee_rate: u64,
    /// Default UTXO selection strategy
    pub default_selection_strategy: UTXOSelectionStrategy,
    /// Enable RBF by default
    pub enable_rbf: bool,
    /// Dust threshold
    pub dust_threshold: u64,
    /// Address gap limit
    pub gap_limit: u32,
    /// Auto-backup interval (hours)
    pub backup_interval: u32,
    /// Maximum fee rate protection
    pub max_fee_rate: u64,
}

/// Transaction preparation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionPreparation {
    /// Estimated transaction size in bytes
    pub estimated_size: u64,
    /// Estimated fee
    pub estimated_fee: u64,
    /// Total input value
    pub input_value: u64,
    /// Total output value
    pub output_value: u64,
    /// Change amount
    pub change_amount: u64,
    /// Fee rate used
    pub fee_rate: u64,
    /// Number of inputs
    pub input_count: usize,
    /// Number of outputs
    pub output_count: usize,
    /// Whether RBF is enabled
    pub rbf_enabled: bool,
}

/// Wallet restore options
#[derive(Debug, Clone)]
pub struct WalletRestoreOptions {
    /// Mnemonic phrase
    pub mnemonic: String,
    /// Optional passphrase
    pub passphrase: Option<String>,
    /// Wallet name
    pub name: String,
    /// Account discovery limit
    pub account_discovery_limit: u32,
    /// Address gap limit for discovery
    pub gap_limit: u32,
    /// Whether to rescan blockchain
    pub rescan: bool,
}

impl AdvancedWalletManager {
    /// Create a new advanced wallet manager
    pub fn new() -> Self {
        Self {
            hd_wallets: HashMap::new(),
            simple_wallets: SimpleWalletManager::new(),
            transaction_manager: None,
            wallet_metadata: HashMap::new(),
            settings: WalletManagerSettings::default(),
        }
    }

    /// Create wallet manager with blockchain integration
    pub fn with_blockchain(transaction_manager: Arc<RwLock<TransactionManager>>) -> Self {
        let mut manager = Self::new();
        manager.transaction_manager = Some(transaction_manager.clone());
        manager.simple_wallets.set_transaction_manager(transaction_manager);
        manager
    }

    /// Set transaction manager
    pub fn set_transaction_manager(&mut self, transaction_manager: Arc<RwLock<TransactionManager>>) {
        self.transaction_manager = Some(transaction_manager.clone());
        self.simple_wallets.set_transaction_manager(transaction_manager);
    }

    /// Create a new HD wallet
    pub fn create_hd_wallet(&mut self, name: String, entropy: Option<[u8; 32]>) -> Result<Uuid> {
        let wallet = HDWallet::new(name, entropy)?;
        let wallet_id = wallet.id;

        // Create metadata
        let metadata = WalletMetadata {
            wallet_id,
            wallet_type: WalletType::HD,
            is_default: self.hd_wallets.is_empty() && self.simple_wallets.list_wallets().is_empty(),
            last_backup: None,
            encryption_status: EncryptionStatus::None,
            sync_status: SyncStatus {
                last_sync: None,
                synced_height: 0,
                is_syncing: false,
                sync_progress: 0,
            },
            usage_stats: UsageStatistics {
                total_transactions: 0,
                total_amount: 0,
                average_fee: 0,
                last_transaction: None,
                addresses_generated: 0,
            },
            last_balance: None,
            last_sync_height: None,
            last_sync_time: None,
        };

        self.hd_wallets.insert(wallet_id, wallet);
        self.wallet_metadata.insert(wallet_id, metadata);

        Ok(wallet_id)
    }

    /// Restore HD wallet from mnemonic
    pub fn restore_hd_wallet(&mut self, options: WalletRestoreOptions) -> Result<Uuid> {
        let wallet = HDWallet::from_mnemonic(
            options.name,
            &options.mnemonic,
            options.passphrase.as_deref(),
        )?;
        let wallet_id = wallet.id;

        // Create metadata
        let metadata = WalletMetadata {
            wallet_id,
            wallet_type: WalletType::HD,
            is_default: false,
            last_backup: None,
            encryption_status: EncryptionStatus::None,
            sync_status: SyncStatus {
                last_sync: None,
                synced_height: 0,
                is_syncing: false,
                sync_progress: 0,
            },
            usage_stats: UsageStatistics {
                total_transactions: 0,
                total_amount: 0,
                average_fee: 0,
                last_transaction: None,
                addresses_generated: 0,
            },
            last_balance: None,
            last_sync_height: None,
            last_sync_time: None,
        };

        self.hd_wallets.insert(wallet_id, wallet);
        self.wallet_metadata.insert(wallet_id, metadata);

        // If rescan is requested, trigger blockchain rescan
        if options.rescan {
            self.rescan_wallet_from_genesis(wallet_id)?;
        }

        Ok(wallet_id)
    }

    /// Create a simple wallet (legacy support)
    pub fn create_simple_wallet(&mut self, name: String) -> Result<Uuid> {
        let wallet = self.simple_wallets.create_wallet(name)?;
        let wallet_id = Uuid::new_v4(); // Simple wallets don't have UUIDs, so we create one

        let metadata = WalletMetadata {
            wallet_id,
            wallet_type: WalletType::Simple,
            is_default: false,
            last_backup: None,
            encryption_status: EncryptionStatus::None,
            sync_status: SyncStatus {
                last_sync: None,
                synced_height: 0,
                is_syncing: false,
                sync_progress: 0,
            },
            usage_stats: UsageStatistics {
                total_transactions: 0,
                total_amount: 0,
                average_fee: 0,
                last_transaction: None,
                addresses_generated: 1, // Simple wallet has one address
            },
            last_balance: None,
            last_sync_height: None,
            last_sync_time: None,
        };

        self.wallet_metadata.insert(wallet_id, metadata);
        Ok(wallet_id)
    }

    /// Get HD wallet
    pub fn get_hd_wallet(&mut self, wallet_id: Uuid) -> Option<&mut HDWallet> {
        self.hd_wallets.get_mut(&wallet_id)
    }

    /// List all wallets
    pub fn list_all_wallets(&self) -> Vec<WalletSummary> {
        let mut summaries = Vec::new();

        // HD wallets
        for (id, wallet) in &self.hd_wallets {
            let metadata = self.wallet_metadata.get(id);
            summaries.push(WalletSummary {
                id: *id,
                name: wallet.name.clone(),
                wallet_type: WalletType::HD,
                is_default: metadata.map(|m| m.is_default).unwrap_or(false),
                balance: 0, // Would be calculated from UTXO set
                account_count: wallet.accounts.len() as u32,
                address_count: wallet.accounts.values()
                    .map(|acc| acc.external_addresses.len() + acc.change_addresses.len())
                    .sum::<usize>() as u32,
                created_at: wallet.created_at,
                last_sync: wallet.last_sync,
            });
        }

        // Simple wallets
        for wallet in self.simple_wallets.list_wallets() {
            summaries.push(WalletSummary {
                id: wallet.id,
                name: wallet.name.clone(),
                wallet_type: WalletType::Simple,
                is_default: false,
                balance: wallet.balance,
                account_count: 1,
                address_count: 1,
                created_at: wallet.created_at,
                last_sync: None,
            });
        }

        summaries
    }

    /// Prepare transaction (estimate fees, validate inputs)
    pub async fn prepare_transaction(
        &mut self,
        wallet_id: Uuid,
        outputs: Vec<(String, u64)>,
        options: Option<TxBuildOptions>,
    ) -> Result<TransactionPreparation> {
        let options = options.unwrap_or_default();

        if let Some(hd_wallet) = self.hd_wallets.get_mut(&wallet_id) {
            // For HD wallet, use the default account (0)
            let account_index = 0;
            
            if let Some(tx_manager) = &self.transaction_manager {
                let tx_manager = tx_manager.read().await;
                let utxo_set = tx_manager.get_utxo_set();
                
                // Get account addresses
                if let Some(account) = hd_wallet.accounts.get(&account_index) {
                    let addresses = account.get_all_addresses();
                    let available_utxos = self.collect_utxos_for_addresses(&addresses, utxo_set)?;
                    
                    // Calculate requirements
                    let total_output: u64 = outputs.iter().map(|(_, amount)| amount).sum();
                    let estimated_size = self.estimate_transaction_size(available_utxos.len(), outputs.len());
                    let estimated_fee = estimated_size * options.fee_rate;
                    let total_required = total_output + estimated_fee;
                    
                    // Calculate input value
                    let input_value: u64 = available_utxos.iter().map(|utxo| utxo.value()).sum();
                    let change_amount = if input_value > total_required {
                        input_value - total_required
                    } else {
                        0
                    };

                    return Ok(TransactionPreparation {
                        estimated_size,
                        estimated_fee,
                        input_value,
                        output_value: total_output,
                        change_amount,
                        fee_rate: options.fee_rate,
                        input_count: available_utxos.len(),
                        output_count: outputs.len() + if change_amount > options.dust_threshold { 1 } else { 0 },
                        rbf_enabled: options.enable_rbf,
                    });
                }
            }
        }

        Err(BlockchainError::WalletNotFound(wallet_id.to_string()))
    }

    /// Build and sign transaction
    pub async fn build_transaction(
        &mut self,
        wallet_id: Uuid,
        outputs: Vec<(String, u64)>,
        options: Option<TxBuildOptions>,
    ) -> Result<Transaction> {
        let options = options.unwrap_or_default();

        if let Some(hd_wallet) = self.hd_wallets.get_mut(&wallet_id) {
            let account_index = 0; // Use default account

            if let Some(tx_manager) = &self.transaction_manager {
                let tx_manager = tx_manager.read().await;
                let utxo_set = tx_manager.get_utxo_set();
                
                let transaction = hd_wallet.build_transaction(
                    account_index,
                    outputs.clone(),
                    options,
                    utxo_set,
                ).await?;

                // Update usage statistics
                if let Some(metadata) = self.wallet_metadata.get_mut(&wallet_id) {
                    metadata.usage_stats.total_transactions += 1;
                    metadata.usage_stats.total_amount += outputs.iter().map(|(_, amount)| amount).sum::<u64>();
                    metadata.usage_stats.last_transaction = Some(Utc::now());
                }

                return Ok(transaction);
            }
        }

        Err(BlockchainError::WalletNotFound(wallet_id.to_string()))
    }

    /// Create account in HD wallet
    pub fn create_account(&mut self, wallet_id: Uuid, account_name: String) -> Result<u32> {
        if let Some(hd_wallet) = self.hd_wallets.get_mut(&wallet_id) {
            let account_index = hd_wallet.create_account(account_name)?;
            
            // Update metadata
            if let Some(metadata) = self.wallet_metadata.get_mut(&wallet_id) {
                metadata.usage_stats.addresses_generated += 1; // At least one address generated
            }
            
            Ok(account_index)
        } else {
            Err(BlockchainError::WalletNotFound(wallet_id.to_string()))
        }
    }

    /// Generate new receiving address
    pub fn generate_address(&mut self, wallet_id: Uuid, account_index: Option<u32>) -> Result<String> {
        if let Some(hd_wallet) = self.hd_wallets.get_mut(&wallet_id) {
            let account_idx = account_index.unwrap_or(0);
            
            if let Some(account) = hd_wallet.get_account(account_idx) {
                let address = account.get_next_address()?;
                
                // Update metadata
                if let Some(metadata) = self.wallet_metadata.get_mut(&wallet_id) {
                    metadata.usage_stats.addresses_generated += 1;
                }
                
                Ok(address)
            } else {
                Err(BlockchainError::AccountNotFound(account_idx))
            }
        } else {
            Err(BlockchainError::WalletNotFound(wallet_id.to_string()))
        }
    }

    /// Get wallet balance
    /// Get balance for a specific address using the UTXO set
    pub async fn get_address_balance(&self, address: &str) -> Result<u64> {
        if let Some(tx_manager) = &self.transaction_manager {
            let tx_manager = tx_manager.read().await;
            let utxo_set = tx_manager.get_utxo_set();
            Ok(utxo_set.get_balance(address))
        } else {
            Ok(0) // Return 0 if no transaction manager available
        }
    }

    pub async fn get_wallet_balance(&self, wallet_id: Uuid) -> Result<u64> {
        if let Some(hd_wallet) = self.hd_wallets.get(&wallet_id) {
            if let Some(tx_manager) = &self.transaction_manager {
                let tx_manager = tx_manager.read().await;
                let utxo_set = tx_manager.get_utxo_set();
                
                let mut total_balance = 0u64;
                for account in hd_wallet.accounts.values() {
                    for address in account.get_all_addresses() {
                        total_balance += utxo_set.get_balance(&address);
                    }
                }
                return Ok(total_balance);
            }
        }
        
        Ok(0)
    }

    /// Sync wallet with blockchain
    pub async fn sync_wallet(&mut self, wallet_id: Uuid) -> Result<()> {
        if let Some(_) = self.hd_wallets.get(&wallet_id) {
            // Update sync status
            if let Some(metadata) = self.wallet_metadata.get_mut(&wallet_id) {
                metadata.sync_status.is_syncing = true;
                metadata.sync_status.sync_progress = 0;
            }

            // Perform sync operations
            self.discover_addresses(wallet_id).await?;
            self.update_balances(wallet_id).await?;

            // Update sync completion
            if let Some(metadata) = self.wallet_metadata.get_mut(&wallet_id) {
                metadata.sync_status.is_syncing = false;
                metadata.sync_status.sync_progress = 100;
                metadata.sync_status.last_sync = Some(Utc::now());
            }

            Ok(())
        } else {
            Err(BlockchainError::WalletNotFound(wallet_id.to_string()))
        }
    }

    /// Discover addresses with gap limit
    async fn discover_addresses(&mut self, wallet_id: Uuid) -> Result<()> {
        if let Some(hd_wallet) = self.hd_wallets.get_mut(&wallet_id) {
            if let Some(tx_manager) = &self.transaction_manager {
                let tx_manager = tx_manager.read().await;
                let utxo_set = tx_manager.get_utxo_set();

                for account in hd_wallet.accounts.values_mut() {
                    let gap_limit = account.gap_limit;
                    let mut gap_count = 0;
                    let mut index = account.next_address_index;

                    // Discover external addresses
                    while gap_count < gap_limit {
                        let key_pair = account.derive_address(index)?;
                        let has_transactions = !utxo_set.get_utxos_for_address(&key_pair.address).is_empty();

                        if has_transactions {
                            gap_count = 0;
                        } else {
                            gap_count += 1;
                        }
                        index += 1;
                    }

                    // Update next address index
                    account.next_address_index = index - gap_limit;
                }
            }
        }
        Ok(())
    }

    /// Update wallet balances
    async fn update_balances(&mut self, wallet_id: Uuid) -> Result<()> {
        if let Some(hd_wallet) = self.hd_wallets.get(&wallet_id) {
            if let Some(tx_manager) = &self.transaction_manager {
                let tx_manager = tx_manager.read().await;
                let utxo_set = tx_manager.get_utxo_set();

                // Update balance in metadata or wallet structure
                let mut total_balance = 0u64;
                for account in hd_wallet.accounts.values() {
                    for address in account.get_all_addresses() {
                        total_balance += utxo_set.get_balance(&address);
                    }
                }
                
                // Update the wallet metadata with current balance
                if let Some(metadata) = self.wallet_metadata.get_mut(&wallet_id) {
                    metadata.last_balance = Some(total_balance);
                    metadata.last_sync_height = Some(tx_manager.get_current_height());
                    metadata.last_sync_time = Some(chrono::Utc::now());
                }
            }
        }
        Ok(())
    }

    /// Rescan wallet from genesis block
    fn rescan_wallet_from_genesis(&mut self, _wallet_id: Uuid) -> Result<()> {
        // In a full implementation, this would trigger a blockchain rescan
        // For now, we'll just mark it as needing sync
        Ok(())
    }

    /// Get wallet statistics
    pub async fn get_wallet_statistics(&self, wallet_id: Uuid) -> Result<WalletStatistics> {
        if let Some(hd_wallet) = self.hd_wallets.get(&wallet_id) {
            if let Some(tx_manager) = &self.transaction_manager {
                let tx_manager = tx_manager.read().await;
                let utxo_set = tx_manager.get_utxo_set();
                return Ok(hd_wallet.get_statistics(utxo_set));
            }
        }
        
        Err(BlockchainError::WalletNotFound(wallet_id.to_string()))
    }

    /// Export wallet data
    pub fn export_wallet(&self, wallet_id: Uuid, include_private_keys: bool) -> Result<WalletExport> {
        if let Some(hd_wallet) = self.hd_wallets.get(&wallet_id) {
            Ok(WalletExport {
                wallet_id,
                name: hd_wallet.name.clone(),
                wallet_type: WalletType::HD,
                created_at: hd_wallet.created_at,
                master_xpub: hd_wallet.master_xpub.serialize()?,
                master_xpriv: if include_private_keys {
                    Some(hd_wallet.master_xpriv.serialize()?)
                } else {
                    None
                },
                mnemonic: if include_private_keys {
                    hd_wallet.mnemonic.clone()
                } else {
                    None
                },
                accounts: hd_wallet.accounts.keys().cloned().collect(),
            })
        } else {
            Err(BlockchainError::WalletNotFound(wallet_id.to_string()))
        }
    }

    /// Update wallet settings
    pub fn update_settings(&mut self, settings: WalletManagerSettings) {
        self.settings = settings;
    }

    /// Get current settings
    pub fn get_settings(&self) -> &WalletManagerSettings {
        &self.settings
    }

    // Helper methods

    /// Collect UTXOs for multiple addresses
    fn collect_utxos_for_addresses(&self, addresses: &[String], utxo_set: &UTXOSet) -> Result<Vec<crate::utxo::UTXO>> {
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

    /// Estimate transaction size
    fn estimate_transaction_size(&self, input_count: usize, output_count: usize) -> u64 {
        // Base transaction: 10 bytes
        // Each input: ~148 bytes
        // Each output: ~34 bytes
        let size = 10 + (input_count * 148) + (output_count * 34);
        size as u64
    }
}

/// Wallet summary for listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletSummary {
    pub id: Uuid,
    pub name: String,
    pub wallet_type: WalletType,
    pub is_default: bool,
    pub balance: u64,
    pub account_count: u32,
    pub address_count: u32,
    pub created_at: DateTime<Utc>,
    pub last_sync: Option<DateTime<Utc>>,
}

/// Wallet export data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletExport {
    pub wallet_id: Uuid,
    pub name: String,
    pub wallet_type: WalletType,
    pub created_at: DateTime<Utc>,
    pub master_xpub: String,
    pub master_xpriv: Option<String>,
    pub mnemonic: Option<String>,
    pub accounts: Vec<u32>,
}

impl Default for WalletManagerSettings {
    fn default() -> Self {
        Self {
            default_fee_rate: 1000,
            default_selection_strategy: UTXOSelectionStrategy::BranchAndBound,
            enable_rbf: true,
            dust_threshold: 546,
            gap_limit: 20,
            backup_interval: 24,
            max_fee_rate: 10000,
        }
    }
}

impl Default for UsageStatistics {
    fn default() -> Self {
        Self {
            total_transactions: 0,
            total_amount: 0,
            average_fee: 0,
            last_transaction: None,
            addresses_generated: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_advanced_wallet_manager_creation() {
        let mut manager = AdvancedWalletManager::new();
        
        let wallet_id = manager.create_hd_wallet("Test HD Wallet".to_string(), None).unwrap();
        assert!(!wallet_id.is_nil());
        
        let wallets = manager.list_all_wallets();
        assert_eq!(wallets.len(), 1);
        assert_eq!(wallets[0].name, "Test HD Wallet");
    }

    #[test]
    fn test_wallet_restore() {
        let mut manager = AdvancedWalletManager::new();
        
        // Create original wallet to get mnemonic
        let original_id = manager.create_hd_wallet("Original".to_string(), Some([42u8; 32])).unwrap();
        let original_wallet = manager.get_hd_wallet(original_id).unwrap();
        let mnemonic = original_wallet.mnemonic.as_ref().unwrap().clone();
        
        // Restore wallet
        let restore_options = WalletRestoreOptions {
            mnemonic,
            passphrase: None,
            name: "Restored Wallet".to_string(),
            account_discovery_limit: 10,
            gap_limit: 20,
            rescan: false,
        };
        
        let restored_id = manager.restore_hd_wallet(restore_options).unwrap();
        assert_ne!(original_id, restored_id); // Different IDs
        
        let wallets = manager.list_all_wallets();
        assert_eq!(wallets.len(), 2);
    }

    #[test]
    fn test_account_creation() {
        let mut manager = AdvancedWalletManager::new();
        let wallet_id = manager.create_hd_wallet("Test Wallet".to_string(), None).unwrap();
        
        // Create account first (account 0 doesn't exist by default in our implementation)
        let account_index = manager.create_account(wallet_id, "Main Account".to_string()).unwrap();
        assert_eq!(account_index, 0);
        
        // Generate address
        let address = manager.generate_address(wallet_id, Some(account_index)).unwrap();
        assert!(address.starts_with("edu1q"));
    }
}