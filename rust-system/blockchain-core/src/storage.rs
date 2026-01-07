//! Disk-based block storage module
//! 
//! Implements Bitcoin-style sequential block storage with blk*.dat files.
//! This provides efficient I/O for blockchain synchronization and storage.

use crate::{Hash256, BlockHeight, Result, BlockchainError, block::Block};
use std::collections::HashMap;
use std::fs::{File, OpenOptions, create_dir_all};
use std::io::{Read, Write, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};
use tracing::{info, warn, error, debug};

/// Maximum size for each block file (2GB)
const MAX_BLOCK_FILE_SIZE: u64 = 2_000_000_000;

/// Block file prefix
const BLOCK_FILE_PREFIX: &str = "blk";

/// Block file extension
const BLOCK_FILE_EXT: &str = "dat";

/// Block location in storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockLocation {
    /// File number (e.g., 0 for blk00000.dat)
    pub file_num: u32,
    /// Byte offset within the file
    pub offset: u64,
    /// Size of the block in bytes
    pub size: u32,
}

/// Block index entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockIndexEntry {
    /// Block hash
    pub hash: Hash256,
    /// Block height
    pub height: BlockHeight,
    /// Storage location
    pub location: BlockLocation,
    /// Previous block hash
    pub prev_hash: Hash256,
    /// Timestamp
    pub timestamp: u64,
}

/// Disk-based block storage engine
pub struct DiskBlockStorage {
    /// Base directory for block files
    data_dir: PathBuf,
    /// Current active file handle
    current_file: Arc<RwLock<Option<File>>>,
    /// Current file number
    current_file_num: Arc<RwLock<u32>>,
    /// Current file size
    current_file_size: Arc<RwLock<u64>>,
    /// Block index: hash -> location
    block_index: Arc<RwLock<HashMap<Hash256, BlockIndexEntry>>>,
    /// Height index: height -> hash
    height_index: Arc<RwLock<HashMap<BlockHeight, Hash256>>>,
}

impl DiskBlockStorage {
    /// Create a new disk block storage
    pub fn new<P: AsRef<Path>>(data_dir: P) -> Result<Self> {
        let data_dir = data_dir.as_ref().to_path_buf();
        
        // Create data directory if it doesn't exist
        create_dir_all(&data_dir)
            .map_err(|e| BlockchainError::InvalidInput(format!("Failed to create data dir: {}", e)))?;

        info!("📁 Initializing disk block storage at: {}", data_dir.display());

        let storage = Self {
            data_dir,
            current_file: Arc::new(RwLock::new(None)),
            current_file_num: Arc::new(RwLock::new(0)),
            current_file_size: Arc::new(RwLock::new(0)),
            block_index: Arc::new(RwLock::new(HashMap::new())),
            height_index: Arc::new(RwLock::new(HashMap::new())),
        };

        // Load existing block index
        storage.load_block_index()?;

        Ok(storage)
    }

    
    pub async fn write_block(&self, block: &Block) -> Result<BlockLocation> {
        let block_hash = block.header.calculate_hash();
        let block_height = block.header.height;

        
        let block_bytes = self.serialize_block(block)?;
        let block_size = block_bytes.len() as u32;

        debug!("💾 Writing block {} (height {}, {} bytes)", 
               hex::encode(&block_hash), block_height, block_size);

        
        let current_size = *self.current_file_size.read().await;
        if current_size + block_size as u64 > MAX_BLOCK_FILE_SIZE {
            self.rotate_file().await?;
        }

        
        let mut file_guard = self.current_file.write().await;
        if file_guard.is_none() {
            *file_guard = Some(self.open_current_file().await?);
        }

        let file = file_guard.as_mut().unwrap();
        let file_num = *self.current_file_num.read().await;

        // Get current offset (end of file)
        let offset = file.seek(SeekFrom::End(0))
            .map_err(|e| BlockchainError::InvalidInput(format!("Seek error: {}", e)))?;

        // Write block to file
        file.write_all(&block_bytes)
            .map_err(|e| BlockchainError::InvalidInput(format!("Write error: {}", e)))?;

        file.flush()
            .map_err(|e| BlockchainError::InvalidInput(format!("Flush error: {}", e)))?;

        // Update file size
        let mut size_guard = self.current_file_size.write().await;
        *size_guard += block_size as u64;

        let location = BlockLocation {
            file_num,
            offset,
            size: block_size,
        };

        // Update block index
        let index_entry = BlockIndexEntry {
            hash: block_hash,
            height: block_height as u64,
            location: location.clone(),
            prev_hash: block.header.prev_block_hash,
            timestamp: block.header.timestamp as u64,
        };

        let mut block_index = self.block_index.write().await;
        block_index.insert(block_hash, index_entry);

        let mut height_index = self.height_index.write().await;
        height_index.insert(block_height as u64, block_hash);

        debug!("✅ Block written to file {} at offset {}", file_num, offset);

        Ok(location)
    }

    /// Read a block from disk storage
    pub async fn read_block(&self, location: &BlockLocation) -> Result<Block> {
        debug!("📖 Reading block from file {} at offset {}", 
               location.file_num, location.offset);

        let file_path = self.get_block_file_path(location.file_num);
        
        let mut file = File::open(&file_path)
            .map_err(|e| BlockchainError::InvalidInput(format!("Failed to open file: {}", e)))?;

        // Seek to block position
        file.seek(SeekFrom::Start(location.offset))
            .map_err(|e| BlockchainError::InvalidInput(format!("Seek error: {}", e)))?;

        // Read block data
        let mut buffer = vec![0u8; location.size as usize];
        file.read_exact(&mut buffer)
            .map_err(|e| BlockchainError::InvalidInput(format!("Read error: {}", e)))?;

        // Deserialize block
        self.deserialize_block(&buffer)
    }

