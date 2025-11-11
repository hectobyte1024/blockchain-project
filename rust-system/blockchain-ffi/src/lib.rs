//! FFI bindings to C++ blockchain core
//!
//! This crate provides safe Rust wrappers around the high-performance C++ 
//! blockchain core components. It handles memory management, error handling,
//! and type safety across the language boundary.

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

// Include the generated bindings
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

// External dependencies
extern crate hex;

pub mod crypto;
pub mod consensus;
pub mod storage;
pub mod vm;
pub mod types;
pub mod error;

use std::fmt;

/// Re-export common types for convenience
pub use error::{BlockchainError, Result};
pub use types::{
    Hash256Wrapper, Hash160Wrapper, PrivateKeyWrapper, PublicKeyWrapper,
    SignatureWrapper, TransactionWrapper, BlockWrapper, OutPointWrapper,
};

/// Safe wrapper for ByteBuffer with RAII cleanup
pub struct SafeByteBuffer {
    inner: *mut ByteBuffer,
}

impl SafeByteBuffer {
    /// Create a new SafeByteBuffer with given capacity
    pub fn with_capacity(capacity: usize) -> Result<Self> {
        let inner = unsafe { byte_buffer_new(capacity) };
        if inner.is_null() {
            return Err(BlockchainError::OutOfMemory);
        }
        Ok(SafeByteBuffer { inner })
    }
    
    /// Get the data as a slice
    pub fn as_slice(&self) -> &[u8] {
        if self.inner.is_null() {
            return &[];
        }
        unsafe {
            let buffer = &*self.inner;
            std::slice::from_raw_parts(buffer.data, buffer.size)
        }
    }
    
    /// Get mutable access to the data
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        if self.inner.is_null() {
            return &mut [];
        }
        unsafe {
            let buffer = &mut *self.inner;
            std::slice::from_raw_parts_mut(buffer.data, buffer.size)
        }
    }
    
    /// Append data to the buffer
    pub fn append_slice(&mut self, data: &[u8]) -> Result<()> {
        let result = unsafe {
            byte_buffer_append(self.inner, data.as_ptr(), data.len())
        };
        error::check_result(result)
    }
    
    /// Get the current length
    pub fn len(&self) -> usize {
        if self.inner.is_null() {
            return 0;
        }
        unsafe { (*self.inner).size }
    }
    
    /// Check if buffer is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    
    /// Get the current capacity  
    pub fn capacity(&self) -> usize {
        if self.inner.is_null() {
            return 0;
        }
        unsafe { (*self.inner).capacity }
    }
    

}

impl Drop for SafeByteBuffer {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            unsafe {
                byte_buffer_destroy(self.inner);
            }
        }
    }
}

unsafe impl Send for SafeByteBuffer {}

impl Clone for SafeByteBuffer {
    fn clone(&self) -> Self {
        let mut new_buffer = SafeByteBuffer::with_capacity(self.len())
            .expect("Failed to allocate SafeByteBuffer");
        new_buffer.append_slice(self.as_slice()).expect("Failed to copy data");
        new_buffer
    }
}

impl fmt::Debug for SafeByteBuffer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SafeByteBuffer")
            .field("length", &self.len())
            .field("capacity", &self.capacity())
            .field("data", &format_args!("{:?}", &hex::encode(&self.as_slice()[..std::cmp::min(16, self.len())])))
            .finish()
    }
}

/// Utility functions for hex encoding/decoding through FFI
pub mod hex_utils {
    use super::*;
    
    /// Encode bytes to hex string using C++ implementation
    pub fn encode(data: &[u8]) -> String {
        let mut output = vec![0u8; data.len() * 2 + 1]; // +1 for null terminator
        unsafe {
            hex_encode(data.as_ptr(), data.len(), output.as_mut_ptr() as *mut i8, output.len());
        }
        // Convert to string, removing null terminator
        String::from_utf8_lossy(&output[..data.len() * 2]).to_string()
    }
    
    /// Decode hex string to bytes using C++ implementation
    pub fn decode(hex_string: &str) -> Result<Vec<u8>> {
        let mut output = vec![0u8; hex_string.len() / 2];
        let mut output_len = output.len();
        
        let c_string = std::ffi::CString::new(hex_string)
            .map_err(|_| BlockchainError::InvalidInput("Invalid hex string".to_string()))?;
            
        let result = unsafe {
            hex_decode(c_string.as_ptr(), output.as_mut_ptr(), &mut output_len)
        };
        
        error::check_result(result)?;
        output.truncate(output_len);
        Ok(output)
    }
}

/// Initialize the FFI layer - call this before using any other functions
pub fn initialize() -> Result<()> {
    // Any global initialization needed
    Ok(())
}

/// Cleanup the FFI layer - call this before program exit
pub fn cleanup() {
    // Any global cleanup needed
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_byte_buffer() {
        let mut buffer = ByteBuffer::with_capacity(100).unwrap();
        assert_eq!(buffer.len(), 0);
        assert!(buffer.capacity() >= 100);
        
        let data = b"Hello, World!";
        buffer.append(data).unwrap();
        assert_eq!(buffer.len(), data.len());
        assert_eq!(buffer.as_slice(), data);
        
        buffer.resize(50).unwrap();
        assert_eq!(buffer.len(), 50);
    }
    
    #[test] 
    fn test_hex_utils() {
        let data = vec![0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF];
        let hex = hex_utils::encode(&data);
        assert_eq!(hex.to_lowercase(), "0123456789abcdef");
        
        let decoded = hex_utils::decode(&hex).unwrap();
        assert_eq!(decoded, data);
    }
    
    #[test]
    fn test_initialize_cleanup() {
        assert!(initialize().is_ok());
        cleanup(); // Should not panic
    }
}