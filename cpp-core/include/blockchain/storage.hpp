#pragma once

#include "blockchain/core.hpp"
#include "blockchain/crypto.hpp"
#include "blockchain/transaction.hpp"
#include "blockchain/block.hpp"
#include "blockchain/storage_config.hpp"
#include <memory>
#include <optional>
#include <vector>
#include <string>
#include <unordered_map>
#include <mutex>
#include <shared_mutex>
#include <atomic>

namespace blockchain {
namespace storage {

using namespace crypto;
using namespace transaction;
using namespace block;

// Forward declarations
class BlockchainDatabase;
class UTXODatabase;
class TransactionIndex;
class BlockIndex;

/// Database operation result
enum class StorageResult {
    SUCCESS = 0,
    NOT_FOUND,
    ALREADY_EXISTS,
    CORRUPTION_ERROR,
    IO_ERROR,
    INVALID_DATA,
    DATABASE_ERROR,
    INSUFFICIENT_SPACE,
    PERMISSION_ERROR
};

/// Storage configuration
struct StorageConfig {
    std::string data_directory = "./blockchain_data";
    size_t cache_size_mb = 256;           // LevelDB cache size
    size_t write_buffer_size_mb = 64;     // LevelDB write buffer
    size_t max_open_files = 1000;         // LevelDB max open files
    bool enable_compression = true;        // Enable LZ4 compression
    bool enable_bloom_filter = true;      // Enable bloom filters for faster lookups
    size_t utxo_cache_size = 100000;      // In-memory UTXO cache entries
    size_t tx_cache_size = 10000;         // Transaction cache entries
    size_t block_cache_size = 1000;       // Block cache entries
    bool enable_pruning = false;          // Enable blockchain pruning
    uint64_t prune_target_mb = 5000;      // Target size for pruned blockchain
    bool enable_txindex = true;           // Enable transaction indexing
    bool enable_addrindex = false;        // Enable address indexing
};

/// UTXO (Unspent Transaction Output) entry
struct UTXOEntry {
    Hash256 tx_hash;                      // Transaction hash containing this UTXO
    uint32_t output_index;                // Output index in transaction
    TxOutput output;                      // Output data (value + script)
    uint32_t block_height;                // Block height when created
    bool is_coinbase;                     // True if from coinbase transaction
    
    /// Serialize UTXO to bytes
    std::vector<uint8_t> serialize() const;
    
    /// Deserialize UTXO from bytes
    static std::optional<UTXOEntry> deserialize(const std::vector<uint8_t>& data);
    
    /// Get serialized size
    size_t get_serialized_size() const;
};

/// Block metadata for indexing
struct BlockMetadata {
    Hash256 block_hash;                   // Block hash
    Hash256 prev_block_hash;              // Previous block hash
    uint32_t height;                      // Block height
    uint32_t timestamp;                   // Block timestamp
    uint32_t tx_count;                    // Number of transactions
    uint64_t total_work;                  // Cumulative work (difficulty)
    size_t file_position;                 // Position in block file
    size_t block_size;                    // Block size in bytes
    
    /// Serialize metadata to bytes
    std::vector<uint8_t> serialize() const;
    
    /// Deserialize metadata from bytes
    static std::optional<BlockMetadata> deserialize(const std::vector<uint8_t>& data);
};

/// Transaction metadata for indexing
struct TransactionMetadata {
    Hash256 tx_hash;                      // Transaction hash
    Hash256 block_hash;                   // Block containing transaction
    uint32_t block_height;                // Block height
    uint32_t tx_index;                    // Index within block
    size_t file_position;                 // Position in transaction file
    size_t tx_size;                       // Transaction size in bytes
    
    /// Serialize metadata to bytes
    std::vector<uint8_t> serialize() const;
    
    /// Deserialize metadata from bytes
    static std::optional<TransactionMetadata> deserialize(const std::vector<uint8_t>& data);
};

/// Abstract storage interface
class IBlockchainStorage {
public:
    virtual ~IBlockchainStorage() = default;
    
