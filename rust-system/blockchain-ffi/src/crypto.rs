//! Safe Rust wrapper for C++ crypto engine

use crate::{
    error::{check_result, check_result_with_bool, Result},
    types::{Hash256Wrapper, Hash160Wrapper, PrivateKeyWrapper, PublicKeyWrapper, SignatureWrapper},
};
use std::ptr;

/// Safe wrapper around C++ CryptoEngine
pub struct CryptoEngine {
    inner: *mut crate::CryptoEngine,
}

impl CryptoEngine {
    /// Create new crypto engine instance
    pub fn new() -> Result<Self> {
        let inner = unsafe { crate::crypto_engine_new() };
        if inner.is_null() {
            return Err(crate::BlockchainError::OutOfMemory.into());
        }
        Ok(Self { inner })
    }
    
    /// Generate a new private key
    pub fn generate_private_key(&self) -> Result<PrivateKeyWrapper> {
        let mut private_key = std::mem::MaybeUninit::uninit();
        let result = unsafe {
            crate::crypto_generate_private_key(private_key.as_mut_ptr())
        };
        check_result(result)?;
        
        let private_key = unsafe { private_key.assume_init() };
        Ok(PrivateKeyWrapper::from_ffi(&private_key))
    }
    
    /// Derive public key from private key
    pub fn derive_public_key(&self, private_key: &PrivateKeyWrapper) -> Result<PublicKeyWrapper> {
        let mut public_key = std::mem::MaybeUninit::uninit();
        let private_key_ffi = private_key.to_ffi();
        
        let result = unsafe {
            crate::crypto_derive_public_key(&private_key_ffi, public_key.as_mut_ptr())
        };
        check_result(result)?;
        
        let public_key = unsafe { public_key.assume_init() };
        Ok(PublicKeyWrapper::from_ffi(&public_key))
    }
    
    /// Check if private key is valid
    pub fn is_valid_private_key(&self, private_key: &PrivateKeyWrapper) -> bool {
        let private_key_ffi = private_key.to_ffi();
        unsafe { crate::crypto_is_valid_private_key(&private_key_ffi) }
    }
    
    /// Check if public key is valid
    pub fn is_valid_public_key(&self, public_key: &PublicKeyWrapper) -> bool {
        let public_key_ffi = public_key.to_ffi();
        unsafe { crate::crypto_is_valid_public_key(&public_key_ffi) }
    }
    
    /// Sign a message hash with private key
    pub fn sign_message(&self, private_key: &PrivateKeyWrapper, message_hash: &Hash256Wrapper) -> Result<SignatureWrapper> {
        let mut signature = std::mem::MaybeUninit::uninit();
        let private_key_ffi = private_key.to_ffi();
        let message_hash_ffi = message_hash.to_ffi();
        
        let result = unsafe {
            crate::crypto_sign_message(&private_key_ffi, &message_hash_ffi, signature.as_mut_ptr())
        };
        check_result(result)?;
        
        let signature = unsafe { signature.assume_init() };
        Ok(SignatureWrapper::from_ffi(&signature))
    }
    
    /// Verify signature against public key and message hash
    pub fn verify_signature(&self, public_key: &PublicKeyWrapper, message_hash: &Hash256Wrapper, signature: &SignatureWrapper) -> Result<bool> {
        let mut is_valid = false;
        let public_key_ffi = public_key.to_ffi();
        let message_hash_ffi = message_hash.to_ffi();
        let signature_ffi = signature.to_ffi();
        
        let result = unsafe {
            crate::crypto_verify_signature(&public_key_ffi, &message_hash_ffi, &signature_ffi, &mut is_valid)
        };
        check_result_with_bool(result, is_valid)
    }
    
    /// Calculate SHA-256 hash
    pub fn sha256(&self, input: &[u8]) -> Result<Hash256Wrapper> {
        let mut output = std::mem::MaybeUninit::uninit();
        
        let result = unsafe {
            crate::crypto_sha256(input.as_ptr(), input.len(), output.as_mut_ptr())
        };
        check_result(result)?;
        
        let output = unsafe { output.assume_init() };
        Ok(Hash256Wrapper::from_ffi(&output))
    }
    
    /// Calculate double SHA-256 hash (Bitcoin-style)
    pub fn double_sha256(&self, input: &[u8]) -> Result<Hash256Wrapper> {
        let mut output = std::mem::MaybeUninit::uninit();
        
        let result = unsafe {
            crate::crypto_double_sha256(input.as_ptr(), input.len(), output.as_mut_ptr())
        };
        check_result(result)?;
        
        let output = unsafe { output.assume_init() };
        Ok(Hash256Wrapper::from_ffi(&output))
    }
    
