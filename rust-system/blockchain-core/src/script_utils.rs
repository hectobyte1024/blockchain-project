use crate::{BlockchainError, Result as BlockchainResult};
use sha2::{Sha256, Digest};
use blake3;

/// Bitcoin script opcodes
pub mod opcodes {
    pub const OP_DUP: u8 = 0x76;
    pub const OP_HASH160: u8 = 0xa9;
    pub const OP_EQUALVERIFY: u8 = 0x88;
    pub const OP_CHECKSIG: u8 = 0xac;
    pub const OP_PUSHDATA_20: u8 = 0x14; // Push 20 bytes
    pub const OP_RETURN: u8 = 0x6a;
    pub const OP_1: u8 = 0x51;
    pub const OP_2: u8 = 0x52;
    pub const OP_3: u8 = 0x53;
    pub const OP_CHECKMULTISIG: u8 = 0xae;
}

/// Script creation utilities for different address types
pub struct ScriptBuilder;

impl ScriptBuilder {
    /// Create a Pay-to-Public-Key-Hash (P2PKH) script
    /// Format: OP_DUP OP_HASH160 <pubkey_hash> OP_EQUALVERIFY OP_CHECKSIG
    pub fn create_p2pkh_script(pubkey_hash: &[u8; 20]) -> Vec<u8> {
        let mut script = Vec::with_capacity(25);
        script.push(opcodes::OP_DUP);
        script.push(opcodes::OP_HASH160);
        script.push(opcodes::OP_PUSHDATA_20);
        script.extend_from_slice(pubkey_hash);
        script.push(opcodes::OP_EQUALVERIFY);
        script.push(opcodes::OP_CHECKSIG);
        script
    }

    /// Create P2PKH script from EDU address
    pub fn create_p2pkh_from_address(address: &str) -> BlockchainResult<Vec<u8>> {
        let pubkey_hash = Self::address_to_hash160(address)?;
        Ok(Self::create_p2pkh_script(&pubkey_hash))
    }

    /// Create a Pay-to-Script-Hash (P2SH) script
    /// Format: OP_HASH160 <script_hash> OP_EQUAL
    pub fn create_p2sh_script(script_hash: &[u8; 20]) -> Vec<u8> {
        let mut script = Vec::with_capacity(23);
        script.push(opcodes::OP_HASH160);
        script.push(opcodes::OP_PUSHDATA_20);
        script.extend_from_slice(script_hash);
        script.push(opcodes::OP_EQUALVERIFY);
        script
    }

    /// Create a multisig script
    /// Format: OP_M <pubkey1> <pubkey2> ... OP_N OP_CHECKMULTISIG
    pub fn create_multisig_script(required: u8, public_keys: &[[u8; 33]]) -> BlockchainResult<Vec<u8>> {
        if required == 0 || required > public_keys.len() as u8 || public_keys.len() > 16 {
            return Err(BlockchainError::InvalidScript("Invalid multisig parameters".to_string()));
        }

        let mut script = Vec::new();
        
        // Push required signatures count
        script.push(opcodes::OP_1 + required - 1);
        
        // Push public keys
        for pubkey in public_keys {
            script.push(33); // Push 33 bytes
            script.extend_from_slice(pubkey);
        }
        
        // Push total public keys count
        script.push(opcodes::OP_1 + public_keys.len() as u8 - 1);
        script.push(opcodes::OP_CHECKMULTISIG);
        
        Ok(script)
    }

    /// Create an OP_RETURN script for data storage
    pub fn create_op_return_script(data: &[u8]) -> BlockchainResult<Vec<u8>> {
        if data.len() > 80 {
            return Err(BlockchainError::InvalidScript("OP_RETURN data too large".to_string()));
        }

        let mut script = Vec::with_capacity(2 + data.len());
        script.push(opcodes::OP_RETURN);
        script.push(data.len() as u8);
        script.extend_from_slice(data);
        Ok(script)
    }

    /// Generate proper EDU address from public key
    pub fn pubkey_to_address(public_key: &[u8; 33]) -> BlockchainResult<String> {
        let pubkey_hash = Self::hash160(public_key);
        let encoded = bs58::encode(&pubkey_hash).into_string();
        Ok(format!("edu1q{}", encoded))
    }

    /// Generate P2SH address from script
    pub fn script_to_p2sh_address(script: &[u8]) -> BlockchainResult<String> {
        let script_hash = Self::hash160(script);
        let encoded = bs58::encode(&script_hash).into_string();
        Ok(format!("edu3{}", encoded)) // Different prefix for P2SH
    }

    /// Extract hash160 from EDU address
    pub fn address_to_hash160(address: &str) -> BlockchainResult<[u8; 20]> {
        if address.starts_with("edu1q") && address.len() >= 25 {
            // P2PKH address
            let encoded_part = &address[5..];
            let decoded = bs58::decode(encoded_part)
                .into_vec()
                .map_err(|_| BlockchainError::InvalidAddress("Invalid base58 encoding".to_string()))?;
            
            if decoded.len() == 20 {
                let mut hash160 = [0u8; 20];
                hash160.copy_from_slice(&decoded);
                Ok(hash160)
            } else {
                Err(BlockchainError::InvalidAddress("Invalid hash160 length".to_string()))
            }
        } else if address.starts_with("edu3") && address.len() >= 25 {
            // P2SH address
            let encoded_part = &address[4..];
            let decoded = bs58::decode(encoded_part)
                .into_vec()
                .map_err(|_| BlockchainError::InvalidAddress("Invalid base58 encoding".to_string()))?;
            
            if decoded.len() == 20 {
                let mut hash160 = [0u8; 20];
                hash160.copy_from_slice(&decoded);
                Ok(hash160)
            } else {
                Err(BlockchainError::InvalidAddress("Invalid hash160 length".to_string()))
            }
        } else {
            Err(BlockchainError::InvalidAddress(format!("Unsupported address format: {}", address)))
        }
    }