    // Block operations
    virtual StorageResult store_block(const Block& block) = 0;
    virtual StorageResult get_block(const Hash256& block_hash, Block& block) = 0;
    virtual StorageResult get_block_by_height(uint32_t height, Block& block) = 0;
    virtual StorageResult has_block(const Hash256& block_hash) = 0;
    virtual StorageResult remove_block(const Hash256& block_hash) = 0;
    
    // Block metadata operations
    virtual StorageResult store_block_metadata(const BlockMetadata& metadata) = 0;
    virtual StorageResult get_block_metadata(const Hash256& block_hash, BlockMetadata& metadata) = 0;
    virtual StorageResult get_block_metadata_by_height(uint32_t height, BlockMetadata& metadata) = 0;
    
    // Transaction operations
    virtual StorageResult store_transaction(const Transaction& tx) = 0;
    virtual StorageResult get_transaction(const Hash256& tx_hash, Transaction& tx) = 0;
    virtual StorageResult has_transaction(const Hash256& tx_hash) = 0;
    virtual StorageResult remove_transaction(const Hash256& tx_hash) = 0;
    
    // Transaction metadata operations
    virtual StorageResult store_transaction_metadata(const TransactionMetadata& metadata) = 0;
    virtual StorageResult get_transaction_metadata(const Hash256& tx_hash, TransactionMetadata& metadata) = 0;
    
    // UTXO operations
    virtual StorageResult add_utxo(const Hash256& tx_hash, uint32_t output_index, const UTXOEntry& utxo) = 0;
    virtual StorageResult get_utxo(const Hash256& tx_hash, uint32_t output_index, UTXOEntry& utxo) = 0;
    virtual StorageResult remove_utxo(const Hash256& tx_hash, uint32_t output_index) = 0;
    virtual StorageResult has_utxo(const Hash256& tx_hash, uint32_t output_index) = 0;
    
    // Batch operations
    virtual StorageResult begin_batch() = 0;
    virtual StorageResult commit_batch() = 0;
    virtual StorageResult rollback_batch() = 0;
    
    // Statistics and maintenance
    virtual StorageResult get_blockchain_height(uint32_t& height) = 0;
    virtual StorageResult get_best_block_hash(Hash256& block_hash) = 0;
    virtual StorageResult set_best_block_hash(const Hash256& block_hash) = 0;
    virtual StorageResult get_utxo_count(size_t& count) = 0;
    virtual StorageResult get_database_size(size_t& size_bytes) = 0;
    
    // Cleanup and optimization
    virtual StorageResult compact_database() = 0;
    virtual StorageResult vacuum_database() = 0;
    virtual StorageResult repair_database() = 0;
};

/// LevelDB-based blockchain storage implementation
class LevelDBStorage : public IBlockchainStorage {
public:
    explicit LevelDBStorage(const StorageConfig& config);
    ~LevelDBStorage();
    
    /// Initialize storage (create directories, open databases)
    StorageResult initialize();
    
    /// Close all databases and cleanup
    void shutdown();
    
    /// Check if storage is initialized
    bool is_initialized() const { return initialized_; }
    
    // IBlockchainStorage implementation
    StorageResult store_block(const Block& block) override;
    StorageResult get_block(const Hash256& block_hash, Block& block) override;
    StorageResult get_block_by_height(uint32_t height, Block& block) override;
    StorageResult has_block(const Hash256& block_hash) override;
    StorageResult remove_block(const Hash256& block_hash) override;
    
    StorageResult store_block_metadata(const BlockMetadata& metadata) override;
    StorageResult get_block_metadata(const Hash256& block_hash, BlockMetadata& metadata) override;
    StorageResult get_block_metadata_by_height(uint32_t height, BlockMetadata& metadata) override;
    
    StorageResult store_transaction(const Transaction& tx) override;
    StorageResult get_transaction(const Hash256& tx_hash, Transaction& tx) override;
    StorageResult has_transaction(const Hash256& tx_hash) override;
    StorageResult remove_transaction(const Hash256& tx_hash) override;
    
    StorageResult store_transaction_metadata(const TransactionMetadata& metadata) override;
    StorageResult get_transaction_metadata(const Hash256& tx_hash, TransactionMetadata& metadata) override;
    
