//! Treasury Management System
//! 
//! Handles manual coin sales for real-world cash payments.
//! This allows the platform owner to sell EDU coins to users who pay with cash.

use blockchain_core::transaction::{Transaction, TransactionInput, TransactionOutput};
use blockchain_core::wallet::{WalletManager, Wallet};
use blockchain_core::tx_builder::TransactionBuilder;
use blockchain_core::{Hash256, Result as BlockchainResult};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, error};
use chrono::Utc;

/// Treasury wallet address (from genesis allocation)
pub const TREASURY_ADDRESS: &str = "edu1qTreasury00000000000000000000";

/// Treasury private key (TESTNET ONLY - in production this would be in HSM/secure storage)
/// This is a deterministic key derived from "Treasury" for testnet consistency
const TREASURY_PRIVATE_KEY_HEX: &str = "1111111111111111111111111111111111111111111111111111111111111111";

/// Treasury sale record for tracking off-chain payments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaleRecord {
    /// Unique sale ID
    pub sale_id: String,
    /// Buyer's wallet address
    pub buyer_address: String,
    /// Amount of EDU coins sold
    pub amount: u64,
    /// Price per EDU in USD (cents)
    pub price_per_edu_cents: u64,
    /// Total payment received in USD (cents)
    pub total_payment_cents: u64,
    /// Payment method (cash, check, bank transfer, etc.)
    pub payment_method: String,
    /// Payment proof/receipt number
    pub payment_proof: String,
    /// Transaction hash on blockchain
    pub tx_hash: Option<Hash256>,
    /// Timestamp of sale
    pub timestamp: i64,
    /// Status: pending, completed, failed
    pub status: SaleStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SaleStatus {
    Pending,
    Completed,
    Failed,
}

/// Treasury manager for coin sales
pub struct TreasuryManager {
    /// Reference to blockchain backend
    blockchain: Arc<crate::blockchain::BlockchainBackend>,
    /// Sale records (in-memory, should be persisted to DB in production)
    sales: Arc<RwLock<Vec<SaleRecord>>>,
    /// Current price per EDU in USD cents
    price_per_edu_cents: Arc<RwLock<u64>>,
    /// Treasury wallet for signing transactions
    treasury_wallet: Wallet,
}

impl TreasuryManager {
    /// Create new treasury manager
    pub fn new(blockchain: Arc<crate::blockchain::BlockchainBackend>) -> Result<Self> {
        // Load treasury wallet from private key
        let treasury_wallet = Wallet::from_private_key_hex(TREASURY_PRIVATE_KEY_HEX)
            .map_err(|e| anyhow::anyhow!("Failed to load treasury wallet: {}", e))?;
        
        info!("💰 Treasury wallet loaded: {}", treasury_wallet.address);
        
        Ok(Self {
            blockchain,
            sales: Arc::new(RwLock::new(Vec::new())),
            price_per_edu_cents: Arc::new(RwLock::new(10)), // Default: $0.10 per EDU
            treasury_wallet,
        })
    }

    /// Set the current price per EDU coin
    pub async fn set_price(&self, price_cents: u64) {
        let mut price = self.price_per_edu_cents.write().await;
        *price = price_cents;
        info!("💰 Treasury price updated: ${:.2} per EDU", price_cents as f64 / 100.0);
    }

    /// Get current price
    pub async fn get_price(&self) -> u64 {
        *self.price_per_edu_cents.read().await
    }

    /// Calculate total cost for amount of EDU
    pub async fn calculate_cost(&self, amount: u64) -> u64 {
        let price = self.get_price().await;
        amount * price
    }

