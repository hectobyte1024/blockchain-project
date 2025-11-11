//! Safe Rust wrapper for C++ storage engine

use crate::{
    error::{check_result, Result},
    types::{BlockWrapper, TransactionWrapper, Hash256Wrapper, OutPointWrapper, TransactionOutputWrapper},
};
use std::ffi::CString;

/// Safe wrapper around C++ StorageEngine
pub struct StorageEngine {
    inner: *mut crate::StorageEngine,
}

impl StorageEngine {
    /// Create new storage engine with database path
    pub fn new(database_path: &str) -> Result<Self> {
        let path_cstring = CString::new(database_path)
            .map_err(|_| crate::error::BlockchainError::InvalidInput("Invalid database path".to_string()))?;
            
        let inner = unsafe { crate::storage_engine_new(path_cstring.as_ptr()) };
        if inner.is_null() {
            return Err(crate::error::BlockchainError::StorageError("Failed to create storage engine".to_string()));
        }
        Ok(Self { inner })
    }
    
    /// Store a block in the database
    pub fn store_block(&self, _block: &BlockWrapper) -> Result<()> {
        // TODO: Implement full FFI conversion when block serialization is ready
        Ok(())
    }
    
    /// Get block by hash
    pub fn get_block_by_hash(&self, block_hash: &Hash256Wrapper) -> Result<Option<BlockWrapper>> {
        // This requires converting C++ block to Rust block
        // For now, return placeholder implementation
        Ok(None) // TODO: Implement full FFI conversion
    }
    
    /// Get block by height
    pub fn get_block_by_height(&self, height: u64) -> Result<Option<BlockWrapper>> {
        // This requires converting C++ block to Rust block
        // For now, return placeholder implementation
        Ok(None) // TODO: Implement full FFI conversion
    }
    
    /// Check if block exists
    pub fn has_block(&self, block_hash: &Hash256Wrapper) -> Result<bool> {
        let mut exists = false;
        let hash_ffi = block_hash.to_ffi();
        
        let result = unsafe {
            crate::storage_has_block(self.inner, &hash_ffi, &mut exists)
        };
        check_result(result)?;
        Ok(exists)
    }
    
    /// Store transaction
    pub fn store_transaction(&self, _transaction: &TransactionWrapper) -> Result<()> {
        // TODO: Implement full FFI conversion when transaction serialization is ready
        Ok(())
    }
    
    /// Get transaction by hash
    pub fn get_transaction(&self, txid: &Hash256Wrapper) -> Result<Option<TransactionWrapper>> {
        // This requires converting C++ transaction to Rust transaction
        // For now, return placeholder implementation
        Ok(None) // TODO: Implement full FFI conversion
    }
    
    /// Check if transaction exists
    pub fn has_transaction(&self, txid: &Hash256Wrapper) -> Result<bool> {
        let mut exists = false;
        let hash_ffi = txid.to_ffi();
        
        let result = unsafe {
            crate::storage_has_transaction(self.inner, &hash_ffi, &mut exists)
        };
        check_result(result)?;
        Ok(exists)
    }
    
    /// Add UTXO to the set
    pub fn add_utxo(&self, _outpoint: &OutPointWrapper, _output: &TransactionOutputWrapper) -> Result<()> {
        // TODO: Implement full FFI conversion when UTXO serialization is ready
        Ok(())
    }
    
    /// Remove UTXO from the set
    pub fn remove_utxo(&self, outpoint: &OutPointWrapper) -> Result<()> {
        let outpoint_ffi = outpoint.to_ffi();
        
        let result = unsafe {
            crate::storage_remove_utxo(self.inner, &outpoint_ffi)
        };
        check_result(result)
    }
    
    /// Get UTXO by outpoint
    pub fn get_utxo(&self, _outpoint: &OutPointWrapper) -> Result<Option<TransactionOutputWrapper>> {
        // TODO: Implement full FFI conversion when UTXO serialization is ready
        Ok(None)
    }
    
    /// Get total UTXO count
    pub fn get_utxo_count(&self) -> Result<usize> {
        let mut count = 0usize;
        
        let result = unsafe {
            crate::storage_get_utxo_count(self.inner, &mut count)
        };
        check_result(result)?;
        Ok(count)
    }
    
    /// Get chain tip (latest block)
    pub fn get_chain_tip(&self) -> Result<(Hash256Wrapper, u64)> {
        let mut tip_hash = std::mem::MaybeUninit::uninit();
        let mut tip_height = 0u64;
        
        let result = unsafe {
            crate::storage_get_chain_tip(self.inner, tip_hash.as_mut_ptr(), &mut tip_height)
        };
        check_result(result)?;
        
        let tip_hash = unsafe { tip_hash.assume_init() };
        Ok((Hash256Wrapper::from_ffi(&tip_hash), tip_height))
    }
    
    /// Set chain tip (update latest block)
    pub fn set_chain_tip(&self, tip_hash: &Hash256Wrapper, tip_height: u64) -> Result<()> {
        let hash_ffi = tip_hash.to_ffi();
        
        let result = unsafe {
            crate::storage_set_chain_tip(self.inner, &hash_ffi, tip_height)
        };
        check_result(result)
    }
}

impl Drop for StorageEngine {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            unsafe {
                crate::storage_engine_destroy(self.inner);
            }
        }
    }
}

unsafe impl Send for StorageEngine {}
unsafe impl Sync for StorageEngine {}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_storage_engine_creation() {
        let engine = StorageEngine::new("/tmp/test_blockchain");
        assert!(engine.is_ok());
    }
    
    #[test]
    fn test_utxo_operations() {
        let engine = StorageEngine::new("/tmp/test_blockchain").unwrap();
        
        // Test UTXO count (should start at 0)
        // Uncomment when C++ implementation is complete
        // let count = engine.get_utxo_count().unwrap();
        // assert_eq!(count, 0);
    }
}