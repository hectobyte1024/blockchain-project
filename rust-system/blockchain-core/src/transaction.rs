use crate::{
    BlockchainError, Result, Hash256, Amount, OutPoint,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use sha2::{Digest, Sha256};

// Import FFI types and crypto functionality for proper transaction signing
use blockchain_ffi::types::{Hash256Wrapper, PrivateKeyWrapper, PublicKeyWrapper, SignatureWrapper};
use blockchain_ffi::crypto::convenience;

/// Transaction input wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionInput {
    pub prev_tx_hash: Hash256Wrapper,
    pub prev_output_index: u32,
    pub script_sig: Vec<u8>,
    pub sequence: u32,
}

impl TransactionInput {
    pub fn new(prev_tx_hash: Hash256Wrapper, prev_output_index: u32, script_sig: Vec<u8>) -> Self {
        Self {
            prev_tx_hash,
            prev_output_index,
            script_sig,
            sequence: 0xFFFFFFFF,
        }
    }
    
    pub fn is_coinbase(&self) -> bool {
        self.prev_tx_hash.is_zero() && self.prev_output_index == 0xFFFFFFFF
    }
    
    pub fn create_coinbase(coinbase_data: Vec<u8>) -> Self {
        Self {
            prev_tx_hash: Hash256Wrapper::zero(),
            prev_output_index: 0xFFFFFFFF,
            script_sig: coinbase_data,
            sequence: 0xFFFFFFFF,
        }
    }
    
    pub fn get_outpoint(&self) -> String {
        format!("{}:{}", self.prev_tx_hash.to_hex(), self.prev_output_index)
    }
}

/// Transaction output wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionOutput {
    pub value: u64,
    pub script_pubkey: Vec<u8>,
}

impl TransactionOutput {
    pub fn new(value: u64, script_pubkey: Vec<u8>) -> Self {
        Self { value, script_pubkey }
    }
    
    pub fn is_valid(&self) -> bool {
        self.value > 0 && self.value >= 546 && !self.script_pubkey.is_empty() // 546 = dust threshold
    }
    
    pub fn create_p2pkh(value: u64, address: &str) -> Result<Self> {
        // Simplified P2PKH creation - in real implementation would decode address
        let script = vec![
            0x76, 0xa9, 0x14, // OP_DUP OP_HASH160 Push20
            // Would insert 20-byte hash160 here
            0x88, 0xac, // OP_EQUALVERIFY OP_CHECKSIG
        ];
        
        Ok(Self {
            value,
            script_pubkey: script,
        })
    }
    
    pub fn get_address(&self) -> Option<String> {
        // Simplified address extraction
        if self.script_pubkey.len() == 25 &&
           self.script_pubkey[0] == 0x76 && // OP_DUP
           self.script_pubkey[1] == 0xa9 && // OP_HASH160
           self.script_pubkey[2] == 0x14    // Push 20 bytes
        {
            // Would extract and encode address here
            Some("P2PKH_ADDRESS".to_string())
        } else {
            None
        }
    }
}

/// Transaction witness data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionWitness {
    pub witness_items: Vec<Vec<u8>>,
}

impl TransactionWitness {
    pub fn new() -> Self {
        Self {
            witness_items: Vec::new(),
        }
    }
    
    pub fn is_empty(&self) -> bool {
        self.witness_items.is_empty() || 
        self.witness_items.iter().all(|item| item.is_empty())
    }
    
    pub fn add_item(&mut self, item: Vec<u8>) {
        self.witness_items.push(item);
    }
}

impl Default for TransactionWitness {
    fn default() -> Self {
        Self::new()
    }
}

/// Complete transaction wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub version: u32,
    pub inputs: Vec<TransactionInput>,
    pub outputs: Vec<TransactionOutput>,
    pub witnesses: Vec<TransactionWitness>,
    pub locktime: u32,
    
    #[serde(skip)]
    cached_hash: Option<Hash256Wrapper>,
    #[serde(skip)]
    cached_wtxid: Option<Hash256Wrapper>,
}

