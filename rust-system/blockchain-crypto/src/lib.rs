//! Cryptographic primitives and utilities for blockchain operations
//!
//! This crate provides all cryptographic functionality needed for a blockchain system,
//! including hashing, digital signatures, key management, and encoding.

pub mod hash;
pub mod signature; 
pub mod keys;
pub mod merkle;
pub mod encoding;
pub mod kdf;

use serde::{Deserialize, Serialize};

/// 256-bit hash type
pub type Hash256 = [u8; 32];

/// 160-bit hash type  
pub type Hash160 = [u8; 20];

/// Private key type (32 bytes)
pub type PrivateKey = [u8; 32];

/// Compressed public key type (33 bytes)
pub type PublicKey = [u8; 33];

/// ECDSA signature type (64 bytes: r + s)
pub type Signature = [u8; 64];

/// Recovery ID for signature verification
pub type RecoveryId = u8;

/// Address types supported by the blockchain
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Address {
    /// Pay-to-Public-Key-Hash (P2PKH)
    P2PKH(Hash160),
    /// Pay-to-Script-Hash (P2SH)  
    P2SH(Hash160),
    /// Pay-to-Witness-Public-Key-Hash (P2WPKH)
    P2WPKH(Hash160),
    /// Pay-to-Witness-Script-Hash (P2WSH)
    P2WSH(Hash256),
}

impl Address {
    /// Get the hash contained in this address
    pub fn hash(&self) -> Vec<u8> {
        match self {
            Address::P2PKH(hash) | Address::P2SH(hash) | Address::P2WPKH(hash) => hash.to_vec(),
            Address::P2WSH(hash) => hash.to_vec(),
        }
    }
    
    /// Get the address type as a string
    pub fn address_type(&self) -> &'static str {
        match self {
            Address::P2PKH(_) => "P2PKH",
            Address::P2SH(_) => "P2SH", 
            Address::P2WPKH(_) => "P2WPKH",
            Address::P2WSH(_) => "P2WSH",
        }
    }
}

/// Cryptographic error types
#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    #[error("Invalid private key")]
    InvalidPrivateKey,
    
    #[error("Invalid public key")]
    InvalidPublicKey,
    
    #[error("Invalid signature")]
    InvalidSignature,
    
    #[error("Signature verification failed")]
    SignatureVerificationFailed,
    
    #[error("Key derivation failed: {0}")]
    KeyDerivationFailed(String),
    
    #[error("Encoding error: {0}")]
    EncodingError(String),
    
    #[error("Decoding error: {0}")]
    DecodingError(String),
    
    #[error("Invalid address format")]
    InvalidAddressFormat,
    
    #[error("Secp256k1 error: {0}")]
    Secp256k1Error(#[from] secp256k1::Error),
    
    #[error("Random generation error")]
    RandomError,
}

pub type Result<T> = std::result::Result<T, CryptoError>;

/// Utility functions for common cryptographic operations
pub mod utils {
    use super::*;
    use rand::{RngCore, thread_rng};
    
    /// Generate cryptographically secure random bytes
    pub fn secure_random_bytes(len: usize) -> Vec<u8> {
        let mut bytes = vec![0u8; len];
        thread_rng().fill_bytes(&mut bytes);
        bytes
    }
    
    /// Generate a random 32-byte array
    pub fn random_32_bytes() -> [u8; 32] {
        let mut bytes = [0u8; 32];
        thread_rng().fill_bytes(&mut bytes);
        bytes
    }
    
    /// Constant-time comparison to prevent timing attacks
    pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
        if a.len() != b.len() {
            return false;
        }
        
        let mut result = 0u8;
        for (x, y) in a.iter().zip(b.iter()) {
            result |= x ^ y;
        }
        result == 0
    }
    
    /// Convert bytes to hex string (lowercase)
    pub fn bytes_to_hex(bytes: &[u8]) -> String {
        hex::encode(bytes)
    }
    
    /// Convert hex string to bytes
    pub fn hex_to_bytes(hex: &str) -> Result<Vec<u8>> {
        hex::decode(hex).map_err(|e| CryptoError::DecodingError(e.to_string()))
    }
    
    /// Zeroize sensitive data in memory
    pub fn zeroize(data: &mut [u8]) {
        for byte in data.iter_mut() {
            *byte = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_address_types() {
        let hash160 = [1u8; 20];
        let hash256 = [2u8; 32];
        
        let p2pkh = Address::P2PKH(hash160);
        let p2sh = Address::P2SH(hash160);
        let p2wpkh = Address::P2WPKH(hash160);
        let p2wsh = Address::P2WSH(hash256);
        
        assert_eq!(p2pkh.address_type(), "P2PKH");
        assert_eq!(p2sh.address_type(), "P2SH");
        assert_eq!(p2wpkh.address_type(), "P2WPKH");
        assert_eq!(p2wsh.address_type(), "P2WSH");
        
        assert_eq!(p2pkh.hash(), hash160.to_vec());
        assert_eq!(p2wsh.hash(), hash256.to_vec());
    }
    
    #[test]
    fn test_constant_time_eq() {
        let a = vec![1, 2, 3, 4];
        let b = vec![1, 2, 3, 4];
        let c = vec![1, 2, 3, 5];
        let d = vec![1, 2, 3];
        
        assert!(utils::constant_time_eq(&a, &b));
        assert!(!utils::constant_time_eq(&a, &c));
        assert!(!utils::constant_time_eq(&a, &d));
    }
    
    #[test]
    fn test_random_bytes() {
        let bytes1 = utils::secure_random_bytes(32);
        let bytes2 = utils::secure_random_bytes(32);
        
        assert_eq!(bytes1.len(), 32);
        assert_eq!(bytes2.len(), 32);
        assert_ne!(bytes1, bytes2); // Very unlikely to be equal
        
        let array1 = utils::random_32_bytes();
        let array2 = utils::random_32_bytes();
        
        assert_ne!(array1, array2); // Very unlikely to be equal
    }
}