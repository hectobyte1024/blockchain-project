//! Ethereum Precompiled Contracts
//!
//! Standard precompiled contracts at addresses 0x01-0x09

use sha2::{Sha256, Digest};
use ripemd::{Ripemd160, Digest as RipemdDigest};

/// Precompiled contract addresses
pub const ECRECOVER: [u8; 20] = [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1];
pub const SHA256: [u8; 20] = [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,2];
pub const RIPEMD160: [u8; 20] = [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,3];
pub const IDENTITY: [u8; 20] = [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,4];

/// Execute precompiled contract
pub fn execute_precompile(address: &[u8; 20], input: &[u8]) -> Option<Vec<u8>> {
    match address {
        &ECRECOVER => Some(ecrecover(input)),
        &SHA256 => Some(sha256(input)),
        &RIPEMD160 => Some(ripemd160(input)),
        &IDENTITY => Some(identity(input)),
        _ => None,
    }
}

/// 0x01: ecrecover - Recover signer address from ECDSA signature
/// Input: [hash(32) || v(32) || r(32) || s(32)] = 128 bytes
/// Output: [address(32)] = 32 bytes (left-padded to 32 bytes)
fn ecrecover(input: &[u8]) -> Vec<u8> {
    // Ensure input is exactly 128 bytes
    let mut padded_input = vec![0u8; 128];
    let copy_len = input.len().min(128);
    padded_input[..copy_len].copy_from_slice(&input[..copy_len]);
    
    let hash = &padded_input[0..32];
    let v_bytes = &padded_input[32..64];
    let r = &padded_input[64..96];
    let s = &padded_input[96..128];
    
    // Extract v (last byte of v_bytes)
    let v = v_bytes[31];
    if v != 27 && v != 28 {
        return vec![0u8; 32]; // Invalid v
    }
    
    // Use secp256k1 for signature recovery
    use secp256k1::{Secp256k1, Message, ecdsa::RecoverableSignature, ecdsa::RecoveryId};
    
    let secp = Secp256k1::new();
    
    // Create message from hash
    let message = match Message::from_digest_slice(hash) {
        Ok(msg) => msg,
        Err(_) => return vec![0u8; 32],
    };
    
    // Create recoverable signature
    let recovery_id = match RecoveryId::from_i32((v - 27) as i32) {
        Ok(rid) => rid,
        Err(_) => return vec![0u8; 32],
    };
    
    // Combine r and s into signature bytes
    let mut sig_bytes = [0u8; 64];
    sig_bytes[0..32].copy_from_slice(r);
    sig_bytes[32..64].copy_from_slice(s);
    
    let recoverable_sig = match RecoverableSignature::from_compact(&sig_bytes, recovery_id) {
        Ok(sig) => sig,
        Err(_) => return vec![0u8; 32],
    };
    
    // Recover public key
    let public_key = match secp.recover_ecdsa(&message, &recoverable_sig) {
        Ok(pk) => pk,
        Err(_) => return vec![0u8; 32],
    };
    
    // Hash public key (uncompressed, without 0x04 prefix) with Keccak256
    let pubkey_bytes = &public_key.serialize_uncompressed()[1..]; // Skip 0x04 prefix
    use sha3::{Keccak256, Digest as Keccak256Digest};
    let mut hasher = Keccak256::new();
    hasher.update(pubkey_bytes);
    let keccak_hash = hasher.finalize();
    
    // Take last 20 bytes as Ethereum address, left-pad to 32 bytes
    let mut result = vec![0u8; 32];
    result[12..32].copy_from_slice(&keccak_hash[12..32]);
    result
}

/// 0x02: sha256 - SHA-256 hash function
/// Input: arbitrary bytes
/// Output: [hash(32)] = 32 bytes
fn sha256(input: &[u8]) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(input);
    hasher.finalize().to_vec()
}

/// 0x03: ripemd160 - RIPEMD-160 hash function
/// Input: arbitrary bytes
/// Output: [hash(32)] = 32 bytes (left-padded)
fn ripemd160(input: &[u8]) -> Vec<u8> {
    let mut hasher = Ripemd160::new();
    hasher.update(input);
    let hash = hasher.finalize();
    
    // Left-pad to 32 bytes
    let mut result = vec![0u8; 32];
    result[12..32].copy_from_slice(&hash);
    result
}

/// 0x04: identity - Data copy (return input as-is)
/// Input: arbitrary bytes
/// Output: same as input
fn identity(input: &[u8]) -> Vec<u8> {
    input.to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_identity() {
        let input = vec![1, 2, 3, 4, 5];
        let output = identity(&input);
        assert_eq!(output, input);
    }
    
    #[test]
    fn test_sha256() {
        let input = b"hello world";
        let output = sha256(input);
        assert_eq!(output.len(), 32);
        
        // Known SHA-256 hash of "hello world"
        let expected = hex::decode("b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9").unwrap();
        assert_eq!(output, expected);
    }
    
    #[test]
    fn test_ripemd160() {
        let input = b"hello world";
        let output = ripemd160(input);
        assert_eq!(output.len(), 32);
        
        // Result should be left-padded
        assert_eq!(&output[0..12], &[0u8; 12]);
    }
    
    #[test]
    fn test_ecrecover_invalid_v() {
        let mut input = vec![0u8; 128];
        input[63] = 25; // Invalid v value
        let output = ecrecover(&input);
        assert_eq!(output, vec![0u8; 32]); // Should return zero address
    }
}