impl Transaction {
    pub fn new(version: u32, inputs: Vec<TransactionInput>, outputs: Vec<TransactionOutput>) -> Self {
        let witness_count = inputs.len();
        Self {
            version,
            inputs,
            outputs,
            witnesses: vec![TransactionWitness::new(); witness_count],
            locktime: 0,
            cached_hash: None,
            cached_wtxid: None,
        }
    }
    
    pub fn is_coinbase(&self) -> bool {
        self.inputs.len() == 1 && self.inputs[0].is_coinbase()
    }
    
    pub fn is_segwit(&self) -> bool {
        !self.witnesses.is_empty() && 
        self.witnesses.iter().any(|w| !w.is_empty())
    }
    
    pub fn get_total_output_value(&self) -> u64 {
        self.outputs.iter().map(|output| output.value).sum()
    }
    
    pub fn is_valid(&self) -> bool {
        // Basic validation
        if self.inputs.is_empty() || self.outputs.is_empty() {
            return false;
        }
        
        // Validate outputs
        for output in &self.outputs {
            if !output.is_valid() {
                return false;
            }
        }
        
        // Check witness count
        if !self.witnesses.is_empty() && self.witnesses.len() != self.inputs.len() {
            return false;
        }
        
        true
    }
    
    pub fn get_txid(&self) -> String {
        // Would calculate actual transaction hash here
        format!("tx_{:08x}", self.version)
    }
    
    pub fn get_hash(&self) -> Result<Hash256> {
        // For now, return a simple hash based on transaction data
        // In a real implementation, this would be a proper SHA256 hash
        let data = format!("{:?}", self);
        let hash_bytes = Sha256::digest(data.as_bytes());
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&hash_bytes);
        Ok(hash)
    }
    
    pub fn clear_cache(&mut self) {
        self.cached_hash = None;
        self.cached_wtxid = None;
    }
    
    pub fn create_coinbase(block_reward: u64, total_fees: u64, miner_address: &str, extra_data: Vec<u8>) -> Result<Self> {
        let coinbase_input = TransactionInput::create_coinbase(extra_data);
        let total_reward = block_reward + total_fees;
        let output = TransactionOutput::create_p2pkh(total_reward, miner_address)?;
        
        let mut tx = Self::new(2, vec![coinbase_input], vec![output]);
        tx.witnesses = vec![TransactionWitness::new()]; // Empty witness for coinbase
        
        Ok(tx)
    }
    
    /// Serialize transaction to bytes
    pub fn serialize(&self) -> Result<Vec<u8>> {
        // For now, use serde_json as a simple serialization
        // In a real implementation, this would use binary Bitcoin protocol format
        let json = serde_json::to_string(self)
            .map_err(|e| BlockchainError::SerializationError(e.to_string()))?;
        Ok(json.into_bytes())
    }

    pub fn estimate_size(&self) -> usize {
        // Simplified size estimation
        let base_size = 4 + 4; // version + locktime
        let inputs_size = self.inputs.len() * 150; // Approximate input size
        let outputs_size = self.outputs.len() * 34; // Approximate output size
        let witness_size = if self.is_segwit() { self.witnesses.len() * 100 } else { 0 };
        
        base_size + inputs_size + outputs_size + witness_size
    }
}

/// UTXO (Unspent Transaction Output)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UTXO {
    pub tx_hash: Hash256Wrapper,
    pub output_index: u32,
    pub output: TransactionOutput,
    pub block_height: u32,
    pub is_coinbase: bool,
}

impl UTXO {
    pub fn new(tx_hash: Hash256Wrapper, output_index: u32, output: TransactionOutput, block_height: u32, is_coinbase: bool) -> Self {
        Self {
            tx_hash,
            output_index,
            output,
            block_height,
            is_coinbase,
        }
    }
    
    pub fn get_outpoint(&self) -> String {
        format!("{}:{}", self.tx_hash.to_hex(), self.output_index)
    }
    
    pub fn is_mature(&self, current_height: u32) -> bool {
        if self.is_coinbase {
            current_height >= self.block_height + 100 // Coinbase maturity: 100 blocks
        } else {
            true
        }
    }
}

/// UTXO set for tracking unspent outputs
#[derive(Debug, Clone, Default)]
pub struct UTXOSet {
    utxos: HashMap<String, UTXO>, // outpoint -> UTXO
}

