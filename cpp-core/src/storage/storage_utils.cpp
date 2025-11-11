#include "blockchain/storage.hpp"
#include "blockchain/storage_config.hpp"
#include "blockchain/crypto.hpp"
#include <cstring>

namespace blockchain {
namespace storage {

// Simple storage layer focusing on core functionality

// UTXOEntry implementation
std::vector<uint8_t> UTXOEntry::serialize() const {
    std::vector<uint8_t> buffer;
    
    // For now, implement a basic serialization
    // Hash
    buffer.insert(buffer.end(), tx_hash.begin(), tx_hash.end());
    
    // Output index (4 bytes)
    const auto* idx_bytes = reinterpret_cast<const uint8_t*>(&output_index);
    buffer.insert(buffer.end(), idx_bytes, idx_bytes + sizeof(uint32_t));
    
    // Value (8 bytes)
    const auto* val_bytes = reinterpret_cast<const uint8_t*>(&output.value);
    buffer.insert(buffer.end(), val_bytes, val_bytes + sizeof(uint64_t));
    
    // Script size and script
    uint32_t script_size = static_cast<uint32_t>(output.script_pubkey.size());
    const auto* size_bytes = reinterpret_cast<const uint8_t*>(&script_size);
    buffer.insert(buffer.end(), size_bytes, size_bytes + sizeof(uint32_t));
    buffer.insert(buffer.end(), output.script_pubkey.begin(), output.script_pubkey.end());
    
    // Block height (4 bytes)
    const auto* height_bytes = reinterpret_cast<const uint8_t*>(&block_height);
    buffer.insert(buffer.end(), height_bytes, height_bytes + sizeof(uint32_t));
    
    // Coinbase flag (1 byte)
    uint8_t coinbase_flag = is_coinbase ? 1 : 0;
    buffer.push_back(coinbase_flag);
    
    return buffer;
}

std::optional<UTXOEntry> UTXOEntry::deserialize(const std::vector<uint8_t>& data) {
    if (data.size() < 32 + 4 + 8 + 4 + 4 + 1) { // Minimum size
        return std::nullopt;
    }
    
    UTXOEntry entry;
    const uint8_t* ptr = data.data();
    
    // Hash (32 bytes)
    std::copy(ptr, ptr + 32, entry.tx_hash.begin());
    ptr += 32;
    
    // Output index (4 bytes)
    std::memcpy(&entry.output_index, ptr, sizeof(uint32_t));
    ptr += sizeof(uint32_t);
    
    // Value (8 bytes)  
    std::memcpy(&entry.output.value, ptr, sizeof(uint64_t));
    ptr += sizeof(uint64_t);
    
    // Script size and script
    uint32_t script_size;
    std::memcpy(&script_size, ptr, sizeof(uint32_t));
    ptr += sizeof(uint32_t);
    
    if (ptr + script_size > data.data() + data.size()) {
        return std::nullopt;
    }
    
    entry.output.script_pubkey.assign(ptr, ptr + script_size);
    ptr += script_size;
    
    // Block height (4 bytes)
    std::memcpy(&entry.block_height, ptr, sizeof(uint32_t));
    ptr += sizeof(uint32_t);
    
    // Coinbase flag (1 byte)
    uint8_t coinbase_flag;
    std::memcpy(&coinbase_flag, ptr, sizeof(uint8_t));
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
    
    // Block hash (32 bytes)
    buffer.insert(buffer.end(), block_hash.begin(), block_hash.end());
    
    // Previous block hash (32 bytes)
    buffer.insert(buffer.end(), prev_block_hash.begin(), prev_block_hash.end());
    
    // Other fields (4 bytes each)
    const auto* height_bytes = reinterpret_cast<const uint8_t*>(&height);
    buffer.insert(buffer.end(), height_bytes, height_bytes + sizeof(uint32_t));
    
    const auto* timestamp_bytes = reinterpret_cast<const uint8_t*>(&timestamp);
    buffer.insert(buffer.end(), timestamp_bytes, timestamp_bytes + sizeof(uint32_t));
    
    const auto* tx_count_bytes = reinterpret_cast<const uint8_t*>(&tx_count);
    buffer.insert(buffer.end(), tx_count_bytes, tx_count_bytes + sizeof(uint32_t));
    
    const auto* work_bytes = reinterpret_cast<const uint8_t*>(&total_work);
    buffer.insert(buffer.end(), work_bytes, work_bytes + sizeof(uint64_t));
    
    const auto* pos_bytes = reinterpret_cast<const uint8_t*>(&file_position);
    buffer.insert(buffer.end(), pos_bytes, pos_bytes + sizeof(size_t));
    
    const auto* size_bytes = reinterpret_cast<const uint8_t*>(&block_size);
    buffer.insert(buffer.end(), size_bytes, size_bytes + sizeof(size_t));
    
    return buffer;
}

std::optional<BlockMetadata> BlockMetadata::deserialize(const std::vector<uint8_t>& data) {
    const size_t expected_size = 32 + 32 + 4 + 4 + 4 + 8 + sizeof(size_t) + sizeof(size_t);
    if (data.size() != expected_size) {
        return std::nullopt;
    }
    
    BlockMetadata metadata;
    const uint8_t* ptr = data.data();
    
    // Block hash (32 bytes)
    std::copy(ptr, ptr + 32, metadata.block_hash.begin());
    ptr += 32;
    
    // Previous block hash (32 bytes)
    std::copy(ptr, ptr + 32, metadata.prev_block_hash.begin());
    ptr += 32;
    
    // Other fields
    std::memcpy(&metadata.height, ptr, sizeof(uint32_t));
    ptr += sizeof(uint32_t);
    
    std::memcpy(&metadata.timestamp, ptr, sizeof(uint32_t));
    ptr += sizeof(uint32_t);
    
    std::memcpy(&metadata.tx_count, ptr, sizeof(uint32_t));
    ptr += sizeof(uint32_t);
    
    std::memcpy(&metadata.total_work, ptr, sizeof(uint64_t));
    ptr += sizeof(uint64_t);
    
    std::memcpy(&metadata.file_position, ptr, sizeof(size_t));
    ptr += sizeof(size_t);
    
    std::memcpy(&metadata.block_size, ptr, sizeof(size_t));
    
    return metadata;
}

// TransactionMetadata implementation
std::vector<uint8_t> TransactionMetadata::serialize() const {
    std::vector<uint8_t> buffer;
    buffer.reserve(96);
    
    // Transaction hash (32 bytes)
    buffer.insert(buffer.end(), tx_hash.begin(), tx_hash.end());
    
    // Block hash (32 bytes)
    buffer.insert(buffer.end(), block_hash.begin(), block_hash.end());
    
    // Other fields
    const auto* height_bytes = reinterpret_cast<const uint8_t*>(&block_height);
    buffer.insert(buffer.end(), height_bytes, height_bytes + sizeof(uint32_t));
    
    const auto* idx_bytes = reinterpret_cast<const uint8_t*>(&tx_index);
    buffer.insert(buffer.end(), idx_bytes, idx_bytes + sizeof(uint32_t));
    
    const auto* pos_bytes = reinterpret_cast<const uint8_t*>(&file_position);
    buffer.insert(buffer.end(), pos_bytes, pos_bytes + sizeof(size_t));
    
    const auto* size_bytes = reinterpret_cast<const uint8_t*>(&tx_size);
    buffer.insert(buffer.end(), size_bytes, size_bytes + sizeof(size_t));
    
    return buffer;
}

std::optional<TransactionMetadata> TransactionMetadata::deserialize(const std::vector<uint8_t>& data) {
    const size_t expected_size = 32 + 32 + 4 + 4 + sizeof(size_t) + sizeof(size_t);
    if (data.size() != expected_size) {
        return std::nullopt;
    }
    
    TransactionMetadata metadata;
    const uint8_t* ptr = data.data();
    
    // Transaction hash (32 bytes)
    std::copy(ptr, ptr + 32, metadata.tx_hash.begin());
    ptr += 32;
    
    // Block hash (32 bytes)  
    std::copy(ptr, ptr + 32, metadata.block_hash.begin());
    ptr += 32;
    
    // Other fields
    std::memcpy(&metadata.block_height, ptr, sizeof(uint32_t));
    ptr += sizeof(uint32_t);
    
    std::memcpy(&metadata.tx_index, ptr, sizeof(uint32_t));
    ptr += sizeof(uint32_t);
    
    std::memcpy(&metadata.file_position, ptr, sizeof(size_t));
    ptr += sizeof(size_t);
    
    std::memcpy(&metadata.tx_size, ptr, sizeof(size_t));
    
    return metadata;
}

} // namespace storage
} // namespace blockchain