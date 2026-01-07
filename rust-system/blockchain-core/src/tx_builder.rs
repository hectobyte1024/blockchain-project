//! Transaction Creation and Signing
//! 
//! Provides functionality for creating, signing, and broadcasting blockchain transactions.
//! Integrates with wallet system and UTXO management.

use crate::{Hash256, BlockchainError, Result, PrivateKey};
use crate::transaction::{Transaction, TransactionInput, TransactionOutput};
use crate::utxo::{UTXOSet, UTXO};
use crate::wallet::Wallet;
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Transaction builder for creating new transactions
#[derive(Debug)]
pub struct TransactionBuilder {
    inputs: Vec<TxInput>,
    outputs: Vec<TxOutput>,
    fee_rate: u64, // satoshis per byte
    locktime: u32,
}

/// Input for transaction building
#[derive(Debug, Clone)]
struct TxInput {
    utxo: UTXO,
    private_key: PrivateKey,
}

/// Output for transaction building
#[derive(Debug, Clone)]
struct TxOutput {
    address: String,
    amount: u64,
}

impl TransactionBuilder {
    /// Create a new transaction builder
    pub fn new() -> Self {
        Self {
            inputs: Vec::new(),
            outputs: Vec::new(),
            fee_rate: 1000, // 1000 satoshis per byte default
            locktime: 0,
        }
    }

    /// Set the fee rate (satoshis per byte)
    pub fn fee_rate(mut self, rate: u64) -> Self {
        self.fee_rate = rate;
        self
    }

    /// Set the locktime
    pub fn locktime(mut self, locktime: u32) -> Self {
        self.locktime = locktime;
        self
    }

    /// Add an output to the transaction
    pub fn add_output(mut self, address: String, amount: u64) -> Self {
        self.outputs.push(TxOutput { address, amount });
        self
    }

    /// Build and sign the transaction
    pub fn build(mut self, wallet: &Wallet, utxo_set: &UTXOSet) -> Result<Transaction> {
        // Calculate total output amount
        let total_output: u64 = self.outputs.iter().map(|o| o.amount).sum();
        
        // Estimate transaction size for fee calculation
        let estimated_size = self.estimate_transaction_size();
        let estimated_fee = estimated_size * self.fee_rate;
        let total_needed = total_output + estimated_fee;

        // Select UTXOs to cover the amount
        let selected_utxos = utxo_set.select_utxos_for_amount(&wallet.address, total_needed)?;

        // Calculate actual input value
        let input_value: u64 = selected_utxos.iter().map(|utxo| utxo.value()).sum();
        
        // Calculate change
        let actual_fee = estimated_fee; // For now, use estimated fee
        let change = input_value.saturating_sub(total_output + actual_fee);

        // Create transaction
        let mut tx = Transaction::new(1, Vec::new(), Vec::new());
        tx.locktime = self.locktime;

        // Add inputs
        for utxo in selected_utxos {
            let input = TransactionInput::new(
                utxo.tx_hash,
                utxo.output_index,
                Vec::new(), // Will be filled with signature
            );
            tx.inputs.push(input);
            self.inputs.push(TxInput {
                utxo,
                private_key: wallet.private_key,
            });
        }

        // Add outputs
        for output in &self.outputs {
            let script_pubkey = create_p2pkh_script(&output.address)?;
            tx.outputs.push(TransactionOutput::new(output.amount, script_pubkey));
        }

        // Add change output if necessary
        if change > 546 { // Dust threshold
            let change_script = create_p2pkh_script(&wallet.address)?;
            tx.outputs.push(TransactionOutput::new(change, change_script));
        }

        // Sign all inputs
        self.sign_transaction(&mut tx)?;

        Ok(tx)
    }

    /// Estimate transaction size in bytes
    fn estimate_transaction_size(&self) -> u64 {
        let input_count = self.inputs.len();
        let output_count = self.outputs.len() + 1; // +1 for potential change output
        
        // Rough estimation:
        // - Base transaction: 10 bytes
        // - Each input: ~150 bytes (outpoint + script_sig + sequence)
        // - Each output: ~34 bytes (value + script_pubkey)
        let size = 10 + (input_count * 150) + (output_count * 34);
        size as u64
    }

