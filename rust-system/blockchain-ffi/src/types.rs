//! Safe Rust wrappers for C++ data types

use crate::{error::Result, BlockchainError};
use std::fmt;
use std::mem::MaybeUninit;

/// Safe wrapper for Hash256
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Hash256Wrapper(pub [u8; 32]);

impl Hash256Wrapper {
    /// Create from raw bytes
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
    
    /// Create from slice (with length check)
    pub fn from_slice(slice: &[u8]) -> Result<Self> {
        if slice.len() != 32 {
            return Err(BlockchainError::InvalidInput(
                format!("Hash256 requires exactly 32 bytes, got {}", slice.len())
            ));
        }
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(slice);
        Ok(Self(bytes))
    }
    
    /// Get as byte slice
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
    
    /// Convert to FFI type
    pub fn to_ffi(&self) -> crate::Hash256 {
        crate::Hash256 { data: self.0 }
    }
    
    /// Create from FFI type
    pub fn from_ffi(ffi: &crate::Hash256) -> Self {
        Self(ffi.data)
    }
    
    /// Create zero hash
    pub fn zero() -> Self {
        Self([0u8; 32])
    }
    
    /// Check if hash is zero
    pub fn is_zero(&self) -> bool {
        self.0.iter().all(|&b| b == 0)
    }
}

impl fmt::Display for Hash256Wrapper {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

impl From<[u8; 32]> for Hash256Wrapper {
    fn from(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

impl AsRef<[u8]> for Hash256Wrapper {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl Hash256Wrapper {
    /// Convert to hex string (needed for compatibility)
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }
    
    /// Convert from internal Hash256 ([u8; 32]) to Hash256Wrapper
    pub fn from_hash256(hash: &[u8; 32]) -> Self {
        Self(*hash)
    }
    
    /// Convert to internal Hash256 ([u8; 32])
    pub fn to_hash256(&self) -> [u8; 32] {
        self.0
    }
}

/// Safe wrapper for Hash160
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Hash160Wrapper(pub [u8; 20]);

impl Hash160Wrapper {
    /// Create from raw bytes
    pub fn from_bytes(bytes: [u8; 20]) -> Self {
        Self(bytes)
    }
    
    /// Create from slice (with length check)
    pub fn from_slice(slice: &[u8]) -> Result<Self> {
        if slice.len() != 20 {
            return Err(BlockchainError::InvalidInput(
                format!("Hash160 requires exactly 20 bytes, got {}", slice.len())
            ));
        }
        let mut bytes = [0u8; 20];
        bytes.copy_from_slice(slice);
        Ok(Self(bytes))
    }
    
    /// Get as byte slice
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
    
    /// Convert to FFI type
    pub fn to_ffi(&self) -> crate::Hash160 {
        crate::Hash160 { data: self.0 }
    }
    
    /// Create from FFI type
    pub fn from_ffi(ffi: &crate::Hash160) -> Self {
        Self(ffi.data)
    }
}

impl fmt::Display for Hash160Wrapper {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

/// Safe wrapper for PrivateKey
#[derive(Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PrivateKeyWrapper([u8; 32]);

impl PrivateKeyWrapper {
    /// Create from raw bytes
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
    
    /// Create from slice (with length check)
    pub fn from_slice(slice: &[u8]) -> Result<Self> {
        if slice.len() != 32 {
            return Err(BlockchainError::InvalidInput(
                format!("Private key requires exactly 32 bytes, got {}", slice.len())
            ));
        }
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(slice);
        Ok(Self(bytes))
    }
    
    /// Get as byte slice
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
    
    /// Create from blockchain-core PrivateKey ([u8; 32])
    pub fn from_private_key(private_key: &[u8; 32]) -> Self {
        Self(*private_key)
    }
    
    /// Convert to FFI type
    pub fn to_ffi(&self) -> crate::PrivateKey {
        crate::PrivateKey { data: self.0 }
    }
    
    /// Create from FFI type
    pub fn from_ffi(ffi: &crate::PrivateKey) -> Self {
        Self(ffi.data)
    }
}

impl fmt::Debug for PrivateKeyWrapper {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PrivateKey([HIDDEN])")
    }
}

impl Drop for PrivateKeyWrapper {
    fn drop(&mut self) {
        // Zeroize the private key data when dropped
        self.0.fill(0);
    }
}

/// Safe wrapper for PublicKey
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PublicKeyWrapper(pub [u8; 33]);

impl PublicKeyWrapper {
    /// Create from raw bytes
    pub fn from_bytes(bytes: [u8; 33]) -> Self {
        Self(bytes)
    }
    
    /// Create from slice (with length check)
    pub fn from_slice(slice: &[u8]) -> Result<Self> {
        if slice.len() != 33 {
            return Err(BlockchainError::InvalidInput(
                format!("Public key requires exactly 33 bytes, got {}", slice.len())
            ));
        }
        let mut bytes = [0u8; 33];
        bytes.copy_from_slice(slice);
        Ok(Self(bytes))
    }
    
    /// Get as byte slice
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
    
    /// Convert to FFI type
    pub fn to_ffi(&self) -> crate::PublicKey {
        crate::PublicKey { data: self.0 }
    }
    
    /// Create from FFI type
    pub fn from_ffi(ffi: &crate::PublicKey) -> Self {
        Self(ffi.data)
    }
}

impl fmt::Display for PublicKeyWrapper {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

/// Safe wrapper for Signature
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SignatureWrapper(pub [u8; 64]);

impl SignatureWrapper {
    /// Create from raw bytes
    pub fn from_bytes(bytes: [u8; 64]) -> Self {
        Self(bytes)
    }
    
    /// Create from slice (with length check)
    pub fn from_slice(slice: &[u8]) -> Result<Self> {
        if slice.len() != 64 {
            return Err(BlockchainError::InvalidInput(
                format!("Signature requires exactly 64 bytes, got {}", slice.len())
            ));
        }
        let mut bytes = [0u8; 64];
        bytes.copy_from_slice(slice);
        Ok(Self(bytes))
    }
    
    /// Get as byte slice
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
    
    /// Convert to Vec<u8>
    pub fn to_vec(&self) -> Vec<u8> {
        self.0.to_vec()
    }
    
    /// Convert to FFI type
    pub fn to_ffi(&self) -> crate::Signature {
        crate::Signature { data: self.0 }
    }
    
    /// Create from FFI type
    pub fn from_ffi(ffi: &crate::Signature) -> Self {
        Self(ffi.data)
    }
}

// Custom serde implementations for types with large arrays
impl serde::Serialize for PublicKeyWrapper {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_bytes(&self.0)
    }
}

impl<'de> serde::Deserialize<'de> for PublicKeyWrapper {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let bytes: Vec<u8> = serde::Deserialize::deserialize(deserializer)?;
        if bytes.len() != 33 {
            return Err(serde::de::Error::custom(format!(
                "PublicKey must be exactly 33 bytes, got {}",
                bytes.len()
            )));
        }
        let mut array = [0u8; 33];
        array.copy_from_slice(&bytes);
        Ok(PublicKeyWrapper(array))
    }
}

impl serde::Serialize for SignatureWrapper {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_bytes(&self.0)
    }
}

impl<'de> serde::Deserialize<'de> for SignatureWrapper {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let bytes: Vec<u8> = serde::Deserialize::deserialize(deserializer)?;
        if bytes.len() != 64 {
            return Err(serde::de::Error::custom(format!(
                "Signature must be exactly 64 bytes, got {}",
                bytes.len()
            )));
        }
        let mut array = [0u8; 64];
        array.copy_from_slice(&bytes);
        Ok(SignatureWrapper(array))
    }
}