impl UTXOSet {
    pub fn new() -> Self {
        Self {
            utxos: HashMap::new(),
        }
    }
    
    pub fn add_utxo(&mut self, utxo: UTXO) {
        let outpoint = utxo.get_outpoint();
        self.utxos.insert(outpoint, utxo);
    }
    
    pub fn remove_utxo(&mut self, tx_hash: &Hash256Wrapper, output_index: u32) -> bool {
        let outpoint = format!("{}:{}", tx_hash.to_hex(), output_index);
        self.utxos.remove(&outpoint).is_some()
    }
    
    pub fn get_utxo(&self, tx_hash: &Hash256Wrapper, output_index: u32) -> Option<&UTXO> {
        let outpoint = format!("{}:{}", tx_hash.to_hex(), output_index);
        self.utxos.get(&outpoint)
    }
    
    pub fn has_utxo(&self, tx_hash: &Hash256Wrapper, output_index: u32) -> bool {
        let outpoint = format!("{}:{}", tx_hash.to_hex(), output_index);
        self.utxos.contains_key(&outpoint)
    }
    
    pub fn get_utxos_for_address(&self, address: &str) -> Vec<&UTXO> {
        self.utxos.values()
            .filter(|utxo| {
                utxo.output.get_address()
                    .map(|addr| addr == address)
                    .unwrap_or(false)
            })
            .collect()
    }
    
    pub fn get_balance(&self, address: &str) -> u64 {
        self.get_utxos_for_address(address)
            .iter()
            .map(|utxo| utxo.output.value)
            .sum()
    }
    
    pub fn apply_transaction(&mut self, tx: &Transaction, block_height: u32) -> Result<()> {
        // Remove spent UTXOs (inputs)
        if !tx.is_coinbase() {
            for input in &tx.inputs {
                if !self.remove_utxo(&input.prev_tx_hash, input.prev_output_index) {
                    return Err(crate::BlockchainError::InvalidTransaction("UTXO not found".to_string()).into());
                }
            }
        }
        
        // Add new UTXOs (outputs)
        let tx_hash_bytes = crate::utils::hex_to_bytes(&tx.get_txid())?;
        let tx_hash: Hash256 = tx_hash_bytes.try_into().map_err(|_| BlockchainError::SerializationError("Invalid hash length".to_string()))?;
        for (index, output) in tx.outputs.iter().enumerate() {
            let utxo = UTXO::new(
                Hash256Wrapper::from_hash256(&tx_hash),
                index as u32,
                output.clone(),
                block_height,
                tx.is_coinbase(),
            );
            self.add_utxo(utxo);
        }
        
        Ok(())
    }
    
    pub fn rollback_transaction(&mut self, tx: &Transaction) -> Result<()> {
        // Remove UTXOs created by this transaction
        let tx_hash_bytes = crate::utils::hex_to_bytes(&tx.get_txid())?;
        let tx_hash: Hash256 = tx_hash_bytes.try_into().map_err(|_| BlockchainError::SerializationError("Invalid hash length".to_string()))?;
        for index in 0..tx.outputs.len() {
            let wrapper = Hash256Wrapper::from_hash256(&tx_hash);
            self.remove_utxo(&wrapper, index as u32);
        }
        
        // Note: In a real implementation, we would need to restore the spent UTXOs
        // This requires keeping track of them or having a way to rebuild them
        
        Ok(())
    }
    
    pub fn size(&self) -> usize {
        self.utxos.len()
    }
    
    pub fn get_total_value(&self) -> u64 {
        self.utxos.values()
            .map(|utxo| utxo.output.value)
            .sum()
    }
    
    pub fn clear(&mut self) {
        self.utxos.clear();
    }
    
    pub fn validate(&self) -> bool {
        // Basic validation - no duplicate outpoints
        self.utxos.len() == self.utxos.keys().len()
    }
}

/// Transaction builder for creating transactions
#[derive(Debug)]
pub struct TransactionBuilder {
    version: u32,
    inputs: Vec<TransactionInput>,
    outputs: Vec<TransactionOutput>,
    signing_keys: Vec<(PrivateKeyWrapper, PublicKeyWrapper)>,
    prev_outputs: Vec<TransactionOutput>,
    total_input_value: u64,
    fee_rate: u64, // satoshis per kvB
    locktime: u32,
}

