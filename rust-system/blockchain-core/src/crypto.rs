//! Cryptographic Operations
//!
//! Real secp256k1 ECDSA signatures and public key derivation.

use crate::{BlockchainError, Result, PrivateKey, PublicKey, Signature, Hash256};
use secp256k1::{Secp256k1, Message, SecretKey, PublicKey as Secp256k1PublicKey};
use sha2::{Sha256, Digest};

/// Derive public key from private key using secp256k1 curve
pub fn derive_public_key(private_key: &PrivateKey) -> Result<PublicKey> {
    let secp = Secp256k1::new();
    
    // Convert 32-byte private key to SecretKey
    let secret_key = SecretKey::from_slice(private_key)
        .map_err(|e| BlockchainError::CryptoError(format!("Invalid private key: {}", e)))?;
    
    // Derive public key
    let public_key = Secp256k1PublicKey::from_secret_key(&secp, &secret_key);
    
    // Return compressed public key as Vec (33 bytes)
    Ok(public_key.serialize().to_vec())
}

/// Sign a hash with a private key using ECDSA
pub fn sign_hash(hash: &Hash256, private_key: &PrivateKey) -> Result<Signature> {
    let secp = Secp256k1::new();
    
    // Convert private key
    let secret_key = SecretKey::from_slice(private_key)
        .map_err(|e| BlockchainError::CryptoError(format!("Invalid private key: {}", e)))?;
    
    // Create message from hash
    let message = Message::from_digest_slice(hash)
        .map_err(|e| BlockchainError::CryptoError(format!("Invalid message hash: {}", e)))?;
    
    // Sign
    let signature = secp.sign_ecdsa(&message, &secret_key);
    
    // Return DER-encoded signature
    Ok(signature.serialize_der().to_vec())
}

/// Verify an ECDSA signature
pub fn verify_signature(
    signature: &[u8],
    public_key: &[u8],  // Accept slice for flexibility
    hash: &Hash256,
) -> Result<bool> {
    let secp = Secp256k1::new();
    
    // Parse public key
    let pubkey = Secp256k1PublicKey::from_slice(public_key)
        .map_err(|e| BlockchainError::CryptoError(format!("Invalid public key: {}", e)))?;
    
    // Parse signature (DER format)
    let sig = secp256k1::ecdsa::Signature::from_der(signature)
        .map_err(|e| BlockchainError::CryptoError(format!("Invalid signature: {}", e)))?;
    
    // Create message
    let message = Message::from_digest_slice(hash)
        .map_err(|e| BlockchainError::CryptoError(format!("Invalid message hash: {}", e)))?;
    
    // Verify
    Ok(secp.verify_ecdsa(&message, &sig, &pubkey).is_ok())
}

/// Hash data with SHA256
pub fn sha256(data: &[u8]) -> Hash256 {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result);
    hash
}

/// Hash data twice with SHA256 (Bitcoin-style)
pub fn double_sha256(data: &[u8]) -> Hash256 {
    sha256(&sha256(data))
}

/// Generate a random private key
pub fn generate_private_key() -> Result<PrivateKey> {
    let secp = Secp256k1::new();
    let mut rng = rand::thread_rng();
    let (secret_key, _) = secp.generate_keypair(&mut rng);
    Ok(secret_key.secret_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_derivation() {
        let private_key = generate_private_key().unwrap();
        let public_key = derive_public_key(&private_key).unwrap();
        
        assert_eq!(public_key.len(), 33); // Compressed public key
        assert!(public_key[0] == 0x02 || public_key[0] == 0x03); // Valid prefix
    }

    #[test]
    fn test_sign_and_verify() {
        let private_key = generate_private_key().unwrap();
        let public_key = derive_public_key(&private_key).unwrap();
        
        let message = b"Hello, blockchain!";
        let hash = sha256(message);
        
        let signature = sign_hash(&hash, &private_key).unwrap();
        let valid = verify_signature(&signature, &public_key, &hash).unwrap();
        
        assert!(valid, "Signature should be valid");
    }

    #[test]
    fn test_invalid_signature() {
        let private_key1 = generate_private_key().unwrap();
        let private_key2 = generate_private_key().unwrap();
        
        let public_key1 = derive_public_key(&private_key1).unwrap();
        let public_key2 = derive_public_key(&private_key2).unwrap();
        
        let message = b"Test message";
        let hash = sha256(message);
        
        // Sign with key1, verify with key2's public key
        let signature = sign_hash(&hash, &private_key1).unwrap();
        let valid = verify_signature(&signature, &public_key2, &hash).unwrap();
        
        assert!(!valid, "Signature should be invalid");
    }

    #[test]
    fn test_double_sha256() {
        let data = b"test data";
        let hash1 = double_sha256(data);
        let hash2 = sha256(&sha256(data));
        assert_eq!(hash1, hash2);
    }
}
