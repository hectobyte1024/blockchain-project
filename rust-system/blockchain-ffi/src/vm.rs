//! Safe Rust wrapper for C++ virtual machine engine

use crate::{
    error::{check_result, check_result_with_bool, Result},
    types::{Hash256Wrapper, Hash160Wrapper, TransactionWrapper, PublicKeyWrapper},
    SafeByteBuffer,
};

/// Safe wrapper around C++ VMEngine
pub struct VMEngine {
    inner: *mut crate::VMEngine,
}

impl VMEngine {
    /// Create new VM engine instance
    pub fn new() -> Result<Self> {
        let inner = unsafe { crate::vm_engine_new() };
        if inner.is_null() {
            return Err(crate::error::BlockchainError::OutOfMemory);
        }
        Ok(Self { inner })
    }
    
    /// Execute script with transaction context
    pub fn execute_script(
        &self,
        script: &[u8],
        transaction: &TransactionWrapper,
        input_index: usize,
    ) -> Result<bool> {
        let mut result = false;
        
        // This requires converting Rust transaction to C++ transaction
        // For now, return placeholder implementation
        Ok(true) // TODO: Implement full FFI conversion
    }
    
    /// Validate script syntax
    pub fn validate_script_syntax(&self, script: &[u8]) -> Result<bool> {
        let mut is_valid = false;
        
        let result = unsafe {
            crate::vm_validate_script_syntax(
                script.as_ptr(),
                script.len(),
                &mut is_valid
            )
        };
        check_result_with_bool(result, is_valid)
    }
    
    /// Calculate hash of script
    pub fn calculate_script_hash(&self, script: &[u8]) -> Result<Hash256Wrapper> {
        let mut script_hash = std::mem::MaybeUninit::uninit();
        
        let result = unsafe {
            crate::vm_calculate_script_hash(
                script.as_ptr(),
                script.len(),
                script_hash.as_mut_ptr()
            )
        };
        check_result(result)?;
        
        let script_hash = unsafe { script_hash.assume_init() };
        Ok(Hash256Wrapper::from_ffi(&script_hash))
    }
    
    /// Create Pay-to-Public-Key-Hash (P2PKH) script
    pub fn create_p2pkh_script(&self, pubkey_hash: &Hash160Wrapper) -> Result<Vec<u8>> {
        let mut buffer = SafeByteBuffer::with_capacity(25)?; // Standard P2PKH script size
        let hash_ffi = pubkey_hash.to_ffi();
        
        let result = unsafe {
            crate::vm_create_p2pkh_script(&hash_ffi, buffer.inner)
        };
        check_result(result)?;
        
        Ok(buffer.as_slice().to_vec())
    }
    
    /// Create Pay-to-Script-Hash (P2SH) script
    pub fn create_p2sh_script(&self, script_hash: &Hash256Wrapper) -> Result<Vec<u8>> {
        let mut buffer = SafeByteBuffer::with_capacity(23)?; // Standard P2SH script size
        let hash_ffi = script_hash.to_ffi();
        
        let result = unsafe {
            crate::vm_create_p2sh_script(&hash_ffi, buffer.inner)
        };
        check_result(result)?;
        
        Ok(buffer.as_slice().to_vec())
    }
    
    /// Create multi-signature script
    pub fn create_multisig_script(
        &self,
        pubkeys: &[PublicKeyWrapper],
        required_sigs: usize,
    ) -> Result<Vec<u8>> {
        if pubkeys.is_empty() || required_sigs == 0 || required_sigs > pubkeys.len() {
            return Err(crate::error::BlockchainError::InvalidInput(
                "Invalid multisig parameters".to_string()
            ));
        }
        
        // Convert Rust pubkeys to C pubkeys
        let ffi_pubkeys: Vec<crate::PublicKey> = pubkeys.iter()
            .map(|pk| pk.to_ffi())
            .collect();
        
        // Estimate script size (rough calculation)
        let estimated_size = 1 + pubkeys.len() * 34 + 2; // OP_M + pubkeys + OP_N + OP_CHECKMULTISIG
        let mut script_buffer = SafeByteBuffer::with_capacity(estimated_size)?;
        
        let result = unsafe {
            crate::vm_create_multisig_script(
                ffi_pubkeys.as_ptr(),
                ffi_pubkeys.len(),
                required_sigs,
                script_buffer.inner
            )
        };
        check_result(result)?;
        
        Ok(script_buffer.as_slice().to_vec())
    }
}

impl Drop for VMEngine {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            unsafe {
                crate::vm_engine_destroy(self.inner);
            }
        }
    }
}

unsafe impl Send for VMEngine {}
unsafe impl Sync for VMEngine {}

/// Script opcodes (subset of Bitcoin opcodes)
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Opcode {
    // Constants
    Op0 = 0x00,
    Op1 = 0x51,
    Op2 = 0x52,
    Op3 = 0x53,
    // ... (more opcodes would be defined here)
    
    // Stack operations
    OpDup = 0x76,
    OpDrop = 0x75,
    OpSwap = 0x7c,
    
    // Arithmetic
    OpAdd = 0x93,
    OpSub = 0x94,
    OpMul = 0x95,
    
    // Cryptography
    OpHash256 = 0xaa,
    OpChecksig = 0xac,
    OpCheckmultisig = 0xae,
    
    // Control flow
    OpIf = 0x63,
    OpElse = 0x67,
    OpEndif = 0x68,
    OpVerify = 0x69,
    OpReturn = 0x6a,
}

// Opcode aliases
pub const OpFalse: Opcode = Opcode::Op0;
pub const OpTrue: Opcode = Opcode::Op1;

/// Convenience functions for script creation
pub mod script_builder {
    use super::*;
    
    /// Build a simple P2PKH script
    pub fn build_p2pkh_script(pubkey_hash: &Hash160Wrapper) -> Result<Vec<u8>> {
        let vm = VMEngine::new()?;
        vm.create_p2pkh_script(pubkey_hash)
    }
    
    /// Build a P2SH script
    pub fn build_p2sh_script(script_hash: &Hash256Wrapper) -> Result<Vec<u8>> {
        let vm = VMEngine::new()?;
        vm.create_p2sh_script(script_hash)
    }
    
    /// Build a multisig script
    pub fn build_multisig_script(
        pubkeys: &[PublicKeyWrapper],
        required_sigs: usize,
    ) -> Result<Vec<u8>> {
        let vm = VMEngine::new()?;
        vm.create_multisig_script(pubkeys, required_sigs)
    }
    
    /// Validate script syntax
    pub fn validate_script(script: &[u8]) -> Result<bool> {
        let vm = VMEngine::new()?;
        vm.validate_script_syntax(script)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_vm_engine_creation() {
        let vm = VMEngine::new();
        assert!(vm.is_ok());
    }
    
    #[test]
    fn test_script_validation() {
        let vm = VMEngine::new().unwrap();
        
        // Test empty script (should be valid)
        let empty_script = vec![];
        // Uncomment when C++ implementation is complete
        // let is_valid = vm.validate_script_syntax(&empty_script).unwrap();
        // assert!(is_valid);
    }
    
    #[test]
    fn test_opcodes() {
        assert_eq!(Opcode::Op0 as u8, 0x00);
        assert_eq!(Opcode::OpChecksig as u8, 0xac);
        assert_eq!(Opcode::OpReturn as u8, 0x6a);
    }
}