    /// Sign all inputs of the transaction
    fn sign_transaction(&self, tx: &mut Transaction) -> Result<()> {
        for (i, tx_input) in self.inputs.iter().enumerate() {
            let signature = self.create_signature(tx, i, &tx_input.utxo, &tx_input.private_key)?;
            let public_key = derive_public_key(&tx_input.private_key)?;
            
            // Create script_sig (simplified P2PKH)
            let mut script_sig = Vec::new();
            script_sig.push(signature.len() as u8); // Push signature
            script_sig.extend_from_slice(&signature);
            script_sig.push(public_key.len() as u8); // Push public key
            script_sig.extend_from_slice(&public_key);
            
            tx.inputs[i].script_sig = script_sig;
        }
        Ok(())
    }

    /// Create signature for a specific input
    fn create_signature(
        &self,
        tx: &Transaction,
        input_index: usize,
        utxo: &UTXO,
        private_key: &PrivateKey,
    ) -> Result<Vec<u8>> {
        // Create signature hash (simplified)
        let signature_hash = self.create_signature_hash(tx, input_index, &utxo.output.script_pubkey)?;
        
        // Sign with private key (simplified ECDSA)
        let signature = sign_hash(&signature_hash, private_key)?;
        
        // Add SIGHASH_ALL flag
        let mut sig_with_hashtype = signature;
        sig_with_hashtype.push(0x01); // SIGHASH_ALL
        
        Ok(sig_with_hashtype)
    }

    /// Create the signature hash for signing
    fn create_signature_hash(
        &self,
        tx: &Transaction,
        input_index: usize,
        script_code: &[u8],
    ) -> Result<Hash256> {
        // Use proper Bitcoin-style SIGHASH_ALL
        Ok(tx.calculate_signature_hash(input_index, script_code, 0x01))
    }
}

impl Default for TransactionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Transaction manager for handling transaction lifecycle
#[derive(Debug)]
pub struct TransactionManager {
    utxo_set: UTXOSet,
    pending_transactions: HashMap<String, PendingTransaction>,
}

/// Represents a pending (unconfirmed) transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingTransaction {
    pub id: Uuid,
    pub transaction: Transaction,
    pub fee: u64,
    pub created_at: DateTime<Utc>,
    pub broadcast_count: u32,
    pub last_broadcast: Option<DateTime<Utc>>,
}

impl TransactionManager {
    /// Create a new transaction manager
    pub fn new(utxo_set: UTXOSet) -> Self {
        Self {
            utxo_set,
            pending_transactions: HashMap::new(),
        }
    }

    /// Create a new transaction from wallet to address
    pub fn create_transaction(
        &mut self,
        from_wallet: &Wallet,
        to_address: &str,
        amount: u64,
        fee_rate: Option<u64>,
    ) -> Result<Transaction> {
        let builder = TransactionBuilder::new()
            .fee_rate(fee_rate.unwrap_or(1000))
            .add_output(to_address.to_string(), amount);

        let tx = builder.build(from_wallet, &self.utxo_set)?;
        
        // Validate the transaction
        self.utxo_set.validate_transaction(&tx)?;
        
        Ok(tx)
    }

    /// Create a coinbase transaction for mining
    pub async fn create_coinbase_transaction(
        &self,
        miner_address: &str,
        reward_amount: u64,
        block_height: u64,
    ) -> Result<Transaction> {
        // Create coinbase input with block height
        let mut coinbase_data = Vec::new();
        coinbase_data.extend_from_slice(&block_height.to_le_bytes());
        coinbase_data.extend_from_slice(b"mined by rust blockchain");
        
        let coinbase_input = TransactionInput::create_coinbase(coinbase_data);
        
        // Create output to miner
        let output_script = create_p2pkh_script(miner_address)?;
        let coinbase_output = TransactionOutput {
            value: reward_amount,
            script_pubkey: output_script,
        };
        
        // Create coinbase transaction
        let coinbase_tx = Transaction::new(
            1,
            vec![coinbase_input],
            vec![coinbase_output],
        );
        
        Ok(coinbase_tx)
    }