    /// Read a block by its hash
    pub async fn read_block_by_hash(&self, hash: &Hash256) -> Result<Option<Block>> {
        let block_index = self.block_index.read().await;
        
        if let Some(entry) = block_index.get(hash) {
            let location = entry.location.clone();
            drop(block_index); // Release lock before reading
            
            match self.read_block(&location).await {
                Ok(block) => Ok(Some(block)),
                Err(e) => {
                    error!("Failed to read block {}: {}", hex::encode(hash), e);
                    Ok(None)
                }
            }
        } else {
            Ok(None)
        }
    }

    /// Read a block by its height
    pub async fn read_block_by_height(&self, height: BlockHeight) -> Result<Option<Block>> {
        let height_index = self.height_index.read().await;
        
        if let Some(hash) = height_index.get(&height) {
            let hash = *hash;
            drop(height_index); // Release lock before reading
            
            self.read_block_by_hash(&hash).await
        } else {
            Ok(None)
        }
    }

    /// Get block location by hash
    pub async fn get_block_location(&self, hash: &Hash256) -> Option<BlockLocation> {
        let block_index = self.block_index.read().await;
        block_index.get(hash).map(|entry| entry.location.clone())
    }

    /// Get blockchain height (highest block stored)
    pub async fn get_height(&self) -> BlockHeight {
        let height_index = self.height_index.read().await;
        height_index.keys().copied().max().unwrap_or(0)
    }

    /// Check if a block exists
    pub async fn has_block(&self, hash: &Hash256) -> bool {
        let block_index = self.block_index.read().await;
        block_index.contains_key(hash)
    }

    /// Get block header info without reading full block
    pub async fn get_block_header(&self, hash: &Hash256) -> Option<BlockIndexEntry> {
        let block_index = self.block_index.read().await;
        block_index.get(hash).cloned()
    }

    /// Get total size of blockchain on disk
    pub async fn get_total_size(&self) -> u64 {
        let current_file_num = *self.current_file_num.read().await;
        let mut total_size = 0u64;

        for file_num in 0..=current_file_num {
            let file_path = self.get_block_file_path(file_num);
            if let Ok(metadata) = std::fs::metadata(&file_path) {
                total_size += metadata.len();
            }
        }

        total_size
    }

    /// Get number of blocks stored
    pub async fn get_block_count(&self) -> usize {
        let block_index = self.block_index.read().await;
        block_index.len()
    }

    /// Rotate to a new block file
    async fn rotate_file(&self) -> Result<()> {
        let mut file_guard = self.current_file.write().await;
        let mut file_num_guard = self.current_file_num.write().await;
        let mut size_guard = self.current_file_size.write().await;

        // Close current file
        *file_guard = None;

        // Increment file number
        *file_num_guard += 1;
        *size_guard = 0;

        info!("🔄 Rotating to new block file: blk{:05}.dat", *file_num_guard);

        Ok(())
    }

    /// Open the current block file
    async fn open_current_file(&self) -> Result<File> {
        let file_num = *self.current_file_num.read().await;
        let file_path = self.get_block_file_path(file_num);

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&file_path)
            .map_err(|e| BlockchainError::InvalidInput(format!("Failed to open file: {}", e)))?;

        // Get current file size
        let metadata = file.metadata()
            .map_err(|e| BlockchainError::InvalidInput(format!("Failed to get metadata: {}", e)))?;
        
        let mut size_guard = self.current_file_size.write().await;
        *size_guard = metadata.len();

        Ok(file)
    }

    /// Get path to a block file
    fn get_block_file_path(&self, file_num: u32) -> PathBuf {
        self.data_dir.join(format!("{}{:05}.{}", BLOCK_FILE_PREFIX, file_num, BLOCK_FILE_EXT))
    }

    /// Serialize a block to bytes
    fn serialize_block(&self, block: &Block) -> Result<Vec<u8>> {
        bincode::serialize(block)
            .map_err(|e| BlockchainError::SerializationError(format!("Block serialization failed: {}", e)))
    }

    /// Deserialize a block from bytes
    fn deserialize_block(&self, data: &[u8]) -> Result<Block> {
        bincode::deserialize(data)
            .map_err(|e| BlockchainError::SerializationError(format!("Block deserialization failed: {}", e)))
    }

    /// Load existing block index from disk
    fn load_block_index(&self) -> Result<()> {
        // TODO: Implement persistent index storage (e.g., using RocksDB or LevelDB)
        // For now, we'll rebuild the index by scanning block files on startup
        
        info!("📚 Loading block index...");
        
        // This would scan existing blk*.dat files and rebuild the index
        // For production, use a persistent key-value store like RocksDB
        
        Ok(())
    }

    /// Save block index to disk
    pub async fn save_index(&self) -> Result<()> {
        // TODO: Implement persistent index storage
        // For now, index is kept in memory
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_disk_storage_creation() {
        let temp_dir = TempDir::new().unwrap();
        let storage = DiskBlockStorage::new(temp_dir.path()).unwrap();
        
        assert_eq!(storage.get_height().await, 0);
        assert_eq!(storage.get_block_count().await, 0);
    }

    #[tokio::test]
    async fn test_block_file_path() {
        let temp_dir = TempDir::new().unwrap();
        let storage = DiskBlockStorage::new(temp_dir.path()).unwrap();
        
        let path = storage.get_block_file_path(0);
        assert!(path.to_string_lossy().contains("blk00000.dat"));
        
        let path = storage.get_block_file_path(42);
        assert!(path.to_string_lossy().contains("blk00042.dat"));
    }
}