    StorageResult add_utxo(const Hash256& tx_hash, uint32_t output_index, const UTXOEntry& utxo) override;
    StorageResult get_utxo(const Hash256& tx_hash, uint32_t output_index, UTXOEntry& utxo) override;
    StorageResult remove_utxo(const Hash256& tx_hash, uint32_t output_index) override;
    StorageResult has_utxo(const Hash256& tx_hash, uint32_t output_index) override;
    
    StorageResult begin_batch() override;
    StorageResult commit_batch() override;
    StorageResult rollback_batch() override;
    
    StorageResult get_blockchain_height(uint32_t& height) override;
    StorageResult get_best_block_hash(Hash256& block_hash) override;
    StorageResult set_best_block_hash(const Hash256& block_hash) override;
    StorageResult get_utxo_count(size_t& count) override;
    StorageResult get_database_size(size_t& size_bytes) override;
    
    StorageResult compact_database() override;
    StorageResult vacuum_database() override;
    StorageResult repair_database() override;
    
    /// Get storage statistics
    struct StorageStats {
        size_t total_blocks;
        size_t total_transactions;
        size_t total_utxos;
        size_t database_size_bytes;
        size_t cache_hit_rate;
        size_t cache_miss_rate;
    };
    
    StorageStats get_stats() const;

private:
    // Internal implementation details
    class Impl;
    std::unique_ptr<Impl> impl_;
    
    StorageConfig config_;
    std::atomic<bool> initialized_{false};
    mutable std::shared_mutex mutex_;
    
    // Cache management
    void update_cache_stats();
    void cleanup_caches();
    
    // Key generation utilities
    std::string make_block_key(const Hash256& block_hash);
    std::string make_block_height_key(uint32_t height);
    std::string make_tx_key(const Hash256& tx_hash);
    std::string make_utxo_key(const Hash256& tx_hash, uint32_t output_index);
    std::string make_metadata_key(const std::string& prefix, const Hash256& hash);
};

/// In-memory storage implementation for testing
class MemoryStorage : public IBlockchainStorage {
public:
    MemoryStorage() = default;
    ~MemoryStorage() = default;
    
    // IBlockchainStorage implementation
    StorageResult store_block(const Block& block) override;
    StorageResult get_block(const Hash256& block_hash, Block& block) override;
    StorageResult get_block_by_height(uint32_t height, Block& block) override;
    StorageResult has_block(const Hash256& block_hash) override;
    StorageResult remove_block(const Hash256& block_hash) override;
    
    StorageResult store_block_metadata(const BlockMetadata& metadata) override;
    StorageResult get_block_metadata(const Hash256& block_hash, BlockMetadata& metadata) override;
    StorageResult get_block_metadata_by_height(uint32_t height, BlockMetadata& metadata) override;
    
    StorageResult store_transaction(const Transaction& tx) override;
    StorageResult get_transaction(const Hash256& tx_hash, Transaction& tx) override;
    StorageResult has_transaction(const Hash256& tx_hash) override;
    StorageResult remove_transaction(const Hash256& tx_hash) override;
    
    StorageResult store_transaction_metadata(const TransactionMetadata& metadata) override;
    StorageResult get_transaction_metadata(const Hash256& tx_hash, TransactionMetadata& metadata) override;
    
    StorageResult add_utxo(const Hash256& tx_hash, uint32_t output_index, const UTXOEntry& utxo) override;
    StorageResult get_utxo(const Hash256& tx_hash, uint32_t output_index, UTXOEntry& utxo) override;
    StorageResult remove_utxo(const Hash256& tx_hash, uint32_t output_index) override;
    StorageResult has_utxo(const Hash256& tx_hash, uint32_t output_index) override;
    
    StorageResult begin_batch() override;
    StorageResult commit_batch() override;
    StorageResult rollback_batch() override;
    
    StorageResult get_blockchain_height(uint32_t& height) override;
    StorageResult get_best_block_hash(Hash256& block_hash) override;
    StorageResult set_best_block_hash(const Hash256& block_hash) override;
    StorageResult get_utxo_count(size_t& count) override;
    StorageResult get_database_size(size_t& size_bytes) override;
    
    StorageResult compact_database() override;
    StorageResult vacuum_database() override;
    StorageResult repair_database() override;
    
