#include "blockchain/storage.hpp"
#include "blockchain/storage_config.hpp"
#include "blockchain/crypto.hpp"
#include <filesystem>
#include <fstream>
#include <sstream>
#include <cstring>
#include <algorithm>

#if HAVE_LEVELDB
#include <leveldb/db.h>
#include <leveldb/options.h>
#include <leveldb/write_batch.h>
#include <leveldb/cache.h>
#include <leveldb/filter_policy.h>
#endif

namespace blockchain {
namespace storage {

namespace fs = std::filesystem;
using namespace crypto;

// Utility functions for serialization
namespace {
    
/// Write value to buffer
template<typename T>
void write_value(std::vector<uint8_t>& buffer, const T& value) {
    const auto* bytes = reinterpret_cast<const uint8_t*>(&value);
    buffer.insert(buffer.end(), bytes, bytes + sizeof(T));
}

/// Read value from buffer
template<typename T>
bool read_value(const uint8_t*& data, size_t& remaining, T& value) {
    if (remaining < sizeof(T)) return false;
    std::memcpy(&value, data, sizeof(T));
    data += sizeof(T);
    remaining -= sizeof(T);
    return true;
}

/// Write vector to buffer
template<typename T>
void write_vector(std::vector<uint8_t>& buffer, const std::vector<T>& vec) {
    write_value(buffer, static_cast<uint32_t>(vec.size()));
    for (const auto& item : vec) {
        write_value(buffer, item);
    }
}

/// Read vector from buffer
template<typename T>
bool read_vector(const uint8_t*& data, size_t& remaining, std::vector<T>& vec) {
    uint32_t size;
    if (!read_value(data, remaining, size)) return false;
    
    vec.clear();
    vec.reserve(size);
    for (uint32_t i = 0; i < size; ++i) {
        T item;
        if (!read_value(data, remaining, item)) return false;
        vec.push_back(item);
    }
    return true;
}

/// Write Hash256 to buffer
void write_hash(std::vector<uint8_t>& buffer, const Hash256& hash) {
    buffer.insert(buffer.end(), hash.begin(), hash.end());
}

/// Read Hash256 from buffer
bool read_hash(const uint8_t*& data, size_t& remaining, Hash256& hash) {
    if (remaining < 32) return false;
    std::copy(data, data + 32, hash.begin());
    data += 32;
    remaining -= 32;
    return true;
}

/// Write string to buffer
void write_string(std::vector<uint8_t>& buffer, const std::string& str) {
    write_value(buffer, static_cast<uint32_t>(str.size()));
    buffer.insert(buffer.end(), str.begin(), str.end());
}

/// Read string from buffer
bool read_string(const uint8_t*& data, size_t& remaining, std::string& str) {
    uint32_t size;
    if (!read_value(data, remaining, size)) return false;
    if (remaining < size) return false;
    
    str.assign(reinterpret_cast<const char*>(data), size);
    data += size;
    remaining -= size;
    return true;
}

} // anonymous namespace

// UTXOEntry implementation
std::vector<uint8_t> UTXOEntry::serialize() const {
    std::vector<uint8_t> buffer;
    buffer.reserve(256); // Reasonable initial size
    
    write_hash(buffer, tx_hash);
    write_value(buffer, output_index);
    
    // Serialize TxOutput
    write_value(buffer, output.value);
    write_vector(buffer, output.script_pubkey);
    
    write_value(buffer, block_height);
    write_value(buffer, static_cast<uint8_t>(is_coinbase ? 1 : 0));
    
    return buffer;
}

std::optional<UTXOEntry> UTXOEntry::deserialize(const std::vector<uint8_t>& data) {
    if (data.empty()) return std::nullopt;
    
    const uint8_t* ptr = data.data();
    size_t remaining = data.size();
    
    UTXOEntry entry;
    
    if (!read_hash(ptr, remaining, entry.tx_hash)) return std::nullopt;
    if (!read_value(ptr, remaining, entry.output_index)) return std::nullopt;
    if (!read_value(ptr, remaining, entry.output.value)) return std::nullopt;
    if (!read_vector(ptr, remaining, entry.output.script_pubkey)) return std::nullopt;
    if (!read_value(ptr, remaining, entry.block_height)) return std::nullopt;
    
    uint8_t coinbase_flag;
    if (!read_value(ptr, remaining, coinbase_flag)) return std::nullopt;
    entry.is_coinbase = (coinbase_flag != 0);
    
    return entry;
}

size_t UTXOEntry::get_serialized_size() const {
    return 32 + sizeof(uint32_t) + sizeof(uint64_t) + 
           sizeof(uint32_t) + output.script_pubkey.size() + 
           sizeof(uint32_t) + sizeof(uint8_t);
}

// BlockMetadata implementation
std::vector<uint8_t> BlockMetadata::serialize() const {
    std::vector<uint8_t> buffer;
    buffer.reserve(128);
    
    write_hash(buffer, block_hash);
    write_hash(buffer, prev_block_hash);
    write_value(buffer, height);
    write_value(buffer, timestamp);
    write_value(buffer, tx_count);
    write_value(buffer, total_work);
    write_value(buffer, file_position);
    write_value(buffer, block_size);
    
    return buffer;
}

std::optional<BlockMetadata> BlockMetadata::deserialize(const std::vector<uint8_t>& data) {
    if (data.empty()) return std::nullopt;
    
    const uint8_t* ptr = data.data();
    size_t remaining = data.size();
    
    BlockMetadata metadata;
    
    if (!read_hash(ptr, remaining, metadata.block_hash)) return std::nullopt;
    if (!read_hash(ptr, remaining, metadata.prev_block_hash)) return std::nullopt;
    if (!read_value(ptr, remaining, metadata.height)) return std::nullopt;
    if (!read_value(ptr, remaining, metadata.timestamp)) return std::nullopt;
    if (!read_value(ptr, remaining, metadata.tx_count)) return std::nullopt;
    if (!read_value(ptr, remaining, metadata.total_work)) return std::nullopt;
    if (!read_value(ptr, remaining, metadata.file_position)) return std::nullopt;
    if (!read_value(ptr, remaining, metadata.block_size)) return std::nullopt;
    
    return metadata;
}

// TransactionMetadata implementation
std::vector<uint8_t> TransactionMetadata::serialize() const {
    std::vector<uint8_t> buffer;
    buffer.reserve(96);
    
    write_hash(buffer, tx_hash);
    write_hash(buffer, block_hash);
    write_value(buffer, block_height);
    write_value(buffer, tx_index);
    write_value(buffer, file_position);
    write_value(buffer, tx_size);
    
    return buffer;
}

std::optional<TransactionMetadata> TransactionMetadata::deserialize(const std::vector<uint8_t>& data) {
    if (data.empty()) return std::nullopt;
    
    const uint8_t* ptr = data.data();
    size_t remaining = data.size();
    
    TransactionMetadata metadata;
    
    if (!read_hash(ptr, remaining, metadata.tx_hash)) return std::nullopt;
    if (!read_hash(ptr, remaining, metadata.block_hash)) return std::nullopt;
    if (!read_value(ptr, remaining, metadata.block_height)) return std::nullopt;
    if (!read_value(ptr, remaining, metadata.tx_index)) return std::nullopt;
    if (!read_value(ptr, remaining, metadata.file_position)) return std::nullopt;
    if (!read_value(ptr, remaining, metadata.tx_size)) return std::nullopt;
    
    return metadata;
}

// LevelDBStorage::Impl - Private implementation
class LevelDBStorage::Impl {
public:
    leveldb::DB* blocks_db = nullptr;
    leveldb::DB* metadata_db = nullptr;
    leveldb::DB* transactions_db = nullptr;
    leveldb::DB* utxos_db = nullptr;
    leveldb::DB* index_db = nullptr;
    