/// Safe wrapper for OutPoint
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OutPointWrapper {
    pub txid: Hash256Wrapper,
    pub vout: u32,
}

impl OutPointWrapper {
    /// Create new outpoint
    pub fn new(txid: Hash256Wrapper, vout: u32) -> Self {
        Self { txid, vout }
    }
    
    /// Convert to FFI type
    pub fn to_ffi(&self) -> crate::OutPoint {
        crate::OutPoint {
            txid: self.txid.to_ffi(),
            vout: self.vout,
        }
    }
    
    /// Create from FFI type
    pub fn from_ffi(ffi: &crate::OutPoint) -> Self {
        Self {
            txid: Hash256Wrapper::from_ffi(&ffi.txid),
            vout: ffi.vout,
        }
    }
    
    /// Check if this is a null outpoint (coinbase input)
    pub fn is_null(&self) -> bool {
        self.txid.is_zero() && self.vout == u32::MAX
    }
}

impl fmt::Display for OutPointWrapper {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.txid, self.vout)
    }
}

/// Safe wrapper for Transaction
#[derive(Debug, Clone)]
pub struct TransactionWrapper {
    pub version: u32,
    pub inputs: Vec<TransactionInputWrapper>,
    pub outputs: Vec<TransactionOutputWrapper>,
    pub lock_time: u32,
}

impl TransactionWrapper {
    /// Create new transaction
    pub fn new() -> Self {
        Self {
            version: 1,
            inputs: Vec::new(),
            outputs: Vec::new(),
            lock_time: 0,
        }
    }
    
    /// Calculate transaction hash
    pub fn calculate_hash(&self) -> Result<Hash256Wrapper> {
        // This would serialize and hash through C++
        // For now, return placeholder
        Ok(Hash256Wrapper::zero())
    }
    
    /// Convert to FFI type (complex conversion)
    pub fn to_ffi(&self) -> Result<crate::Transaction> {
        // This requires careful memory management
        // Implementation would allocate C arrays and populate them
        todo!("Complex FFI conversion - implement when needed")
    }
}

impl Default for TransactionWrapper {
    fn default() -> Self {
        Self::new()
    }
}

/// Safe wrapper for TransactionInput
#[derive(Debug, Clone)]
pub struct TransactionInputWrapper {
    pub previous_output: OutPointWrapper,
    pub script_sig: Vec<u8>,
    pub sequence: u32,
}

