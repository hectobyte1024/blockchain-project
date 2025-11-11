//! Error handling for FFI operations

use std::fmt;

/// Blockchain error types that can cross the FFI boundary
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockchainError {
    /// Success case (not really an error)
    Success,
    /// Invalid input parameters
    InvalidInput(String),
    /// Invalid transaction data
    InvalidTransaction(String),
    /// Invalid block data  
    InvalidBlock(String),
    /// Invalid cryptographic signature
    InvalidSignature,
    /// Storage/database error
    StorageError(String),
    /// Consensus validation error
    ConsensusError(String),
    /// Virtual machine execution error
    VmError(String),
    /// Out of memory error
    OutOfMemory,
    /// Unknown error
    Unknown(i32),
}

impl fmt::Display for BlockchainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BlockchainError::Success => write!(f, "Success"),
            BlockchainError::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
            BlockchainError::InvalidTransaction(msg) => write!(f, "Invalid transaction: {}", msg),
            BlockchainError::InvalidBlock(msg) => write!(f, "Invalid block: {}", msg),
            BlockchainError::InvalidSignature => write!(f, "Invalid cryptographic signature"),
            BlockchainError::StorageError(msg) => write!(f, "Storage error: {}", msg),
            BlockchainError::ConsensusError(msg) => write!(f, "Consensus error: {}", msg),
            BlockchainError::VmError(msg) => write!(f, "VM error: {}", msg),
            BlockchainError::OutOfMemory => write!(f, "Out of memory"),
            BlockchainError::Unknown(code) => write!(f, "Unknown error (code: {})", code),
        }
    }
}

impl std::error::Error for BlockchainError {}

/// Result type for blockchain operations
pub type Result<T> = std::result::Result<T, BlockchainError>;

/// Convert FFI result code to Rust Result
pub fn check_result(result: crate::BlockchainResult) -> Result<()> {
    match result {
        crate::BlockchainResult_BLOCKCHAIN_SUCCESS => Ok(()),
        crate::BlockchainResult_BLOCKCHAIN_ERROR_INVALID_INPUT => {
            Err(BlockchainError::InvalidInput("FFI call failed".to_string()))
        }
        crate::BlockchainResult_BLOCKCHAIN_ERROR_INVALID_TRANSACTION => {
            Err(BlockchainError::InvalidTransaction("FFI call failed".to_string()))
        }
        crate::BlockchainResult_BLOCKCHAIN_ERROR_INVALID_BLOCK => {
            Err(BlockchainError::InvalidBlock("FFI call failed".to_string()))
        }
        crate::BlockchainResult_BLOCKCHAIN_ERROR_INVALID_SIGNATURE => {
            Err(BlockchainError::InvalidSignature)
        }
        crate::BlockchainResult_BLOCKCHAIN_ERROR_STORAGE_ERROR => {
            Err(BlockchainError::StorageError("FFI call failed".to_string()))
        }
        crate::BlockchainResult_BLOCKCHAIN_ERROR_CONSENSUS_ERROR => {
            Err(BlockchainError::ConsensusError("FFI call failed".to_string()))
        }
        crate::BlockchainResult_BLOCKCHAIN_ERROR_VM_ERROR => {
            Err(BlockchainError::VmError("FFI call failed".to_string()))
        }
        crate::BlockchainResult_BLOCKCHAIN_ERROR_OUT_OF_MEMORY => {
            Err(BlockchainError::OutOfMemory)
        }
        _ => Err(BlockchainError::Unknown(result as i32)),
    }
}

/// Convert FFI result with boolean output
pub fn check_result_with_bool(result: crate::BlockchainResult, value: bool) -> Result<bool> {
    check_result(result).map(|_| value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let error = BlockchainError::InvalidTransaction("test error".to_string());
        assert_eq!(error.to_string(), "Invalid transaction: test error");
    }

    #[test] 
    fn test_check_result() {
        assert!(check_result(crate::BlockchainResult_BLOCKCHAIN_SUCCESS).is_ok());
        assert!(check_result(crate::BlockchainResult_BLOCKCHAIN_ERROR_INVALID_INPUT).is_err());
    }
}