    /// Clear all data (for testing)
    void clear();

private:
    std::unordered_map<Hash256, Block, crypto::Hash256Hasher> blocks_;
    std::unordered_map<uint32_t, Hash256> height_to_hash_;
    std::unordered_map<Hash256, BlockMetadata, crypto::Hash256Hasher> block_metadata_;
    std::unordered_map<Hash256, Transaction, crypto::Hash256Hasher> transactions_;
    std::unordered_map<Hash256, TransactionMetadata, crypto::Hash256Hasher> tx_metadata_;
    std::unordered_map<std::string, UTXOEntry> utxos_;  // key: tx_hash + output_index
    Hash256 best_block_hash_;
    mutable std::shared_mutex mutex_;
    
    std::string make_utxo_key(const Hash256& tx_hash, uint32_t output_index);
};

/// Storage factory for creating different storage implementations
class StorageFactory {
public:
    enum class StorageType {
        MEMORY,
        LEVELDB
    };
    
    /// Create storage instance
    static std::unique_ptr<IBlockchainStorage> create(StorageType type, const StorageConfig& config = StorageConfig{});
    
    /// Create default storage (LevelDB with default config)
    static std::unique_ptr<IBlockchainStorage> create_default();
    
    /// Create test storage (in-memory)
    static std::unique_ptr<IBlockchainStorage> create_test();
};

/// UTXO set manager for fast UTXO operations
class UTXOManager {
public:
    explicit UTXOManager(std::shared_ptr<IBlockchainStorage> storage);
    ~UTXOManager() = default;
    
    /// Add UTXO from transaction output
    bool add_utxo(const Transaction& tx, uint32_t output_index, uint32_t block_height);
    
    /// Remove UTXO (when spent)
    bool remove_utxo(const Hash256& tx_hash, uint32_t output_index);
    
    /// Get UTXO entry
    std::optional<UTXOEntry> get_utxo(const Hash256& tx_hash, uint32_t output_index);
    
    /// Check if UTXO exists
    bool has_utxo(const Hash256& tx_hash, uint32_t output_index);
    
    /// Get total UTXO count
    size_t get_utxo_count() const;
    
    /// Get UTXOs for address (requires address indexing)
    std::vector<UTXOEntry> get_utxos_for_address(const std::string& address);
    
    /// Calculate total value in UTXO set
    uint64_t get_total_value() const;
    
    /// Validate UTXO set consistency
    bool validate_utxo_set();
    
    /// Flush cache to storage
    bool flush();

private:
    std::shared_ptr<IBlockchainStorage> storage_;
    std::unordered_map<std::string, UTXOEntry> utxo_cache_;  // LRU cache
    mutable std::shared_mutex cache_mutex_;
    size_t cache_size_limit_;
    
    std::string make_cache_key(const Hash256& tx_hash, uint32_t output_index);
    void evict_cache_entries();
};

/// Blockchain storage manager combining all storage components
class BlockchainStorageManager {
public:
    explicit BlockchainStorageManager(const StorageConfig& config);
    ~BlockchainStorageManager();
    
    /// Initialize all storage components
    bool initialize();
    
    /// Shutdown and cleanup
    void shutdown();
    
    /// Get storage interface
    std::shared_ptr<IBlockchainStorage> get_storage() { return storage_; }
    
    /// Get UTXO manager
    std::shared_ptr<UTXOManager> get_utxo_manager() { return utxo_manager_; }
    
    /// Store complete block with all updates
    bool store_block_atomic(const Block& block, uint32_t height);
    
    /// Remove block and revert all changes
    bool remove_block_atomic(const Hash256& block_hash);
    
    /// Get blockchain statistics
    struct BlockchainStats {
        uint32_t height;
        Hash256 best_block_hash;
        size_t total_blocks;
        size_t total_transactions;
        size_t total_utxos;
        uint64_t total_value;
        size_t database_size_bytes;
    };
    
    BlockchainStats get_blockchain_stats();

private:
    StorageConfig config_;
    std::shared_ptr<IBlockchainStorage> storage_;
    std::shared_ptr<UTXOManager> utxo_manager_;
    std::atomic<bool> initialized_{false};
};

} // namespace storage
} // namespace blockchain