    /// Calculate RIPEMD-160 hash
    pub fn ripemd160(&self, input: &[u8]) -> Result<Hash160Wrapper> {
        let mut output = std::mem::MaybeUninit::uninit();
        
        let result = unsafe {
            crate::crypto_ripemd160(input.as_ptr(), input.len(), output.as_mut_ptr())
        };
        check_result(result)?;
        
        let output = unsafe { output.assume_init() };
        Ok(Hash160Wrapper::from_ffi(&output))
    }
    
    /// Calculate Merkle root from leaf hashes
    pub fn calculate_merkle_root(&self, leaf_hashes: &[Hash256Wrapper]) -> Result<Hash256Wrapper> {
        if leaf_hashes.is_empty() {
            return Ok(Hash256Wrapper::zero());
        }
        
        // Convert Rust hashes to C hashes
        let ffi_hashes: Vec<crate::Hash256> = leaf_hashes.iter()
            .map(|h| h.to_ffi())
            .collect();
            
        let mut root = std::mem::MaybeUninit::uninit();
        
        let result = unsafe {
            crate::crypto_calculate_merkle_root(
                ffi_hashes.as_ptr(),
                ffi_hashes.len(),
                root.as_mut_ptr()
            )
        };
        check_result(result)?;
        
        let root = unsafe { root.assume_init() };
        Ok(Hash256Wrapper::from_ffi(&root))
    }
    
    /// Verify Merkle proof
    pub fn verify_merkle_proof(
        &self,
        leaf_hash: &Hash256Wrapper,
        proof: &[Hash256Wrapper],
        root: &Hash256Wrapper,
        leaf_index: usize,
        tree_size: usize,
    ) -> Result<bool> {
        let leaf_hash_ffi = leaf_hash.to_ffi();
        let root_ffi = root.to_ffi();
        
        // Convert proof to C hashes
        let proof_ffi: Vec<crate::Hash256> = proof.iter()
            .map(|h| h.to_ffi())
            .collect();
            
        let mut is_valid = false;
        
        let result = unsafe {
            crate::crypto_verify_merkle_proof(
                &leaf_hash_ffi,
                proof_ffi.as_ptr(),
                proof_ffi.len(),
                &root_ffi,
                leaf_index,
                tree_size,
                &mut is_valid
            )
        };
        
        check_result_with_bool(result, is_valid)
    }
}

impl Drop for CryptoEngine {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            unsafe {
                crate::crypto_engine_destroy(self.inner);
            }
        }
    }
}

unsafe impl Send for CryptoEngine {}
unsafe impl Sync for CryptoEngine {}

/// Convenience functions that create a temporary crypto engine
pub mod convenience {
    use super::*;
    
    /// Generate a private key using temporary engine
    pub fn generate_private_key() -> Result<PrivateKeyWrapper> {
        let engine = CryptoEngine::new()?;
        engine.generate_private_key()
    }
    
    /// Derive public key from private key using temporary engine
    pub fn derive_public_key(private_key: &PrivateKeyWrapper) -> Result<PublicKeyWrapper> {
        let engine = CryptoEngine::new()?;
        engine.derive_public_key(private_key)
    }
    
    /// Calculate SHA-256 hash using temporary engine
    pub fn sha256(input: &[u8]) -> Result<Hash256Wrapper> {
        let engine = CryptoEngine::new()?;
        engine.sha256(input)
    }
    
    /// Calculate double SHA-256 hash using temporary engine
    pub fn double_sha256(input: &[u8]) -> Result<Hash256Wrapper> {
        let engine = CryptoEngine::new()?;
        engine.double_sha256(input)
    }
    
    /// Sign message using temporary engine
    pub fn sign_message(private_key: &PrivateKeyWrapper, message_hash: &Hash256Wrapper) -> Result<SignatureWrapper> {
        let engine = CryptoEngine::new()?;
        engine.sign_message(private_key, message_hash)
    }
    
    /// Verify signature using temporary engine
    pub fn verify_signature(public_key: &PublicKeyWrapper, message_hash: &Hash256Wrapper, signature: &SignatureWrapper) -> Result<bool> {
        let engine = CryptoEngine::new()?;
        engine.verify_signature(public_key, message_hash, signature)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_crypto_engine_creation() {
        let engine = CryptoEngine::new();
        assert!(engine.is_ok());
    }
    
    #[test]
    fn test_convenience_functions() {
        // Test basic functionality (these will fail without C++ implementation)
        // Uncomment when C++ core is built
        
        // let private_key = convenience::generate_private_key().unwrap();
        // let public_key = convenience::derive_public_key(&private_key).unwrap();
        // let message = b"Hello, blockchain!";
        // let hash = convenience::sha256(message).unwrap();
        // let signature = convenience::sign_message(&private_key, &hash).unwrap();
        // let is_valid = convenience::verify_signature(&public_key, &hash, &signature).unwrap();
        // assert!(is_valid);
    }
}