    /// Sell coins to a buyer (after receiving cash payment)
    pub async fn sell_coins(
        &self,
        buyer_address: String,
        amount: u64,
        payment_method: String,
        payment_proof: String,
    ) -> Result<SaleRecord> {
        info!("💵 Processing coin sale: {} EDU to {}", amount, buyer_address);

        // Calculate cost
        let price_cents = self.get_price().await;
        let total_cost = amount * price_cents;

        // Create sale ID
        let sale_id = format!("SALE-{}-{:x}", Utc::now().timestamp(), amount);

        // Create initial sale record
        let mut sale = SaleRecord {
            sale_id: sale_id.clone(),
            buyer_address: buyer_address.clone(),
            amount,
            price_per_edu_cents: price_cents,
            total_payment_cents: total_cost,
            payment_method,
            payment_proof,
            tx_hash: None,
            timestamp: Utc::now().timestamp(),
            status: SaleStatus::Pending,
        };

        // Check treasury balance
        let treasury_balance = self.blockchain.get_balance(TREASURY_ADDRESS).await?;
        if treasury_balance < amount {
            error!("❌ Treasury balance insufficient: has {}, need {}", treasury_balance, amount);
            sale.status = SaleStatus::Failed;
            self.sales.write().await.push(sale.clone());
            return Err(anyhow::anyhow!("Treasury balance insufficient"));
        }

        // Create and send transaction
        match self.create_sale_transaction(&buyer_address, amount).await {
            Ok(tx) => {
                let tx_hash = tx.get_hash()?;
                
                // Submit transaction to blockchain
                match self.blockchain.submit_transaction(tx).await {
                    Ok(_) => {
                        sale.tx_hash = Some(tx_hash);
                        sale.status = SaleStatus::Completed;
                        
                        info!("✅ Coin sale completed: {} EDU to {} for ${:.2}", 
                              amount, buyer_address, total_cost as f64 / 100.0);
                        info!("📝 Transaction hash: {}", hex::encode(tx_hash));
                        info!("💰 Payment: {} via {} ({})", 
                              format!("${:.2}", total_cost as f64 / 100.0),
                              sale.payment_method,
                              sale.payment_proof);
                    }
                    Err(e) => {
                        error!("❌ Failed to submit transaction: {}", e);
                        sale.status = SaleStatus::Failed;
                        return Err(anyhow::anyhow!("Failed to submit transaction: {}", e));
                    }
                }
            }
            Err(e) => {
                error!("❌ Failed to create transaction: {}", e);
                sale.status = SaleStatus::Failed;
                return Err(e);
            }
        }

        // Store sale record
        self.sales.write().await.push(sale.clone());

        Ok(sale)
    }

