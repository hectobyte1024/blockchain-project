//! UTXO Set Management
//! 
//! Handles unspent transaction outputs (UTXOs) for the blockchain.
//! Provides efficient tracking, validation, and management of spendable outputs.

use crate::{Hash256, BlockchainError, Result};
use crate::transaction::{Transaction, TransactionOutput, TransactionInput};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Represents an unspent transaction output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UTXO {
    /// Transaction hash that created this output
    pub tx_hash: Hash256,
    /// Output index within the transaction
    pub output_index: u32,
    /// The actual output data
    pub output: TransactionOutput,
    /// Block height where this UTXO was created
    pub block_height: u32,
    /// Whether this is from a coinbase transaction
    pub is_coinbase: bool,
    /// Timestamp when this UTXO was created
    pub created_at: DateTime<Utc>,
}

impl UTXO {
    /// Create a new UTXO
    pub fn new(
        tx_hash: Hash256,
        output_index: u32,
        output: TransactionOutput,
        block_height: u32,
        is_coinbase: bool,
    ) -> Self {
        Self {
            tx_hash,
            output_index,
            output,
            block_height,
            is_coinbase,
            created_at: Utc::now(),
        }
    }

    /// Get the outpoint string (txhash:index)
    pub fn get_outpoint(&self) -> String {
        format!("{}:{}", hex::encode(&self.tx_hash), self.output_index)
    }

    /// Check if this UTXO is mature (can be spent)
    pub fn is_mature(&self, current_height: u32) -> bool {
        if !self.is_coinbase {
            return true; // Regular transactions are immediately spendable
        }
        
        // Coinbase outputs need 100 confirmations
        current_height >= self.block_height + 100
    }

    /// Get the value of this UTXO
    pub fn value(&self) -> u64 {
        self.output.value
    }

    /// Extract address from script (simplified)
    pub fn get_address(&self) -> Option<String> {
        // For now, return a simplified address
        // In a real implementation, this would parse the script
        if self.output.script_pubkey.len() >= 25 {
            Some(format!("edu1q{}", hex::encode(&self.output.script_pubkey[3..23])))
        } else {
            None
        }
    }
}

/// Manages the set of all unspent transaction outputs
#[derive(Debug, Clone)]
pub struct UTXOSet {
    /// Map from outpoint to UTXO
    utxos: HashMap<String, UTXO>,
    /// Index by address for quick lookups
    address_index: HashMap<String, Vec<String>>, // address -> outpoints
    /// Total supply tracking
    total_supply: u64,
    /// Current block height
    current_height: u32,
}

impl UTXOSet {
    /// Create a new empty UTXO set
    pub fn new() -> Self {
        Self {
            utxos: HashMap::new(),
            address_index: HashMap::new(),
            total_supply: 0,
            current_height: 0,
        }
    }

    /// Add UTXOs from a transaction
    pub fn add_transaction(&mut self, tx: &Transaction, block_height: u32) -> Result<()> {
        let tx_hash = tx.get_hash()?;
        
        // Remove spent UTXOs (inputs)
        if !tx.is_coinbase() {
            for input in &tx.inputs {
                let outpoint = format!("{}:{}", hex::encode(&input.prev_tx_hash), input.prev_output_index);
                
                if let Some(utxo) = self.utxos.remove(&outpoint) {
                    // Update address index
                    if let Some(address) = utxo.get_address() {
                        if let Some(outpoints) = self.address_index.get_mut(&address) {
                            outpoints.retain(|op| *op != outpoint);
                            if outpoints.is_empty() {
                                self.address_index.remove(&address);
                            }
                        }
                    }
                    self.total_supply -= utxo.value();
                } else {
                    return Err(BlockchainError::InvalidTransaction(
                        format!("Attempted to spend non-existent UTXO: {}", outpoint)
                    ));
                }
            }
        }

        // Add new UTXOs (outputs)
        for (index, output) in tx.outputs.iter().enumerate() {
            let utxo = UTXO::new(
                tx_hash,
                index as u32,
                output.clone(),
                block_height,
                tx.is_coinbase(),
            );

            let outpoint = utxo.get_outpoint();
            
            // Update address index
            if let Some(address) = utxo.get_address() {
                self.address_index
                    .entry(address)
                    .or_insert_with(Vec::new)
                    .push(outpoint.clone());
            }

            self.total_supply += utxo.value();
            self.utxos.insert(outpoint, utxo);
        }

        self.current_height = block_height;
        Ok(())
    }