impl TransactionInputWrapper {
    /// Create new transaction input
    pub fn new(previous_output: OutPointWrapper, script_sig: Vec<u8>) -> Self {
        Self {
            previous_output,
            script_sig,
            sequence: 0xFFFFFFFF,
        }
    }
}

/// Safe wrapper for TransactionOutput  
#[derive(Debug, Clone)]
pub struct TransactionOutputWrapper {
    pub value: u64,
    pub script_pubkey: Vec<u8>,
}

impl TransactionOutputWrapper {
    /// Create new transaction output
    pub fn new(value: u64, script_pubkey: Vec<u8>) -> Self {
        Self { value, script_pubkey }
    }
    
    /// Convert to FFI type
    pub fn to_ffi(&self) -> crate::TransactionOutput {
        // Create ByteBuffer for script
        let script_buffer = crate::ByteBuffer {
            data: self.script_pubkey.as_ptr() as *mut u8,
            size: self.script_pubkey.len(),
            capacity: self.script_pubkey.len(),
        };
        
        crate::TransactionOutput {
            value: self.value,
            script_pubkey: script_buffer,
        }
    }
    
    /// Create from FFI type
    pub fn from_ffi(ffi_output: &crate::TransactionOutput) -> Self {
        let script_pubkey = if ffi_output.script_pubkey.size > 0 && !ffi_output.script_pubkey.data.is_null() {
            unsafe {
                std::slice::from_raw_parts(ffi_output.script_pubkey.data, ffi_output.script_pubkey.size).to_vec()
            }
        } else {
            Vec::new()
        };
        
        Self {
            value: ffi_output.value,
            script_pubkey,
        }
    }
}

/// Safe wrapper for BlockHeader
#[derive(Debug, Clone)]
pub struct BlockHeaderWrapper {
    pub version: u32,
    pub previous_block_hash: Hash256Wrapper,
    pub merkle_root: Hash256Wrapper,
    pub timestamp: u64,
    pub difficulty_target: u32,
    pub nonce: u64,
}

impl BlockHeaderWrapper {
    /// Create new block header
    pub fn new() -> Self {
        Self {
            version: 1,
            previous_block_hash: Hash256Wrapper::zero(),
            merkle_root: Hash256Wrapper::zero(),
            timestamp: 0,
            difficulty_target: 0,
            nonce: 0,
        }
    }
    
    /// Convert to FFI type
    pub fn to_ffi(&self) -> crate::BlockHeader {
        crate::BlockHeader {
            version: self.version,
            previous_block_hash: self.previous_block_hash.to_ffi(),
            merkle_root: self.merkle_root.to_ffi(),
            timestamp: self.timestamp,
            difficulty_target: self.difficulty_target,
            nonce: self.nonce,
        }
    }
    
    /// Create from FFI type
    pub fn from_ffi(ffi: &crate::BlockHeader) -> Self {
        Self {
            version: ffi.version,
            previous_block_hash: Hash256Wrapper::from_ffi(&ffi.previous_block_hash),
            merkle_root: Hash256Wrapper::from_ffi(&ffi.merkle_root),
            timestamp: ffi.timestamp,
            difficulty_target: ffi.difficulty_target,
            nonce: ffi.nonce,
        }
    }
}

impl Default for BlockHeaderWrapper {
    fn default() -> Self {
        Self::new()
    }
}

/// Safe wrapper for Block
#[derive(Debug, Clone)]
pub struct BlockWrapper {
    pub header: BlockHeaderWrapper,
    pub transactions: Vec<TransactionWrapper>,
}

impl BlockWrapper {
    /// Create new block
    pub fn new(header: BlockHeaderWrapper) -> Self {
        Self {
            header,
            transactions: Vec::new(),
        }
    }
    
    /// Add transaction to block
    pub fn add_transaction(&mut self, tx: TransactionWrapper) {
        self.transactions.push(tx);
    }
    
    /// Get block hash (same as header hash)
    pub fn calculate_hash(&self) -> Result<Hash256Wrapper> {
        // This would hash the header through C++
        Ok(Hash256Wrapper::zero())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_hash256_wrapper() {
        let bytes = [1u8; 32];
        let hash = Hash256Wrapper::from_bytes(bytes);
        assert_eq!(hash.as_bytes(), &bytes);
        assert!(!hash.is_zero());
        
        let zero = Hash256Wrapper::zero();
        assert!(zero.is_zero());
    }
    
    #[test]
    fn test_outpoint_wrapper() {
        let txid = Hash256Wrapper::from_bytes([1u8; 32]);
        let outpoint = OutPointWrapper::new(txid, 0);
        assert_eq!(outpoint.txid, txid);
        assert_eq!(outpoint.vout, 0);
        assert!(!outpoint.is_null());
        
        let null_outpoint = OutPointWrapper::new(Hash256Wrapper::zero(), u32::MAX);
        assert!(null_outpoint.is_null());
    }
    
    #[test]
    fn test_private_key_zeroization() {
        let mut key = PrivateKeyWrapper::from_bytes([42u8; 32]);
        assert_eq!(key.as_bytes()[0], 42);
        drop(key);
        // Key should be zeroized after drop (we can't test this directly)
    }
}