//! Safe Rust wrapper for C++ consensus engine

use crate::{
    error::{check_result, check_result_with_bool, Result},
    types::{BlockWrapper, BlockHeaderWrapper, TransactionWrapper},
};

/// Safe wrapper around C++ ConsensusEngine
pub struct ConsensusEngine {
    inner: *mut crate::ConsensusEngine,
}

impl ConsensusEngine {
    /// Create new consensus engine instance
    pub fn new() -> Result<Self> {
        let inner = unsafe { crate::consensus_engine_new() };
        if inner.is_null() {
            return Err(crate::error::BlockchainError::OutOfMemory);
        }
        Ok(Self { inner })
    }
    
    /// Validate block header
    pub fn validate_block_header(&self, header: &BlockHeaderWrapper) -> Result<bool> {
        let mut is_valid = false;
        let header_ffi = header.to_ffi();
        
        let result = unsafe {
            crate::consensus_validate_block_header(self.inner, &header_ffi, &mut is_valid)
        };
        check_result_with_bool(result, is_valid)
    }
    
    /// Validate transaction
    pub fn validate_transaction(&self, _transaction: &TransactionWrapper) -> Result<bool> {
        // Basic validation - TODO: Implement full FFI conversion
        Ok(true)
    }
    
    /// Check proof of work for block header
    pub fn check_proof_of_work(&self, header: &BlockHeaderWrapper) -> Result<bool> {
        let mut meets_target = false;
        let header_ffi = header.to_ffi();
        
        let result = unsafe {
            crate::consensus_check_proof_of_work(&header_ffi, &mut meets_target)
        };
        check_result_with_bool(result, meets_target)
    }
    
    /// Calculate next difficulty adjustment
    pub fn calculate_difficulty_adjustment(
        &self,
        current_height: u64,
        current_timestamp: u64,
    ) -> Result<u32> {
        let mut new_target = 0u32;
        
        let result = unsafe {
            crate::consensus_calculate_difficulty_adjustment(
                self.inner,
                current_height,
                current_timestamp,
                &mut new_target
            )
        };
        check_result(result)?;
        Ok(new_target)
    }
    
    /// Get block reward for given height
    pub fn get_block_reward(&self, height: u64) -> Result<u64> {
        let mut reward = 0u64;
        
        let result = unsafe {
            crate::consensus_get_block_reward(height, &mut reward)
        };
        check_result(result)?;
        Ok(reward)
    }
    
    /// Get next difficulty target
    pub fn get_next_difficulty_target(&self, height: u64) -> Result<u32> {
        let mut target = 0u32;
        
        let result = unsafe {
            crate::consensus_get_next_difficulty_target(self.inner, height, &mut target)
        };
        check_result(result)?;
        Ok(target)
    }
}

impl Drop for ConsensusEngine {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            unsafe {
                crate::consensus_engine_destroy(self.inner);
            }
        }
    }
}

unsafe impl Send for ConsensusEngine {}
unsafe impl Sync for ConsensusEngine {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Hash256Wrapper;
    
    #[test]
    fn test_consensus_engine_creation() {
        let engine = ConsensusEngine::new();
        assert!(engine.is_ok());
    }
    
    #[test] 
    fn test_block_reward_calculation() {
        let engine = ConsensusEngine::new().unwrap();
        
        // Test initial block reward (height 0)
        // Uncomment when C++ implementation is complete
        // let reward = engine.get_block_reward(0).unwrap();
        // assert_eq!(reward, 50 * 100_000_000); // 50 BTC in satoshis
    }
}