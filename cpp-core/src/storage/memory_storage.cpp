#include "blockchain/storage.hpp"
#include <sstream>

namespace blockchain {
namespace storage {

// MemoryStorage implementation
StorageResult MemoryStorage::store_block(const Block& block) {
    std::lock_guard<std::shared_mutex> lock(mutex_);
    
    Hash256 block_hash = block.calculate_hash();
    
    if (blocks_.find(block_hash) != blocks_.end()) {
        return StorageResult::ALREADY_EXISTS;
    }
    
    blocks_[block_hash] = block;
    height_to_hash_[block.header.height] = block_hash;
    
    return StorageResult::SUCCESS;
}

StorageResult MemoryStorage::get_block(const Hash256& block_hash, Block& block) {
    std::shared_lock<std::shared_mutex> lock(mutex_);
    
    auto it = blocks_.find(block_hash);
    if (it == blocks_.end()) {
        return StorageResult::NOT_FOUND;
    }
    
    block = it->second;
    return StorageResult::SUCCESS;
}

StorageResult MemoryStorage::get_block_by_height(uint32_t height, Block& block) {
    std::shared_lock<std::shared_mutex> lock(mutex_);
    
    auto it = height_to_hash_.find(height);
    if (it == height_to_hash_.end()) {
        return StorageResult::NOT_FOUND;
    }
    
    return get_block(it->second, block);
}

StorageResult MemoryStorage::has_block(const Hash256& block_hash) {
    std::shared_lock<std::shared_mutex> lock(mutex_);
    
    return blocks_.find(block_hash) != blocks_.end() ? 
           StorageResult::SUCCESS : StorageResult::NOT_FOUND;
}

StorageResult MemoryStorage::remove_block(const Hash256& block_hash) {
    std::lock_guard<std::shared_mutex> lock(mutex_);
    
    auto it = blocks_.find(block_hash);
    if (it == blocks_.end()) {
        return StorageResult::NOT_FOUND;
    }
    
    // Remove from height mapping
    uint32_t height = it->second.header.height;
    height_to_hash_.erase(height);
    
    blocks_.erase(it);
    return StorageResult::SUCCESS;
}

StorageResult MemoryStorage::store_block_metadata(const BlockMetadata& metadata) {
    std::lock_guard<std::shared_mutex> lock(mutex_);
    block_metadata_[metadata.block_hash] = metadata;
    return StorageResult::SUCCESS;
}

StorageResult MemoryStorage::get_block_metadata(const Hash256& block_hash, BlockMetadata& metadata) {
    std::shared_lock<std::shared_mutex> lock(mutex_);
    
    auto it = block_metadata_.find(block_hash);
    if (it == block_metadata_.end()) {
        return StorageResult::NOT_FOUND;
    }
    
    metadata = it->second;
    return StorageResult::SUCCESS;
}

StorageResult MemoryStorage::get_block_metadata_by_height(uint32_t height, BlockMetadata& metadata) {
    std::shared_lock<std::shared_mutex> lock(mutex_);
    
    auto it = height_to_hash_.find(height);
    if (it == height_to_hash_.end()) {
        return StorageResult::NOT_FOUND;
    }
    
    return get_block_metadata(it->second, metadata);
}

StorageResult MemoryStorage::store_transaction(const Transaction& tx) {
    std::lock_guard<std::shared_mutex> lock(mutex_);
    
    Hash256 tx_hash = tx.calculate_hash();
    
    if (transactions_.find(tx_hash) != transactions_.end()) {
        return StorageResult::ALREADY_EXISTS;
    }
    
    transactions_[tx_hash] = tx;
    return StorageResult::SUCCESS;
}

StorageResult MemoryStorage::get_transaction(const Hash256& tx_hash, Transaction& tx) {
    std::shared_lock<std::shared_mutex> lock(mutex_);
    
    auto it = transactions_.find(tx_hash);
    if (it == transactions_.end()) {
        return StorageResult::NOT_FOUND;
    }
    
    tx = it->second;
    return StorageResult::SUCCESS;
}

StorageResult MemoryStorage::has_transaction(const Hash256& tx_hash) {
    std::shared_lock<std::shared_mutex> lock(mutex_);
    
    return transactions_.find(tx_hash) != transactions_.end() ? 
           StorageResult::SUCCESS : StorageResult::NOT_FOUND;
}

StorageResult MemoryStorage::remove_transaction(const Hash256& tx_hash) {
    std::lock_guard<std::shared_mutex> lock(mutex_);
    
    auto it = transactions_.find(tx_hash);
    if (it == transactions_.end()) {
        return StorageResult::NOT_FOUND;
    }
    
    transactions_.erase(it);
    return StorageResult::SUCCESS;
}

StorageResult MemoryStorage::store_transaction_metadata(const TransactionMetadata& metadata) {
    std::lock_guard<std::shared_mutex> lock(mutex_);
    tx_metadata_[metadata.tx_hash] = metadata;
    return StorageResult::SUCCESS;
}

StorageResult MemoryStorage::get_transaction_metadata(const Hash256& tx_hash, TransactionMetadata& metadata) {
    std::shared_lock<std::shared_mutex> lock(mutex_);
    
    auto it = tx_metadata_.find(tx_hash);
    if (it == tx_metadata_.end()) {
        return StorageResult::NOT_FOUND;
    }
    
    metadata = it->second;
    return StorageResult::SUCCESS;
}

StorageResult MemoryStorage::add_utxo(const Hash256& tx_hash, uint32_t output_index, const UTXOEntry& utxo) {
    std::lock_guard<std::shared_mutex> lock(mutex_);
    
    std::string key = make_utxo_key(tx_hash, output_index);
    
    if (utxos_.find(key) != utxos_.end()) {
        return StorageResult::ALREADY_EXISTS;
    }
    
    utxos_[key] = utxo;
    return StorageResult::SUCCESS;
}

StorageResult MemoryStorage::get_utxo(const Hash256& tx_hash, uint32_t output_index, UTXOEntry& utxo) {
    std::shared_lock<std::shared_mutex> lock(mutex_);
    
    std::string key = make_utxo_key(tx_hash, output_index);
    auto it = utxos_.find(key);
    if (it == utxos_.end()) {
        return StorageResult::NOT_FOUND;
    }
    
    utxo = it->second;
    return StorageResult::SUCCESS;
}

StorageResult MemoryStorage::remove_utxo(const Hash256& tx_hash, uint32_t output_index) {
    std::lock_guard<std::shared_mutex> lock(mutex_);
    
    std::string key = make_utxo_key(tx_hash, output_index);
    auto it = utxos_.find(key);
    if (it == utxos_.end()) {
        return StorageResult::NOT_FOUND;
    }
    
    utxos_.erase(it);
    return StorageResult::SUCCESS;
}

StorageResult MemoryStorage::has_utxo(const Hash256& tx_hash, uint32_t output_index) {
    std::shared_lock<std::shared_mutex> lock(mutex_);
    
    std::string key = make_utxo_key(tx_hash, output_index);
    return utxos_.find(key) != utxos_.end() ? 
           StorageResult::SUCCESS : StorageResult::NOT_FOUND;
}

StorageResult MemoryStorage::begin_batch() {
    // In-memory storage doesn't need batching, but we simulate it
    return StorageResult::SUCCESS;
}

StorageResult MemoryStorage::commit_batch() {
    // No-op for in-memory storage
    return StorageResult::SUCCESS;
}

StorageResult MemoryStorage::rollback_batch() {
    // No-op for in-memory storage
    return StorageResult::SUCCESS;
}

StorageResult MemoryStorage::get_blockchain_height(uint32_t& height) {
    std::shared_lock<std::shared_mutex> lock(mutex_);
    
    if (height_to_hash_.empty()) {
        height = 0;
        return StorageResult::NOT_FOUND;
    }
    
    // Find maximum height since unordered_map doesn't have rbegin()
    uint32_t max_height = 0;
    for (const auto& [h, hash] : height_to_hash_) {
        max_height = std::max(max_height, h);
    }
    
    height = max_height;
    return StorageResult::SUCCESS;
}

StorageResult MemoryStorage::get_best_block_hash(Hash256& block_hash) {
    std::shared_lock<std::shared_mutex> lock(mutex_);
    
    if (best_block_hash_ == Hash256{}) {
        return StorageResult::NOT_FOUND;
    }
    
    block_hash = best_block_hash_;
    return StorageResult::SUCCESS;
}

StorageResult MemoryStorage::set_best_block_hash(const Hash256& block_hash) {
    std::lock_guard<std::shared_mutex> lock(mutex_);
    best_block_hash_ = block_hash;
    return StorageResult::SUCCESS;
}

StorageResult MemoryStorage::get_utxo_count(size_t& count) {
    std::shared_lock<std::shared_mutex> lock(mutex_);
    count = utxos_.size();
    return StorageResult::SUCCESS;
}

StorageResult MemoryStorage::get_database_size(size_t& size_bytes) {
    std::shared_lock<std::shared_mutex> lock(mutex_);
    
    // Estimate memory usage
    size_bytes = 0;
    
    // Blocks
    for (const auto& [hash, block] : blocks_) {
        size_bytes += block.get_serialized_size();
    }
    
    // Transactions
    for (const auto& [hash, tx] : transactions_) {
        size_bytes += tx.get_serialized_size();
    }
    
    // UTXOs
    for (const auto& [key, utxo] : utxos_) {
        size_bytes += utxo.get_serialized_size();
    }
    
    return StorageResult::SUCCESS;
}

StorageResult MemoryStorage::compact_database() {
    // No-op for in-memory storage
    return StorageResult::SUCCESS;
}

StorageResult MemoryStorage::vacuum_database() {
    // No-op for in-memory storage
    return StorageResult::SUCCESS;
}

StorageResult MemoryStorage::repair_database() {
    // No-op for in-memory storage
    return StorageResult::SUCCESS;
}

void MemoryStorage::clear() {
    std::lock_guard<std::shared_mutex> lock(mutex_);
    blocks_.clear();
    height_to_hash_.clear();
    block_metadata_.clear();
    transactions_.clear();
    tx_metadata_.clear();
    utxos_.clear();
    best_block_hash_ = Hash256{};
}

std::string MemoryStorage::make_utxo_key(const Hash256& tx_hash, uint32_t output_index) {
    std::ostringstream oss;
    for (auto byte : tx_hash) {
        oss << std::hex << static_cast<int>(byte);
    }
    oss << ":" << output_index;
    return oss.str();
}

// StorageFactory implementation
std::unique_ptr<IBlockchainStorage> StorageFactory::create(StorageType type, const StorageConfig& config) {
    switch (type) {
        case StorageType::MEMORY:
            return std::make_unique<MemoryStorage>();
        case StorageType::LEVELDB:
            return std::make_unique<LevelDBStorage>(config);
        default:
            return nullptr;
    }
}

std::unique_ptr<IBlockchainStorage> StorageFactory::create_default() {
    StorageConfig config;
    return std::make_unique<LevelDBStorage>(config);
}

std::unique_ptr<IBlockchainStorage> StorageFactory::create_test() {
    return std::make_unique<MemoryStorage>();
}

// UTXOManager implementation
UTXOManager::UTXOManager(std::shared_ptr<IBlockchainStorage> storage)
    : storage_(storage), cache_size_limit_(10000) {
}

bool UTXOManager::add_utxo(const Transaction& tx, uint32_t output_index, uint32_t block_height) {
    if (output_index >= tx.outputs.size()) {
        return false;
    }
    
    Hash256 tx_hash = tx.calculate_hash();
    
    UTXOEntry utxo;
    utxo.tx_hash = tx_hash;
    utxo.output_index = output_index;
    utxo.output = tx.outputs[output_index];
    utxo.block_height = block_height;
    utxo.is_coinbase = (tx.inputs.empty() || 
                       (tx.inputs.size() == 1 && tx.inputs[0].prev_tx_hash == Hash256{}));
    
    // Add to storage
    StorageResult result = storage_->add_utxo(tx_hash, output_index, utxo);
    if (result != StorageResult::SUCCESS) {
        return false;
    }
    
    // Add to cache
    std::lock_guard<std::shared_mutex> lock(cache_mutex_);
    std::string cache_key = make_cache_key(tx_hash, output_index);
    utxo_cache_[cache_key] = utxo;
    
    // Evict if cache is full
    if (utxo_cache_.size() > cache_size_limit_) {
        evict_cache_entries();
    }
    
    return true;
}

bool UTXOManager::remove_utxo(const Hash256& tx_hash, uint32_t output_index) {
    // Remove from storage
    StorageResult result = storage_->remove_utxo(tx_hash, output_index);
    if (result != StorageResult::SUCCESS && result != StorageResult::NOT_FOUND) {
        return false;
    }
    
    // Remove from cache
    std::lock_guard<std::shared_mutex> lock(cache_mutex_);
    std::string cache_key = make_cache_key(tx_hash, output_index);
    utxo_cache_.erase(cache_key);
    
    return true;
}

std::optional<UTXOEntry> UTXOManager::get_utxo(const Hash256& tx_hash, uint32_t output_index) {
    std::string cache_key = make_cache_key(tx_hash, output_index);
    
    // Check cache first
    {
        std::shared_lock<std::shared_mutex> lock(cache_mutex_);
        auto it = utxo_cache_.find(cache_key);
        if (it != utxo_cache_.end()) {
            return it->second;
        }
    }
    
    // Check storage
    UTXOEntry utxo;
    StorageResult result = storage_->get_utxo(tx_hash, output_index, utxo);
    if (result != StorageResult::SUCCESS) {
        return std::nullopt;
    }
    
    // Add to cache
    {
        std::lock_guard<std::shared_mutex> lock(cache_mutex_);
        utxo_cache_[cache_key] = utxo;
        
        if (utxo_cache_.size() > cache_size_limit_) {
            evict_cache_entries();
        }
    }
    
    return utxo;
}

bool UTXOManager::has_utxo(const Hash256& tx_hash, uint32_t output_index) {
    return get_utxo(tx_hash, output_index).has_value();
}

size_t UTXOManager::get_utxo_count() const {
    size_t count;
    StorageResult result = storage_->get_utxo_count(count);
    return (result == StorageResult::SUCCESS) ? count : 0;
}

std::vector<UTXOEntry> UTXOManager::get_utxos_for_address(const std::string& address) {
    // This would require address indexing to be efficient
    // For now, return empty vector
    return {};
}

uint64_t UTXOManager::get_total_value() const {
    // This would require iterating through all UTXOs
    // Expensive operation, should be cached
    return 0;
}

bool UTXOManager::validate_utxo_set() {
    // Validate consistency between cache and storage
    // This is a maintenance operation
    return true;
}

bool UTXOManager::flush() {
    // For LevelDB storage, this might trigger compaction
    return storage_->compact_database() == StorageResult::SUCCESS;
}

std::string UTXOManager::make_cache_key(const Hash256& tx_hash, uint32_t output_index) {
    std::ostringstream oss;
    for (auto byte : tx_hash) {
        oss << std::hex << static_cast<int>(byte);
    }
    oss << ":" << output_index;
    return oss.str();
}

void UTXOManager::evict_cache_entries() {
    // Simple LRU eviction - remove 10% of entries
    // In a real implementation, this would use proper LRU tracking
    size_t target_size = cache_size_limit_ * 0.9;
    
    auto it = utxo_cache_.begin();
    while (utxo_cache_.size() > target_size && it != utxo_cache_.end()) {
        it = utxo_cache_.erase(it);
    }
}

// BlockchainStorageManager implementation
BlockchainStorageManager::BlockchainStorageManager(const StorageConfig& config) 
    : config_(config) {
}

BlockchainStorageManager::~BlockchainStorageManager() {
    if (initialized_) {
        shutdown();
    }
}

bool BlockchainStorageManager::initialize() {
    if (initialized_) return true;
    
    // Create storage
    storage_ = std::make_shared<LevelDBStorage>(config_);
    auto leveldb_storage = static_cast<LevelDBStorage*>(storage_.get());
    
    StorageResult result = leveldb_storage->initialize();
    if (result != StorageResult::SUCCESS) {
        return false;
    }
    
    // Create UTXO manager
    utxo_manager_ = std::make_shared<UTXOManager>(storage_);
    
    initialized_ = true;
    return true;
}

void BlockchainStorageManager::shutdown() {
    if (!initialized_) return;
    
    if (utxo_manager_) {
        utxo_manager_->flush();
    }
    
    if (storage_) {
        auto leveldb_storage = static_cast<LevelDBStorage*>(storage_.get());
        leveldb_storage->shutdown();
    }
    
    initialized_ = false;
}

bool BlockchainStorageManager::store_block_atomic(const Block& block, uint32_t height) {
    if (!initialized_) return false;
    
    // Begin transaction
    if (storage_->begin_batch() != StorageResult::SUCCESS) {
        return false;
    }
    
    try {
        // Store block
        if (storage_->store_block(block) != StorageResult::SUCCESS) {
            storage_->rollback_batch();
            return false;
        }
        
        // Process transactions and update UTXO set
        Hash256 block_hash = block.calculate_hash();
        
        for (size_t tx_idx = 0; tx_idx < block.transactions.size(); ++tx_idx) {
            const Transaction& tx = block.transactions[tx_idx];
            Hash256 tx_hash = tx.calculate_hash();
            
            // Store transaction
            if (storage_->store_transaction(tx) != StorageResult::SUCCESS) {
                storage_->rollback_batch();
                return false;
            }
            
            // Remove spent UTXOs (except for coinbase)
            if (tx_idx > 0) { // Skip coinbase transaction
                for (const auto& input : tx.inputs) {
                    if (!utxo_manager_->remove_utxo(input.prev_tx_hash, input.prev_output_index)) {
                        storage_->rollback_batch();
                        return false;
                    }
                }
            }
            
            // Add new UTXOs
            for (size_t output_idx = 0; output_idx < tx.outputs.size(); ++output_idx) {
                if (!utxo_manager_->add_utxo(tx, static_cast<uint32_t>(output_idx), height)) {
                    storage_->rollback_batch();
                    return false;
                }
            }
        }
        
        // Update best block
        storage_->set_best_block_hash(block_hash);
        
        // Commit transaction
        if (storage_->commit_batch() != StorageResult::SUCCESS) {
            return false;
        }
        
        return true;
        
    } catch (const std::exception& e) {
        storage_->rollback_batch();
        return false;
    }
}

bool BlockchainStorageManager::remove_block_atomic(const Hash256& block_hash) {
    // Implementation would reverse all operations from store_block_atomic
    return true;
}

BlockchainStorageManager::BlockchainStats BlockchainStorageManager::get_blockchain_stats() {
    BlockchainStats stats;
    
    if (!initialized_) {
        return stats;
    }
    
    storage_->get_blockchain_height(stats.height);
    storage_->get_best_block_hash(stats.best_block_hash);
    storage_->get_utxo_count(stats.total_utxos);
    storage_->get_database_size(stats.database_size_bytes);
    
    stats.total_value = utxo_manager_->get_total_value();
    
    return stats;
}

} // namespace storage
} // namespace blockchain