    /// Add a transaction to pending pool
    pub fn add_pending_transaction(&mut self, tx: Transaction) -> Result<Uuid> {
        let tx_id = Uuid::new_v4();
        let fee = self.utxo_set.calculate_fee(&tx)?;
        
        let pending = PendingTransaction {
            id: tx_id,
            transaction: tx,
            fee,
            created_at: Utc::now(),
            broadcast_count: 0,
            last_broadcast: None,
        };

        let tx_hash = hex::encode(pending.transaction.get_hash()?);
        self.pending_transactions.insert(tx_hash, pending);
        
        Ok(tx_id)
    }

    /// Get pending transaction by hash
    pub fn get_pending_transaction(&self, tx_hash: &str) -> Option<&PendingTransaction> {
        self.pending_transactions.get(tx_hash)
    }

    /// Confirm a transaction (move from pending to UTXO set)
    pub fn confirm_transaction(&mut self, tx_hash: &str, block_height: u32) -> Result<()> {
        if let Some(pending) = self.pending_transactions.remove(tx_hash) {
            self.utxo_set.add_transaction(&pending.transaction, block_height)?;
        }
        Ok(())
    }

    /// Get current UTXO set
    pub fn get_utxo_set(&self) -> &UTXOSet {
        &self.utxo_set
    }

    /// Get mutable UTXO set
    pub fn get_utxo_set_mut(&mut self) -> &mut UTXOSet {
        &mut self.utxo_set
    }

    /// Get balance for an address
    pub fn get_balance(&self, address: &str) -> u64 {
        self.utxo_set.get_balance(address)
    }

    /// Get current blockchain height
    pub fn get_current_height(&self) -> u64 {
        self.utxo_set.get_current_height()
    }

    /// List all pending transactions
    pub fn get_pending_transactions(&self) -> Vec<&PendingTransaction> {
        self.pending_transactions.values().collect()
    }

    /// Remove expired pending transactions
    pub fn cleanup_pending_transactions(&mut self, max_age_hours: i64) {
        let cutoff = Utc::now() - chrono::Duration::hours(max_age_hours);
        self.pending_transactions.retain(|_, tx| tx.created_at > cutoff);
    }
}

/// Create a P2PKH script for an address
pub fn create_p2pkh_script(address: &str) -> Result<Vec<u8>> {
    // Simplified P2PKH script creation
    // In a real implementation, this would decode the address properly
    
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

/// Derive public key from private key using secp256k1 ECDSA
fn derive_public_key(private_key: &PrivateKey) -> Result<Vec<u8>> {
    let pubkey = crate::crypto::derive_public_key(private_key)?;
    Ok(pubkey.to_vec())
}

/// Sign a hash with a private key using secp256k1 ECDSA
fn sign_hash(hash: &Hash256, private_key: &PrivateKey) -> Result<Vec<u8>> {
    crate::crypto::sign_hash(hash, private_key)
}

/// Verify an ECDSA signature
pub fn verify_signature(
    signature: &[u8],
    public_key: &[u8],
    hash: &Hash256,
) -> Result<bool> {
    if public_key.len() != 33 {
        return Ok(false);
    }
    
    crate::crypto::verify_signature(signature, public_key, hash)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wallet::Wallet;

    #[test]
    fn test_transaction_builder() {
        let wallet = Wallet::new("test_wallet".to_string()).unwrap();
        let utxo_set = UTXOSet::new();
        
        let builder = TransactionBuilder::new()
            .fee_rate(1000)
            .add_output("edu1qtest123".to_string(), 1000);
        
        // This will fail because no UTXOs, but tests the interface
        assert!(builder.build(&wallet, &utxo_set).is_err());
    }

    #[test]
    fn test_p2pkh_script_creation() {
        let address = "edu1q".to_owned() + &"a".repeat(40); // 40 hex chars = 20 bytes
        let script = create_p2pkh_script(&address).unwrap();
        
        assert_eq!(script.len(), 25);
        assert_eq!(script[0], 0x76); // OP_DUP
        assert_eq!(script[1], 0xa9); // OP_HASH160
        assert_eq!(script[2], 0x14); // Push 20 bytes
    }

    #[test]
    fn test_signature_creation() {
        let private_key = [1u8; 32];
        let hash = [2u8; 32];
        
        let signature = sign_hash(&hash, &private_key).unwrap();
        assert_eq!(signature.len(), 64);
        
        let public_key = derive_public_key(&private_key).unwrap();
        assert!(verify_signature(&signature, &public_key, &hash).unwrap());
    }
}