    std::unique_ptr<leveldb::WriteBatch> current_batch;
    leveldb::WriteOptions write_options;
    leveldb::ReadOptions read_options;
    
    // Caches
    std::shared_ptr<leveldb::Cache> cache;
    const leveldb::FilterPolicy* filter_policy = nullptr;
    
    // Statistics
    mutable std::atomic<size_t> cache_hits{0};
    mutable std::atomic<size_t> cache_misses{0};
    
    ~Impl() {
        delete blocks_db;
        delete metadata_db;
        delete transactions_db;
        delete utxos_db;
        delete index_db;
        delete filter_policy;
    }
};

// LevelDBStorage implementation
LevelDBStorage::LevelDBStorage(const StorageConfig& config) 
    : impl_(std::make_unique<Impl>()), config_(config) {
    
    // Configure LevelDB options
    impl_->write_options.sync = true;
    impl_->read_options.verify_checksums = true;
}

LevelDBStorage::~LevelDBStorage() {
    if (initialized_) {
        shutdown();
    }
}

StorageResult LevelDBStorage::initialize() {
    std::lock_guard<std::shared_mutex> lock(mutex_);
    
    if (initialized_) {
        return StorageResult::ALREADY_EXISTS;
    }
    
#if !HAVE_LEVELDB
    // Fallback to memory-only storage when LevelDB is not available
    return StorageResult::DATABASE_ERROR;
#else
    
    try {
        // Create data directory
        if (!fs::exists(config_.data_directory)) {
            fs::create_directories(config_.data_directory);
        }
        
        // Setup LevelDB options
        leveldb::Options options;
        options.create_if_missing = true;
        options.error_if_exists = false;
        
        // Configure cache
        impl_->cache.reset(leveldb::NewLRUCache(config_.cache_size_mb * 1024 * 1024));
        options.block_cache = impl_->cache.get();
        
        // Configure bloom filter
        if (config_.enable_bloom_filter) {
            impl_->filter_policy = leveldb::NewBloomFilterPolicy(10);
            options.filter_policy = impl_->filter_policy;
        }
        
        // Configure other options
        options.write_buffer_size = config_.write_buffer_size_mb * 1024 * 1024;
        options.max_open_files = static_cast<int>(config_.max_open_files);
        
        if (config_.enable_compression) {
            options.compression = leveldb::kSnappyCompression;
        } else {
            options.compression = leveldb::kNoCompression;
        }
        
        // Open databases
        leveldb::Status status;
        
        std::string blocks_path = config_.data_directory + "/blocks";
        status = leveldb::DB::Open(options, blocks_path, &impl_->blocks_db);
        if (!status.ok()) {
            return StorageResult::DATABASE_ERROR;
        }
        
        std::string metadata_path = config_.data_directory + "/metadata";
        status = leveldb::DB::Open(options, metadata_path, &impl_->metadata_db);
        if (!status.ok()) {
            return StorageResult::DATABASE_ERROR;
        }
        
        std::string transactions_path = config_.data_directory + "/transactions";
        status = leveldb::DB::Open(options, transactions_path, &impl_->transactions_db);
        if (!status.ok()) {
            return StorageResult::DATABASE_ERROR;
        }
        
        std::string utxos_path = config_.data_directory + "/utxos";
        status = leveldb::DB::Open(options, utxos_path, &impl_->utxos_db);
        if (!status.ok()) {
            return StorageResult::DATABASE_ERROR;
        }
        
        std::string index_path = config_.data_directory + "/index";
        status = leveldb::DB::Open(options, index_path, &impl_->index_db);
        if (!status.ok()) {
            return StorageResult::DATABASE_ERROR;
        }
        
        initialized_ = true;
        return StorageResult::SUCCESS;
        
    } catch (const std::exception& e) {
        return StorageResult::IO_ERROR;
    }
#endif
}

void LevelDBStorage::shutdown() {
    std::lock_guard<std::shared_mutex> lock(mutex_);
    
    if (!initialized_) return;
    
    // Cancel any pending batch
    impl_->current_batch.reset();
    
    // Close all databases
    delete impl_->blocks_db; impl_->blocks_db = nullptr;
    delete impl_->metadata_db; impl_->metadata_db = nullptr;
    delete impl_->transactions_db; impl_->transactions_db = nullptr;
    delete impl_->utxos_db; impl_->utxos_db = nullptr;
    delete impl_->index_db; impl_->index_db = nullptr;
    
    initialized_ = false;
}

StorageResult LevelDBStorage::store_block(const Block& block) {
    if (!initialized_) return StorageResult::DATABASE_ERROR;
    
    std::shared_lock<std::shared_mutex> lock(mutex_);
    
    try {
        Hash256 block_hash = block.calculate_hash();
        std::string key = make_block_key(block_hash);
        
        // Serialize block
        std::vector<uint8_t> serialized_block = block.serialize();
        std::string value(serialized_block.begin(), serialized_block.end());
        
        leveldb::Status status = impl_->blocks_db->Put(impl_->write_options, key, value);
        if (!status.ok()) {
            return StorageResult::DATABASE_ERROR;
        }
        
        // Store height mapping
        std::string height_key = make_block_height_key(block.header.height);
        std::string hash_value(block_hash.begin(), block_hash.end());
        status = impl_->index_db->Put(impl_->write_options, height_key, hash_value);
        if (!status.ok()) {
            return StorageResult::DATABASE_ERROR;
        }
        
        return StorageResult::SUCCESS;
        
    } catch (const std::exception& e) {
        return StorageResult::IO_ERROR;
    }
}

StorageResult LevelDBStorage::get_block(const Hash256& block_hash, Block& block) {
    if (!initialized_) return StorageResult::DATABASE_ERROR;
    
    std::shared_lock<std::shared_mutex> lock(mutex_);
    
    try {
        std::string key = make_block_key(block_hash);
        std::string value;
        
        leveldb::Status status = impl_->blocks_db->Get(impl_->read_options, key, &value);
        if (status.IsNotFound()) {
            impl_->cache_misses++;
            return StorageResult::NOT_FOUND;
        }
        if (!status.ok()) {
            return StorageResult::DATABASE_ERROR;
        }
        
        impl_->cache_hits++;
        
        // Deserialize block
        std::vector<uint8_t> data(value.begin(), value.end());
        auto deserialized_block = Block::deserialize(data);
        if (!deserialized_block) {
            return StorageResult::CORRUPTION_ERROR;
        }
        
        block = *deserialized_block;
        return StorageResult::SUCCESS;
        
    } catch (const std::exception& e) {
        return StorageResult::IO_ERROR;
    }
}

StorageResult LevelDBStorage::get_block_by_height(uint32_t height, Block& block) {
    if (!initialized_) return StorageResult::DATABASE_ERROR;
    
    std::shared_lock<std::shared_mutex> lock(mutex_);
    
    try {
        // Get hash from height index
        std::string height_key = make_block_height_key(height);
        std::string hash_value;
        
        leveldb::Status status = impl_->index_db->Get(impl_->read_options, height_key, &hash_value);
        if (status.IsNotFound()) {
            return StorageResult::NOT_FOUND;
        }
        if (!status.ok()) {
            return StorageResult::DATABASE_ERROR;
        }
        
        // Convert to Hash256
        if (hash_value.size() != 32) {
            return StorageResult::CORRUPTION_ERROR;
        }
        
        Hash256 block_hash;
        std::copy(hash_value.begin(), hash_value.end(), block_hash.begin());
        
        // Get block by hash
        return get_block(block_hash, block);
        
    } catch (const std::exception& e) {
        return StorageResult::IO_ERROR;
    }
}

StorageResult LevelDBStorage::has_block(const Hash256& block_hash) {
    if (!initialized_) return StorageResult::DATABASE_ERROR;
    
    std::shared_lock<std::shared_mutex> lock(mutex_);
    
    try {
        std::string key = make_block_key(block_hash);
        std::string value;
        
        leveldb::Status status = impl_->blocks_db->Get(impl_->read_options, key, &value);
        if (status.IsNotFound()) {
            return StorageResult::NOT_FOUND;
        }
        if (!status.ok()) {
            return StorageResult::DATABASE_ERROR;
        }
        
        return StorageResult::SUCCESS;
        
    } catch (const std::exception& e) {
        return StorageResult::IO_ERROR;
    }
}

StorageResult LevelDBStorage::add_utxo(const Hash256& tx_hash, uint32_t output_index, const UTXOEntry& utxo) {
    if (!initialized_) return StorageResult::DATABASE_ERROR;
    
    std::shared_lock<std::shared_mutex> lock(mutex_);
    
    try {
        std::string key = make_utxo_key(tx_hash, output_index);
        
        // Serialize UTXO
        std::vector<uint8_t> serialized_utxo = utxo.serialize();
        std::string value(serialized_utxo.begin(), serialized_utxo.end());
        
        leveldb::Status status = impl_->utxos_db->Put(impl_->write_options, key, value);
        if (!status.ok()) {
            return StorageResult::DATABASE_ERROR;
        }
        
        return StorageResult::SUCCESS;
        
    } catch (const std::exception& e) {
        return StorageResult::IO_ERROR;
    }
}

StorageResult LevelDBStorage::get_utxo(const Hash256& tx_hash, uint32_t output_index, UTXOEntry& utxo) {
    if (!initialized_) return StorageResult::DATABASE_ERROR;
    
    std::shared_lock<std::shared_mutex> lock(mutex_);
    
    try {
        std::string key = make_utxo_key(tx_hash, output_index);
        std::string value;
        
        leveldb::Status status = impl_->utxos_db->Get(impl_->read_options, key, &value);
        if (status.IsNotFound()) {
            impl_->cache_misses++;
            return StorageResult::NOT_FOUND;
        }
        if (!status.ok()) {
            return StorageResult::DATABASE_ERROR;
        }
        
        impl_->cache_hits++;
        
        // Deserialize UTXO
        std::vector<uint8_t> data(value.begin(), value.end());
        auto deserialized_utxo = UTXOEntry::deserialize(data);
        if (!deserialized_utxo) {
            return StorageResult::CORRUPTION_ERROR;
        }
        
        utxo = *deserialized_utxo;
        return StorageResult::SUCCESS;
        
    } catch (const std::exception& e) {
        return StorageResult::IO_ERROR;
    }
}

StorageResult LevelDBStorage::remove_utxo(const Hash256& tx_hash, uint32_t output_index) {
    if (!initialized_) return StorageResult::DATABASE_ERROR;
    
    std::shared_lock<std::shared_mutex> lock(mutex_);
    
    try {
        std::string key = make_utxo_key(tx_hash, output_index);
        
        leveldb::Status status = impl_->utxos_db->Delete(impl_->write_options, key);
        if (!status.ok()) {
            return StorageResult::DATABASE_ERROR;
        }
        
        return StorageResult::SUCCESS;
        
    } catch (const std::exception& e) {
        return StorageResult::IO_ERROR;
    }
}

StorageResult LevelDBStorage::has_utxo(const Hash256& tx_hash, uint32_t output_index) {
    if (!initialized_) return StorageResult::DATABASE_ERROR;
    
    std::shared_lock<std::shared_mutex> lock(mutex_);
    
    try {
        std::string key = make_utxo_key(tx_hash, output_index);
        std::string value;
        
        leveldb::Status status = impl_->utxos_db->Get(impl_->read_options, key, &value);
        if (status.IsNotFound()) {
            return StorageResult::NOT_FOUND;
        }
        if (!status.ok()) {
            return StorageResult::DATABASE_ERROR;
        }
        
        return StorageResult::SUCCESS;
        
    } catch (const std::exception& e) {
        return StorageResult::IO_ERROR;
    }
}

// Key generation utilities
std::string LevelDBStorage::make_block_key(const Hash256& block_hash) {
    return "b:" + std::string(block_hash.begin(), block_hash.end());
}

std::string LevelDBStorage::make_block_height_key(uint32_t height) {
    return "h:" + std::to_string(height);
}

std::string LevelDBStorage::make_tx_key(const Hash256& tx_hash) {
    return "t:" + std::string(tx_hash.begin(), tx_hash.end());
}

std::string LevelDBStorage::make_utxo_key(const Hash256& tx_hash, uint32_t output_index) {
    return "u:" + std::string(tx_hash.begin(), tx_hash.end()) + ":" + std::to_string(output_index);
}

std::string LevelDBStorage::make_metadata_key(const std::string& prefix, const Hash256& hash) {
    return prefix + ":" + std::string(hash.begin(), hash.end());
}

// Batch operations (simplified for now)
StorageResult LevelDBStorage::begin_batch() {
    std::lock_guard<std::shared_mutex> lock(mutex_);
    impl_->current_batch = std::make_unique<leveldb::WriteBatch>();
    return StorageResult::SUCCESS;
}

StorageResult LevelDBStorage::commit_batch() {
    std::lock_guard<std::shared_mutex> lock(mutex_);
    if (!impl_->current_batch) {
        return StorageResult::INVALID_DATA;
    }
    
    leveldb::Status status = impl_->blocks_db->Write(impl_->write_options, impl_->current_batch.get());
    impl_->current_batch.reset();
    
    return status.ok() ? StorageResult::SUCCESS : StorageResult::DATABASE_ERROR;
}

StorageResult LevelDBStorage::rollback_batch() {
    std::lock_guard<std::shared_mutex> lock(mutex_);
    impl_->current_batch.reset();
    return StorageResult::SUCCESS;
}

// Statistics and metadata operations (simplified implementations)
StorageResult LevelDBStorage::get_blockchain_height(uint32_t& height) {
    // Implementation would iterate through height index to find max
    height = 0;
    return StorageResult::SUCCESS;
}

StorageResult LevelDBStorage::get_best_block_hash(Hash256& block_hash) {
    // Implementation would retrieve from metadata
    return StorageResult::NOT_FOUND;
}

StorageResult LevelDBStorage::set_best_block_hash(const Hash256& block_hash) {
    // Implementation would store in metadata
    return StorageResult::SUCCESS;
}

StorageResult LevelDBStorage::get_utxo_count(size_t& count) {
    count = 0;
    return StorageResult::SUCCESS;
}

StorageResult LevelDBStorage::get_database_size(size_t& size_bytes) {
    size_bytes = 0;
    return StorageResult::SUCCESS;
}

// Maintenance operations
StorageResult LevelDBStorage::compact_database() {
    if (!initialized_) return StorageResult::DATABASE_ERROR;
    
    // Compact all databases
    impl_->blocks_db->CompactRange(nullptr, nullptr);
    impl_->metadata_db->CompactRange(nullptr, nullptr);
    impl_->transactions_db->CompactRange(nullptr, nullptr);
    impl_->utxos_db->CompactRange(nullptr, nullptr);
    impl_->index_db->CompactRange(nullptr, nullptr);
    
    return StorageResult::SUCCESS;
}

StorageResult LevelDBStorage::vacuum_database() {
    return compact_database(); // LevelDB doesn't have separate vacuum
}

StorageResult LevelDBStorage::repair_database() {
    // LevelDB repair would need to be done at the database level
    return StorageResult::SUCCESS;
}

LevelDBStorage::StorageStats LevelDBStorage::get_stats() const {
    StorageStats stats;
    stats.cache_hit_rate = impl_->cache_hits.load();
    stats.cache_miss_rate = impl_->cache_misses.load();
    // Other statistics would be populated here
    return stats;
}

// Placeholder implementations for other required methods
StorageResult LevelDBStorage::remove_block(const Hash256& block_hash) {
    return StorageResult::SUCCESS; // Implementation needed
}

StorageResult LevelDBStorage::store_block_metadata(const BlockMetadata& metadata) {
    return StorageResult::SUCCESS; // Implementation needed
}

StorageResult LevelDBStorage::get_block_metadata(const Hash256& block_hash, BlockMetadata& metadata) {
    return StorageResult::NOT_FOUND; // Implementation needed
}

StorageResult LevelDBStorage::get_block_metadata_by_height(uint32_t height, BlockMetadata& metadata) {
    return StorageResult::NOT_FOUND; // Implementation needed
}

StorageResult LevelDBStorage::store_transaction(const Transaction& tx) {
    return StorageResult::SUCCESS; // Implementation needed
}

StorageResult LevelDBStorage::get_transaction(const Hash256& tx_hash, Transaction& tx) {
    return StorageResult::NOT_FOUND; // Implementation needed
}

StorageResult LevelDBStorage::has_transaction(const Hash256& tx_hash) {
    return StorageResult::NOT_FOUND; // Implementation needed
}

StorageResult LevelDBStorage::remove_transaction(const Hash256& tx_hash) {
    return StorageResult::SUCCESS; // Implementation needed
}

StorageResult LevelDBStorage::store_transaction_metadata(const TransactionMetadata& metadata) {
    return StorageResult::SUCCESS; // Implementation needed
}

StorageResult LevelDBStorage::get_transaction_metadata(const Hash256& tx_hash, TransactionMetadata& metadata) {
    return StorageResult::NOT_FOUND; // Implementation needed
}

} // namespace storage
} // namespace blockchain