    /// Get UTXO by outpoint
    pub fn get_utxo(&self, outpoint: &str) -> Option<&UTXO> {
        self.utxos.get(outpoint)
    }

    /// Get all UTXOs for an address
    pub fn get_utxos_for_address(&self, address: &str) -> Vec<&UTXO> {
        if let Some(outpoints) = self.address_index.get(address) {
            outpoints
                .iter()
                .filter_map(|op| self.utxos.get(op))
                .filter(|utxo| utxo.is_mature(self.current_height))
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get balance for an address
    pub fn get_balance(&self, address: &str) -> u64 {
        self.get_utxos_for_address(address)
            .iter()
            .map(|utxo| utxo.value())
            .sum()
    }

    /// Get current blockchain height
    pub fn get_current_height(&self) -> u64 {
        self.current_height as u64
    }

    /// Check if a UTXO exists and is spendable
    pub fn is_utxo_available(&self, outpoint: &str) -> bool {
        if let Some(utxo) = self.utxos.get(outpoint) {
            utxo.is_mature(self.current_height)
        } else {
            false
        }
    }

    /// Select UTXOs for spending (coin selection)
    pub fn select_utxos_for_amount(&self, address: &str, amount: u64) -> Result<Vec<UTXO>> {
        let available_utxos = self.get_utxos_for_address(address);
        
        // Simple greedy coin selection
        let mut selected = Vec::new();
        let mut total_selected = 0u64;

        // Sort by value descending (largest first)
        let mut sorted_utxos = available_utxos;
        sorted_utxos.sort_by(|a, b| b.value().cmp(&a.value()));

        for utxo in sorted_utxos {
            selected.push(utxo.clone());
            total_selected += utxo.value();

            if total_selected >= amount {
                return Ok(selected);
            }
        }

        Err(BlockchainError::InsufficientFunds(
            format!("Need {} satoshis, only have {} available", amount, total_selected)
        ))
    }

    /// Validate that transaction inputs exist and are spendable
    pub fn validate_transaction(&self, tx: &Transaction) -> Result<()> {
        if tx.is_coinbase() {
            return Ok(()); // Coinbase transactions don't spend UTXOs
        }

        let mut input_value = 0u64;
        let mut spent_outpoints = HashSet::new();

        for input in &tx.inputs {
            let outpoint = format!("{}:{}", hex::encode(&input.prev_tx_hash), input.prev_output_index);

            // Check for double spending within this transaction
            if spent_outpoints.contains(&outpoint) {
                return Err(BlockchainError::InvalidTransaction(
                    "Transaction attempts to double-spend UTXO".to_string()
                ));
            }
            spent_outpoints.insert(outpoint.clone());

            // Check if UTXO exists and is spendable
            if let Some(utxo) = self.utxos.get(&outpoint) {
                if !utxo.is_mature(self.current_height) {
                    return Err(BlockchainError::InvalidTransaction(
                        format!("Attempted to spend immature coinbase UTXO: {}", outpoint)
                    ));
                }
                input_value += utxo.value();
            } else {
                return Err(BlockchainError::InvalidTransaction(
                    format!("Referenced UTXO does not exist: {}", outpoint)
                ));
            }
        }

        // Calculate output value
        let output_value: u64 = tx.outputs.iter().map(|o| o.value).sum();

        // Validate fee (input_value must be >= output_value)
        if input_value < output_value {
            return Err(BlockchainError::InvalidTransaction(
                format!("Transaction outputs ({}) exceed inputs ({})", output_value, input_value)
            ));
        }

        Ok(())
    }

    /// Get current total supply
    pub fn get_total_supply(&self) -> u64 {
        self.total_supply
    }



    /// Get total number of UTXOs
    pub fn get_utxo_count(&self) -> usize {
        self.utxos.len()
    }

    /// Add a single UTXO (used by consensus module)
    pub fn add_utxo(&mut self, tx_hash: Hash256, output_index: u32, utxo: UTXO) -> Result<()> {
        let outpoint = format!("{}:{}", hex::encode(&tx_hash), output_index);
        
        // Update address index
        if let Some(address) = utxo.get_address() {
            self.address_index
                .entry(address)
                .or_insert_with(Vec::new)
                .push(outpoint.clone());
        }

        self.total_supply += utxo.value();
        self.utxos.insert(outpoint, utxo);
        Ok(())
    }

    /// Remove a single UTXO (used by consensus module)
    pub fn remove_utxo(&mut self, tx_hash: &Hash256, output_index: u32) -> Result<()> {
        let outpoint = format!("{}:{}", hex::encode(tx_hash), output_index);
        
        if let Some(utxo) = self.utxos.remove(&outpoint) {
            // Update address index
            if let Some(address) = utxo.get_address() {
                if let Some(outpoints) = self.address_index.get_mut(&address) {
                    outpoints.retain(|op| *op != outpoint);
                    if outpoints.is_empty() {
                        self.address_index.remove(&address);
                    }
                }
            }
            self.total_supply -= utxo.value();
            Ok(())
        } else {
            Err(BlockchainError::InvalidTransaction(
                format!("Attempted to remove non-existent UTXO: {}", outpoint)
            ))
        }
    }

    /// Calculate transaction fee
    pub fn calculate_fee(&self, tx: &Transaction) -> Result<u64> {
        if tx.is_coinbase() {
            return Ok(0); // Coinbase transactions have no fee
        }

        let input_value: u64 = tx.inputs
            .iter()
            .map(|input| {
                let outpoint = format!("{}:{}", hex::encode(&input.prev_tx_hash), input.prev_output_index);
                self.utxos.get(&outpoint).map(|utxo| utxo.value()).unwrap_or(0)
            })
            .sum();

        let output_value: u64 = tx.outputs.iter().map(|o| o.value).sum();

        if input_value >= output_value {
            Ok(input_value - output_value)
        } else {
            Err(BlockchainError::InvalidTransaction(
                "Transaction outputs exceed inputs".to_string()
            ))
        }
    }

    /// Create a snapshot of the UTXO set for rollback
    pub fn create_snapshot(&self) -> UTXOSetSnapshot {
        UTXOSetSnapshot {
            utxos: self.utxos.clone(),
            address_index: self.address_index.clone(),
            total_supply: self.total_supply,
            current_height: self.current_height,
        }
    }

    /// Restore from snapshot (for chain reorganization)
    pub fn restore_snapshot(&mut self, snapshot: UTXOSetSnapshot) {
        self.utxos = snapshot.utxos;
        self.address_index = snapshot.address_index;
        self.total_supply = snapshot.total_supply;
        self.current_height = snapshot.current_height;
    }
}

/// Snapshot of UTXO set for rollback operations
#[derive(Debug, Clone)]
pub struct UTXOSetSnapshot {
    utxos: HashMap<String, UTXO>,
    address_index: HashMap<String, Vec<String>>,
    total_supply: u64,
    current_height: u32,
}

impl Default for UTXOSet {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transaction::{Transaction, TransactionInput, TransactionOutput};

    #[test]
    fn test_utxo_creation() {
        let tx_hash = [1u8; 32];
        let output = TransactionOutput::new(1000, vec![0x76, 0xa9]);
        let utxo = UTXO::new(tx_hash, 0, output, 100, false);
        
        assert_eq!(utxo.value(), 1000);
        assert!(utxo.is_mature(200));
        assert!(!utxo.is_coinbase);
    }

    #[test]
    fn test_coinbase_maturity() {
        let tx_hash = [2u8; 32];
        let output = TransactionOutput::new(5000000000, vec![0x76, 0xa9]); // 50 EDU
        let utxo = UTXO::new(tx_hash, 0, output, 100, true);
        
        assert!(!utxo.is_mature(150)); // Only 50 blocks
        assert!(utxo.is_mature(200));  // 100+ blocks
    }

    #[test]
    fn test_utxo_set_operations() {
        let mut utxo_set = UTXOSet::new();
        
        // Create a simple transaction
        let outputs = vec![TransactionOutput::new(1000, vec![0x76, 0xa9])];
        let mut tx = Transaction::new(1, Vec::new(), outputs);
        
        // Add transaction to UTXO set
        assert!(utxo_set.add_transaction(&tx, 100).is_ok());
        
        assert_eq!(utxo_set.get_total_supply(), 1000);
        assert_eq!(utxo_set.get_utxo_count(), 1);
    }
}