    /// Create a transaction sending coins from treasury to buyer
    async fn create_sale_transaction(
        &self,
        buyer_address: &str,
        amount: u64,
    ) -> Result<Transaction> {
        // Get treasury UTXOs
        let utxo_set = self.blockchain.utxo_set.read().await;
        
        // Select UTXOs from treasury address
        // For now, just select one large UTXO (the genesis allocation)
        let treasury_utxos = utxo_set.get_utxos_for_address(TREASURY_ADDRESS);
        if treasury_utxos.is_empty() {
            return Err(anyhow::anyhow!("No UTXOs available in treasury"));
        }
        
        // Use first UTXO (should be the genesis allocation)
        let utxo = treasury_utxos[0];
        let input_value = utxo.value();
        
        // Calculate proper fee based on transaction size
        // Input: ~148 bytes, Output: ~34 bytes, Overhead: ~10 bytes
        // 1 input + 2 outputs (buyer + change) = 148 + 68 + 10 = 226 bytes
        let estimated_size = 226;
        let fee_rate = 1000; // sats/byte
        let estimated_fee = estimated_size * fee_rate; // 226000 satoshis
        
        let total_needed = amount + estimated_fee;
        
        if input_value < total_needed {
            return Err(anyhow::anyhow!("Insufficient funds in treasury UTXO"));
        }
        
        // Calculate change
        let change = input_value - amount - estimated_fee;
        
        // Create transaction outputs first
        let mut outputs = Vec::new();
        
        // Output to buyer
        let buyer_output = TransactionOutput::create_p2pkh(amount, buyer_address)
            .map_err(|e| anyhow::anyhow!("Failed to create buyer output: {}", e))?;
        outputs.push(buyer_output);
        
        // Change output back to treasury (if significant)
        if change > 546 { // Dust threshold
            let change_output = TransactionOutput::create_p2pkh(change, TREASURY_ADDRESS)
                .map_err(|e| anyhow::anyhow!("Failed to create change output: {}", e))?;
            outputs.push(change_output);
        }
        
        // Create transaction with empty script_sig (will sign next)
        let mut tx = Transaction::new(
            1,
            vec![TransactionInput::new(utxo.tx_hash, utxo.output_index, Vec::new())],
            outputs
        );
        
        // Sign the input
        for (input_index, input) in tx.inputs.clone().iter().enumerate() {
            // Calculate signature hash
            let sig_hash = tx.calculate_signature_hash(
                input_index,
                &utxo.output.script_pubkey,
                0x01 // SIGHASH_ALL
            );
            
            // Sign with treasury private key
            let signature = blockchain_core::crypto::sign_hash(
                &sig_hash,
                &self.treasury_wallet.private_key
            ).map_err(|e| anyhow::anyhow!("Failed to sign transaction: {}", e))?;
            
            // Append SIGHASH_ALL flag
            let mut sig_with_hashtype = signature;
            sig_with_hashtype.push(0x01);
            
            // Create script_sig: <sig_len> <signature+hashtype> <pubkey_len> <pubkey>
            let mut script_sig = Vec::new();
            script_sig.push(sig_with_hashtype.len() as u8);
            script_sig.extend_from_slice(&sig_with_hashtype);
            script_sig.push(self.treasury_wallet.public_key.len() as u8);
            script_sig.extend_from_slice(&self.treasury_wallet.public_key);
            
            // Update transaction input with signature
            tx.inputs[input_index].script_sig = script_sig;
        }
        
        info!("💳 Created sale transaction: {} EDU to {}", amount, buyer_address);
        info!("   Input: {}, Fee: {}, Change: {}", input_value, estimated_fee, change);
        
        Ok(tx)
    }

    /// Get all sale records
    pub async fn get_sales(&self) -> Vec<SaleRecord> {
        self.sales.read().await.clone()
    }

    /// Get sale by ID
    pub async fn get_sale(&self, sale_id: &str) -> Option<SaleRecord> {
        let sales = self.sales.read().await;
        sales.iter().find(|s| s.sale_id == sale_id).cloned()
    }

    /// Get sales statistics
    pub async fn get_stats(&self) -> SaleStats {
        let sales = self.sales.read().await;
        let treasury_balance = self.blockchain.get_balance(TREASURY_ADDRESS).await.unwrap_or(0);
        
        let total_sales = sales.len();
        let completed_sales = sales.iter().filter(|s| matches!(s.status, SaleStatus::Completed)).count();
        let total_edu_sold: u64 = sales.iter()
            .filter(|s| matches!(s.status, SaleStatus::Completed))
            .map(|s| s.amount)
            .sum();
        let total_revenue_cents: u64 = sales.iter()
            .filter(|s| matches!(s.status, SaleStatus::Completed))
            .map(|s| s.total_payment_cents)
            .sum();

        SaleStats {
            total_sales,
            completed_sales,
            failed_sales: sales.iter().filter(|s| matches!(s.status, SaleStatus::Failed)).count(),
            pending_sales: sales.iter().filter(|s| matches!(s.status, SaleStatus::Pending)).count(),
            total_edu_sold,
            total_revenue_cents,
            treasury_balance,
            current_price_cents: self.get_price().await,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaleStats {
    pub total_sales: usize,
    pub completed_sales: usize,
    pub failed_sales: usize,
    pub pending_sales: usize,
    pub total_edu_sold: u64,
    pub total_revenue_cents: u64,
    pub treasury_balance: u64,
    pub current_price_cents: u64,
}