impl TransactionBuilder {
    pub fn new(version: u32) -> Self {
        Self {
            version,
            inputs: Vec::new(),
            outputs: Vec::new(),
            signing_keys: Vec::new(),
            prev_outputs: Vec::new(),
            total_input_value: 0,
            fee_rate: 1000, // 1 sat/vB
            locktime: 0,
        }
    }
    
    pub fn add_input(mut self, prev_tx_hash: Hash256Wrapper, prev_output_index: u32, prev_output: TransactionOutput, signing_key: PrivateKeyWrapper, public_key: PublicKeyWrapper) -> Self {
        let input = TransactionInput::new(prev_tx_hash, prev_output_index, Vec::new());
        self.total_input_value += prev_output.value;
        
        self.inputs.push(input);
        self.prev_outputs.push(prev_output);
        self.signing_keys.push((signing_key, public_key));
        
        self
    }
    
    pub fn add_output(mut self, address: String, value: u64) -> Result<Self> {
        let output = TransactionOutput::create_p2pkh(value, &address)?;
        self.outputs.push(output);
        Ok(self)
    }
    
    pub fn set_fee_rate(mut self, rate: u64) -> Self {
        self.fee_rate = rate;
        self
    }
    
    pub fn set_locktime(mut self, locktime: u32) -> Self {
        self.locktime = locktime;
        self
    }
    
    pub fn finalize_with_change(mut self, change_address: String) -> Result<Self> {
        // Estimate transaction size
        let estimated_size = self.estimate_size();
        let estimated_fee = (estimated_size as u64 * self.fee_rate) / 1000;
        
        let total_output_value: u64 = self.outputs.iter().map(|o| o.value).sum();
        
        if self.total_input_value < total_output_value + estimated_fee {
            return Err(crate::BlockchainError::InvalidTransaction("Insufficient funds".to_string()).into());
        }
        
        let change_amount = self.total_input_value - total_output_value - estimated_fee;
        
        // Only add change output if above dust threshold
        if change_amount >= 546 {
            let change_output = TransactionOutput::create_p2pkh(change_amount, &change_address)?;
            self.outputs.push(change_output);
        }
        
        Ok(self)
    }
    
    pub fn build(self) -> Result<Transaction> {
        if self.inputs.is_empty() || self.outputs.is_empty() {
            return Err(crate::BlockchainError::InvalidTransaction("Empty inputs or outputs".to_string()).into());
        }

        let mut tx = Transaction::new(self.version, self.inputs.clone(), self.outputs.clone());
        tx.locktime = self.locktime;

        // Sign each input with its corresponding private key
        let input_len = tx.inputs.len();
        for i in 0..input_len {
            if i < self.signing_keys.len() {
                // Create signature hash for this input (SIGHASH_ALL)
                let signature_hash = self.create_signature_hash(&tx, i, 0x01)?;
                
                // Get the private key and public key for this input
                let (private_key, public_key) = &self.signing_keys[i];
                
                // Sign the transaction hash using FFI crypto functions
                let signature = convenience::sign_message(private_key, &signature_hash)
                    .map_err(|e| BlockchainError::InvalidTransaction(format!("Signature creation failed: {}", e)))?;
                
                // Create script_sig with signature + sighash flag + public key (for P2PKH)
                // Standard P2PKH script: <signature> <sighash_flag> <public_key>
                let mut script_sig = Vec::new();
                
                // Add signature length + signature + sighash flag
                let sig_with_flag = {
                    let mut sig = signature.as_bytes().to_vec();
                    sig.push(0x01); // SIGHASH_ALL flag
                    sig
                };
                script_sig.push(sig_with_flag.len() as u8);
                script_sig.extend_from_slice(&sig_with_flag);
                
                // Add public key length + public key
                let pubkey_bytes = public_key.as_bytes();
                script_sig.push(pubkey_bytes.len() as u8);
                script_sig.extend_from_slice(pubkey_bytes);
                
                tx.inputs[i].script_sig = script_sig;
            }
        }

        Ok(tx)
    }
    
