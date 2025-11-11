//! Safe Rust bindings for C++ consensus engine
//!
//! This module provides memory-safe wrappers around the high-performance 
//! C++ consensus functions, enabling the Rust system layer to leverage
//! optimized mining and validation operations.

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

// Include the generated bindings
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

use std::ffi::CString;

/// Mining result from C++ engine
#[derive(Debug, Clone)]
pub struct MiningResult {
    pub success: bool,
    pub nonce: u32,
    pub block_hash: [u8; 32],
    pub hash_operations: u64,
    pub elapsed_seconds: f64,
}

/// Safe wrapper for C++ mining functionality
pub struct ConsensusMiner;

impl ConsensusMiner {
    /// Mine a block using the high-performance C++ engine
    pub fn mine_block(
        block_data: &str,
        difficulty_target: u32,
        max_iterations: Option<u64>,
    ) -> Result<MiningResult, &'static str> {
        // Convert Rust string to C string
        let c_block_data = CString::new(block_data)
            .map_err(|_| "Invalid block data: contains null bytes")?;
        
        let iterations = max_iterations.unwrap_or(0);
        
        // Call C++ function
        let c_result = unsafe {
            c_mine_block(
                c_block_data.as_ptr(),
                difficulty_target,
                iterations,
            )
        };
        
        Ok(MiningResult {
            success: c_result.success,
            nonce: c_result.nonce,
            block_hash: c_result.block_hash,
            hash_operations: c_result.hash_operations,
            elapsed_seconds: c_result.elapsed_seconds,
        })
    }
    
    /// Verify proof-of-work using C++ engine
    pub fn verify_proof_of_work(
        block_data: &str,
        nonce: u32,
        difficulty_target: u32,
    ) -> Result<bool, &'static str> {
        let c_block_data = CString::new(block_data)
            .map_err(|_| "Invalid block data: contains null bytes")?;
        
        let is_valid = unsafe {
            c_verify_proof_of_work(
                c_block_data.as_ptr(),
                nonce,
                difficulty_target,
            )
        };
        
        Ok(is_valid)
    }
    
    /// Calculate next difficulty target
    pub fn calculate_next_difficulty(
        current_difficulty: u32,
        actual_time_span: u64,
        target_time_span: u64,
    ) -> u32 {
        unsafe {
            c_calculate_next_difficulty(
                current_difficulty,
                actual_time_span,
                target_time_span,
            )
        }
    }
    
    /// Check if difficulty should be adjusted
    pub fn should_adjust_difficulty(block_height: u32) -> bool {
        unsafe { c_should_adjust_difficulty(block_height) }
    }
}

/// Performance utilities
impl ConsensusMiner {
    /// Estimate mining time based on difficulty and hash rate
    pub fn estimate_mining_time(
        difficulty_target: u32,
        hash_rate: f64, // hashes per second
    ) -> f64 {
        // Simple estimation based on difficulty
        let required_zeros = (difficulty_target >> 24) as f64;
        let expected_hashes = 16_f64.powf(required_zeros);
        expected_hashes / hash_rate.max(1.0)
    }
    
    /// Get difficulty from target bits
    pub fn bits_to_difficulty(difficulty_bits: u32) -> f64 {
        let required_zeros = (difficulty_bits >> 24) as f64;
        16_f64.powf(required_zeros)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mining_integration() {
        let block_data = "test_block_data_for_mining";
        let difficulty_target = 0x01000000; // 1 leading zero (easy)
        
        let result = ConsensusMiner::mine_block(
            block_data,
            difficulty_target,
            Some(10000),
        ).expect("Mining should not fail");
        
        println!("Mining result: {:?}", result);
        
        if result.success {
            // Verify the found nonce
            let is_valid = ConsensusMiner::verify_proof_of_work(
                block_data,
                result.nonce,
                difficulty_target,
            ).expect("Verification should not fail");
            
            assert!(is_valid, "Found nonce should be valid");
            println!("✅ Mining and verification successful!");
        } else {
            println!("⚠️  Mining didn't find solution in iteration limit");
        }
    }
    
    #[test]
    fn test_difficulty_adjustment() {
        let current_difficulty = 0x01000000;
        
        // Test extreme fast case (less than half of target time)
        let fast_time = 250; // Much faster than half target
        let target_time = 600; // 10 minutes
        
        let new_difficulty = ConsensusMiner::calculate_next_difficulty(
            current_difficulty,
            fast_time,
            target_time,
        );
        
        println!("Fast mining: 0x{:08x} -> 0x{:08x}", 
                 current_difficulty, new_difficulty);
        
        // Should increase difficulty (more zeros required)
        assert_eq!(new_difficulty, 0x02000000); // increased from 1 to 2 zeros
        
        // Test extreme slow case (more than double target time)
        let slow_time = 1300; // Much slower than double target
        let new_difficulty_slow = ConsensusMiner::calculate_next_difficulty(
            current_difficulty,
            slow_time,
            target_time,
        );
        
        println!("Slow mining: 0x{:08x} -> 0x{:08x}", 
                 current_difficulty, new_difficulty_slow);
        
        // Should decrease difficulty (fewer zeros required)
        assert_eq!(new_difficulty_slow, 0x00000000); // decreased to 0 zeros
        
        assert!(ConsensusMiner::should_adjust_difficulty(10));
        assert!(!ConsensusMiner::should_adjust_difficulty(5));
    }
}