    /// Compute HASH160 (SHA256 + RIPEMD160)
    fn hash160(input: &[u8]) -> [u8; 20] {
        // For EDU blockchain, we use SHA256 followed by taking first 20 bytes
        // In Bitcoin, this would be SHA256 + RIPEMD160
        let sha256_hash = Sha256::digest(input);
        let mut hash160 = [0u8; 20];
        hash160.copy_from_slice(&sha256_hash[0..20]);
        hash160
    }

    /// Generate deterministic mining address for coinbase outputs
    pub fn generate_mining_address(node_id: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(b"EDU_MINING_");
        hasher.update(node_id.as_bytes());
        let hash = hasher.finalize();
        
        // Create EDU address format: edu1q + base58 encoded hash
        let addr_bytes = &hash[0..20];
        let encoded = bs58::encode(addr_bytes).into_string();
        format!("edu1q{}", encoded)
    }

    /// Create coinbase transaction output script
    pub fn create_coinbase_script(miner_address: &str) -> BlockchainResult<Vec<u8>> {
        Self::create_p2pkh_from_address(miner_address)
    }

    /// Validate script format
    pub fn is_valid_script(script: &[u8]) -> bool {
        !script.is_empty() && script.len() <= 10000 // Bitcoin's MAX_SCRIPT_SIZE
    }

    /// Check if script is P2PKH
    pub fn is_p2pkh_script(script: &[u8]) -> bool {
        script.len() == 25 &&
        script[0] == opcodes::OP_DUP &&
        script[1] == opcodes::OP_HASH160 &&
        script[2] == opcodes::OP_PUSHDATA_20 &&
        script[23] == opcodes::OP_EQUALVERIFY &&
        script[24] == opcodes::OP_CHECKSIG
    }

    /// Check if script is P2SH
    pub fn is_p2sh_script(script: &[u8]) -> bool {
        script.len() == 23 &&
        script[0] == opcodes::OP_HASH160 &&
        script[1] == opcodes::OP_PUSHDATA_20 &&
        script[22] == opcodes::OP_EQUALVERIFY
    }

    /// Extract address from P2PKH script
    pub fn extract_p2pkh_address(script: &[u8]) -> BlockchainResult<String> {
        if !Self::is_p2pkh_script(script) {
            return Err(BlockchainError::InvalidScript("Not a P2PKH script".to_string()));
        }

        let hash160_bytes = &script[3..23];
        let mut hash160 = [0u8; 20];
        hash160.copy_from_slice(hash160_bytes);
        
        let encoded = bs58::encode(&hash160).into_string();
        Ok(format!("edu1q{}", encoded))
    }

    /// Extract address from P2SH script
    pub fn extract_p2sh_address(script: &[u8]) -> BlockchainResult<String> {
        if !Self::is_p2sh_script(script) {
            return Err(BlockchainError::InvalidScript("Not a P2SH script".to_string()));
        }

        let hash160_bytes = &script[2..22];
        let mut hash160 = [0u8; 20];
        hash160.copy_from_slice(hash160_bytes);
        
        let encoded = bs58::encode(&hash160).into_string();
        Ok(format!("edu3{}", encoded))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_p2pkh_script_creation() {
        let hash160 = [0x89, 0xab, 0xcd, 0xef, 0x9d, 0xcb, 0xa9, 0x87, 0x65, 0x43, 
                      0x21, 0x0f, 0xed, 0xcb, 0xa9, 0x87, 0x65, 0x43, 0x21, 0x0f];
        
        let script = ScriptBuilder::create_p2pkh_script(&hash160);
        
        assert_eq!(script.len(), 25);
        assert_eq!(script[0], opcodes::OP_DUP);
        assert_eq!(script[1], opcodes::OP_HASH160);
        assert_eq!(script[2], opcodes::OP_PUSHDATA_20);
        assert_eq!(&script[3..23], &hash160);
        assert_eq!(script[23], opcodes::OP_EQUALVERIFY);
        assert_eq!(script[24], opcodes::OP_CHECKSIG);
    }

    #[test]
    fn test_address_generation() {
        let public_key = [0x02; 33]; // Compressed public key
        let address = ScriptBuilder::pubkey_to_address(&public_key).unwrap();
        assert!(address.starts_with("edu1q"));
    }

    #[test]
    fn test_script_validation() {
        let hash160 = [0; 20];
        let valid_script = ScriptBuilder::create_p2pkh_script(&hash160);
        assert!(ScriptBuilder::is_valid_script(&valid_script));
        assert!(ScriptBuilder::is_p2pkh_script(&valid_script));
    }

    #[test]
    fn test_multisig_script() {
        let pubkeys = [
            [0x02; 33],
            [0x03; 33],
            [0x04; 33],
        ];
        
        let script = ScriptBuilder::create_multisig_script(2, &pubkeys).unwrap();
        assert!(!script.is_empty());
        assert_eq!(script[0], opcodes::OP_2);
        assert_eq!(script[script.len() - 2], opcodes::OP_3);
        assert_eq!(script[script.len() - 1], opcodes::OP_CHECKMULTISIG);
    }

    #[test]
    fn test_op_return_script() {
        let data = b"Hello EDU Blockchain!";
        let script = ScriptBuilder::create_op_return_script(data).unwrap();
        
        assert_eq!(script[0], opcodes::OP_RETURN);
        assert_eq!(script[1], data.len() as u8);
        assert_eq!(&script[2..], data);
    }
}