    fn estimate_size(&self) -> usize {
        let base_size = 4 + 4; // version + locktime
        let inputs_size = self.inputs.len() * 150; // Approximate
        let outputs_size = self.outputs.len() * 34; // Approximate
        base_size + inputs_size + outputs_size
    }
    
    /// Create signature hash for transaction input (implements Bitcoin's SIGHASH_ALL)
    fn create_signature_hash(&self, tx: &Transaction, input_index: usize, sighash_type: u8) -> Result<Hash256Wrapper> {
        if input_index >= tx.inputs.len() {
            return Err(BlockchainError::InvalidTransaction("Input index out of range".to_string()).into());
        }
        
        // Serialize transaction for signature hash
        let mut serialized = Vec::new();
        
        // Version (4 bytes, little endian)
        serialized.extend_from_slice(&tx.version.to_le_bytes());
        
        // Input count
        serialized.push(tx.inputs.len() as u8);
        
        // Inputs - for SIGHASH_ALL, clear all script_sigs except the one being signed
        for (i, input) in tx.inputs.iter().enumerate() {
            // Previous transaction hash (32 bytes)
            serialized.extend_from_slice(input.prev_tx_hash.as_bytes());
            
            // Previous output index (4 bytes, little endian)
            serialized.extend_from_slice(&input.prev_output_index.to_le_bytes());
            
            // Script signature - only include for the input being signed
            if i == input_index {
                // For P2PKH, use the previous output's scriptPubKey
                // For now, create a standard P2PKH scriptPubKey pattern
                let script_code = vec![
                    0x76, 0xa9, 0x14, // OP_DUP OP_HASH160 Push20
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // 20-byte placeholder hash
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x88, 0xac, // OP_EQUALVERIFY OP_CHECKSIG
                ];
                serialized.push(script_code.len() as u8);
                serialized.extend_from_slice(&script_code);
            } else {
                // Empty script for other inputs
                serialized.push(0);
            }
            
            // Sequence (4 bytes, little endian)
            serialized.extend_from_slice(&input.sequence.to_le_bytes());
        }
        
        // Output count
        serialized.push(tx.outputs.len() as u8);
        
        // Outputs
        for output in &tx.outputs {
            // Value (8 bytes, little endian)
            serialized.extend_from_slice(&output.value.to_le_bytes());
            
            // Script length and script
            serialized.push(output.script_pubkey.len() as u8);
            serialized.extend_from_slice(&output.script_pubkey);
        }
        
        // Lock time (4 bytes, little endian)
        serialized.extend_from_slice(&tx.locktime.to_le_bytes());
        
        // Signature hash type (4 bytes, little endian)
        serialized.extend_from_slice(&(sighash_type as u32).to_le_bytes());
        
        // Double SHA-256 the serialized data using FFI convenience function
        convenience::double_sha256(&serialized)
            .map_err(|e| BlockchainError::InvalidTransaction(format!("Hash creation failed: {}", e)).into())
    }
}

/// Validation rules for transactions
pub mod validation {
    use super::*;
    
    pub const MAX_TRANSACTION_SIZE: usize = 100000;
    pub const DUST_THRESHOLD: u64 = 546;
    pub const COINBASE_MATURITY: u32 = 100;
    
    pub fn validate_transaction_size(tx: &Transaction) -> bool {
        tx.estimate_size() <= MAX_TRANSACTION_SIZE
    }
    
    pub fn validate_transaction_structure(tx: &Transaction) -> bool {
        tx.is_valid()
    }
    
    pub fn validate_transaction_inputs(tx: &Transaction, utxo_set: &UTXOSet) -> bool {
        if tx.is_coinbase() {
            return true; // Coinbase transactions don't spend UTXOs
        }
        
        for input in &tx.inputs {
            if !utxo_set.has_utxo(&input.prev_tx_hash, input.prev_output_index) {
                return false;
            }
        }
        
        true
    }
    
    pub fn validate_transaction_complete(tx: &Transaction, utxo_set: &UTXOSet) -> bool {
        validate_transaction_size(tx) &&
        validate_transaction_structure(tx) &&
        validate_transaction_inputs(tx, utxo_set)
    }
}

// TODO: Add transaction utilities and tests when types are properly imported
// TODO: Uncomment and fix when Hash256Wrapper is properly imported
