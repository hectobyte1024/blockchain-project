#include "blockchain/block.hpp"
#include "blockchain/crypto.hpp"
#include <algorithm>
#include <sstream>
#include <iomanip>
#include <cstring>
#include <cmath>
#include <mutex>
#include <shared_mutex>

namespace blockchain {
namespace block {

using namespace crypto;

// Helper functions for serialization
namespace {
    void write_uint32_le(std::vector<uint8_t>& data, uint32_t value) {
        for (int i = 0; i < 4; ++i) {
            data.push_back((value >> (i * 8)) & 0xFF);
        }
    }
    
    std::optional<uint32_t> read_uint32_le(const std::vector<uint8_t>& data, size_t& offset) {
        if (offset + 4 > data.size()) return std::nullopt;
        uint32_t value = 0;
        for (int i = 0; i < 4; ++i) {
            value |= (static_cast<uint32_t>(data[offset + i]) << (i * 8));
        }
        offset += 4;
        return value;
    }
    
    void write_varint(std::vector<uint8_t>& data, uint64_t value) {
        if (value < 0xFD) {
            data.push_back(static_cast<uint8_t>(value));
        } else if (value <= 0xFFFF) {
            data.push_back(0xFD);
            data.push_back(value & 0xFF);
            data.push_back((value >> 8) & 0xFF);
        } else if (value <= 0xFFFFFFFF) {
            data.push_back(0xFE);
            for (int i = 0; i < 4; ++i) {
                data.push_back((value >> (i * 8)) & 0xFF);
            }
        } else {
            data.push_back(0xFF);
            for (int i = 0; i < 8; ++i) {
                data.push_back((value >> (i * 8)) & 0xFF);
            }
        }
    }
    
    std::optional<uint64_t> read_varint(const std::vector<uint8_t>& data, size_t& offset) {
        if (offset >= data.size()) return std::nullopt;
        
        uint8_t first = data[offset++];
        if (first < 0xFD) {
            return first;
        } else if (first == 0xFD) {
            if (offset + 2 > data.size()) return std::nullopt;
            uint64_t value = data[offset] | (data[offset + 1] << 8);
            offset += 2;
            return value;
        } else if (first == 0xFE) {
            if (offset + 4 > data.size()) return std::nullopt;
            uint64_t value = 0;
            for (int i = 0; i < 4; ++i) {
                value |= (static_cast<uint64_t>(data[offset + i]) << (i * 8));
            }
            offset += 4;
            return value;
        } else { // 0xFF
            if (offset + 8 > data.size()) return std::nullopt;
            uint64_t value = 0;
            for (int i = 0; i < 8; ++i) {
                value |= (static_cast<uint64_t>(data[offset + i]) << (i * 8));
            }
            offset += 8;
            return value;
        }
    }
}

// BlockHeader implementation
BlockHeader::BlockHeader(uint32_t version, const Hash256& prev_hash, const Hash256& merkle_root,
                        uint32_t timestamp, uint32_t difficulty_target, uint32_t nonce)
    : version(version), prev_block_hash(prev_hash), merkle_root(merkle_root), 
      timestamp(timestamp), difficulty_target(difficulty_target), nonce(nonce) {}

std::vector<uint8_t> BlockHeader::serialize() const {
    std::vector<uint8_t> data;
    data.reserve(SERIALIZED_SIZE);
    
    // Version (4 bytes)
    write_uint32_le(data, version);
    
    // Previous block hash (32 bytes)
    data.insert(data.end(), prev_block_hash.begin(), prev_block_hash.end());
    
    // Merkle root (32 bytes)
    data.insert(data.end(), merkle_root.begin(), merkle_root.end());
    
    // Timestamp (4 bytes)
    write_uint32_le(data, timestamp);
    
    // Difficulty target (4 bytes)
    write_uint32_le(data, difficulty_target);
    
    // Nonce (4 bytes)
    write_uint32_le(data, nonce);
    
    return data;
}

std::optional<BlockHeader> BlockHeader::deserialize(const std::vector<uint8_t>& data) {
    if (data.size() < SERIALIZED_SIZE) return std::nullopt;
    
    size_t offset = 0;
    BlockHeader header;
    
    // Version
    auto ver = read_uint32_le(data, offset);
    if (!ver) return std::nullopt;
    header.version = *ver;
    
    // Previous block hash
    if (offset + 32 > data.size()) return std::nullopt;
    std::copy(data.begin() + offset, data.begin() + offset + 32, header.prev_block_hash.begin());
    offset += 32;
    
    // Merkle root
    if (offset + 32 > data.size()) return std::nullopt;
    std::copy(data.begin() + offset, data.begin() + offset + 32, header.merkle_root.begin());
    offset += 32;
    
    // Timestamp
    auto ts = read_uint32_le(data, offset);
    if (!ts) return std::nullopt;
    header.timestamp = *ts;
    
    // Difficulty target
    auto diff = read_uint32_le(data, offset);
    if (!diff) return std::nullopt;
    header.difficulty_target = *diff;
    
    // Nonce
    auto n = read_uint32_le(data, offset);
    if (!n) return std::nullopt;
    header.nonce = *n;
    
    return header;
}

Hash256 BlockHeader::calculate_hash() const {
    auto data = serialize();
    return SHA256::double_hash(data);
}

std::string BlockHeader::get_hash_string() const {
    auto hash = calculate_hash();
    // Reverse for display (Bitcoin convention)
    std::reverse(hash.begin(), hash.end());
    return crypto::utils::to_hex(hash);
}

bool BlockHeader::meets_difficulty_target() const {
    auto hash = calculate_hash();
    auto target = get_target();
    
    // Compare hash with target (both as big-endian)
    return std::memcmp(hash.data(), target.data(), 32) <= 0;
}

double BlockHeader::get_difficulty() const {
    return nbits_to_difficulty(difficulty_target);
}

std::array<uint8_t, 32> BlockHeader::get_target() const {
    std::array<uint8_t, 32> target = {};
    
    // Extract exponent and mantissa from nBits format
    uint32_t exponent = difficulty_target >> 24;
    uint32_t mantissa = difficulty_target & 0x007FFFFF;
    
    // Validate exponent
    if (exponent <= 3) return target; // Invalid
    
    // Calculate target value
    size_t start_byte = 32 - exponent;
    if (start_byte >= 32) return target; // Overflow
    
    // Set mantissa bytes (big-endian)
    if (start_byte + 2 < 32) target[start_byte + 2] = mantissa & 0xFF;
    if (start_byte + 1 < 32) target[start_byte + 1] = (mantissa >> 8) & 0xFF;
    if (start_byte < 32) target[start_byte] = (mantissa >> 16) & 0xFF;
    
    return target;
}

uint32_t BlockHeader::difficulty_to_nbits(double difficulty) {
    // Convert difficulty to target, then to nBits
    // This is a simplified implementation
    if (difficulty <= 0) return 0x207FFFFF; // Maximum difficulty
    
    // Calculate target = max_target / difficulty
    // For simplicity, use a reference difficulty
    uint32_t exponent = 0x1D; // Start with common exponent
    uint32_t mantissa = static_cast<uint32_t>(0x00FFFF / difficulty);
    
    // Normalize mantissa
    while (mantissa > 0x007FFFFF && exponent > 0) {
        mantissa >>= 8;
        exponent--;
    }
    
    return (exponent << 24) | (mantissa & 0x007FFFFF);
}

double BlockHeader::nbits_to_difficulty(uint32_t nbits) {
    // Extract exponent and mantissa
    uint32_t exponent = nbits >> 24;
    uint32_t mantissa = nbits & 0x007FFFFF;
    
    if (exponent <= 3 || mantissa == 0) return 0.0;
    
    // Calculate difficulty (simplified)
    double target_value = static_cast<double>(mantissa) * std::pow(256.0, exponent - 3);
    double max_target = static_cast<double>(0x00FFFF) * std::pow(256.0, 0x1D - 3);
    
    return max_target / target_value;
}

bool BlockHeader::is_valid() const {
    // Basic validation
    if (version == 0) return false;
    if (difficulty_target == 0) return false;
    if (timestamp == 0) return false;
    
    return true;
}

// Block implementation
Block::Block(const BlockHeader& header, std::vector<Transaction> transactions)
    : header(header), transactions(std::move(transactions)) {}

std::vector<uint8_t> Block::serialize() const {
    std::vector<uint8_t> data;
    
    // Serialize header
    auto header_data = header.serialize();
    data.insert(data.end(), header_data.begin(), header_data.end());
    
    // Transaction count
    write_varint(data, transactions.size());
    
    // Serialize transactions
    for (const auto& tx : transactions) {
        auto tx_data = tx.serialize();
        data.insert(data.end(), tx_data.begin(), tx_data.end());
    }
    
    return data;
}

std::optional<Block> Block::deserialize(const std::vector<uint8_t>& data) {
    size_t offset = 0;
    
    // Deserialize header
    if (offset + BlockHeader::SERIALIZED_SIZE > data.size()) return std::nullopt;
    
    std::vector<uint8_t> header_data(data.begin() + offset, data.begin() + offset + BlockHeader::SERIALIZED_SIZE);
    auto header = BlockHeader::deserialize(header_data);
    if (!header) return std::nullopt;
    offset += BlockHeader::SERIALIZED_SIZE;
    
    // Transaction count
    auto tx_count = read_varint(data, offset);
    if (!tx_count) return std::nullopt;
    
    // Deserialize transactions
    std::vector<Transaction> transactions;
    transactions.reserve(*tx_count);
    
    for (uint64_t i = 0; i < *tx_count; ++i) {
        std::vector<uint8_t> remaining_data(data.begin() + offset, data.end());
        auto tx = Transaction::deserialize(remaining_data);
        if (!tx) return std::nullopt;
        
        transactions.push_back(*tx);
        offset += tx->get_serialized_size();
    }
    
    Block block(*header, std::move(transactions));
    return block;
}

Hash256 Block::get_hash() const {
    if (!cached_hash) {
        cached_hash = header.calculate_hash();
    }
    return *cached_hash;
}

std::string Block::get_hash_string() const {
    return header.get_hash_string();
}

Hash256 Block::calculate_merkle_root() const {
    if (!cached_merkle) {
        if (transactions.empty()) {
            cached_merkle = Hash256{}; // All zeros
        } else {
            // Collect transaction hashes
            std::vector<Hash256> tx_hashes;
            tx_hashes.reserve(transactions.size());
            
            for (const auto& tx : transactions) {
                tx_hashes.push_back(tx.get_hash());
            }
            
            // Build Merkle tree
            MerkleTree tree(tx_hashes);
            cached_merkle = tree.get_root();
        }
    }
    return *cached_merkle;
}

void Block::update_merkle_root() {
    cached_merkle.reset();
    header.merkle_root = calculate_merkle_root();
}

size_t Block::get_serialized_size() const {
    return serialize().size();
}

size_t Block::get_weight() const {
    size_t base_size = BlockHeader::SERIALIZED_SIZE;
    size_t total_size = base_size;
    
    // Add varint for transaction count
    if (transactions.size() < 0xFD) base_size += 1;
    else if (transactions.size() <= 0xFFFF) base_size += 3;
    else if (transactions.size() <= 0xFFFFFFFF) base_size += 5;
    else base_size += 9;
    
    total_size = base_size;
    
    // Add transaction sizes
    for (const auto& tx : transactions) {
        base_size += tx.get_base_size();
        total_size += tx.get_serialized_size();
    }
    
    return base_size * 3 + total_size;
}

size_t Block::get_transaction_count() const {
    return transactions.size();
}

uint64_t Block::get_total_fees(const UTXOSet& utxo_set) const {
    uint64_t total_fees = 0;
    
    for (const auto& tx : transactions) {
        if (!tx.is_coinbase()) {
            total_fees += tx.calculate_fee(utxo_set);
        }
    }
    
    return total_fees;
}

uint64_t Block::get_block_reward() const {
    if (transactions.empty() || !transactions[0].is_coinbase()) {
        return 0;
    }
    
    return transactions[0].get_total_output_value();
}

bool Block::is_valid() const {
    return validate_structure() && validate_merkle_root() && validate_proof_of_work();
}

bool Block::validate_structure() const {
    // Must have at least one transaction (coinbase)
    if (transactions.empty()) return false;
    
    // First transaction must be coinbase
    if (!transactions[0].is_coinbase()) return false;
    
    // No other transactions can be coinbase
    for (size_t i = 1; i < transactions.size(); ++i) {
        if (transactions[i].is_coinbase()) return false;
    }
    
    // Validate all transactions
    for (const auto& tx : transactions) {
        if (!tx.is_valid()) return false;
    }
    
    return true;
}

bool Block::validate_transactions(const UTXOSet& utxo_set) const {
    for (const auto& tx : transactions) {
        if (!tx.is_coinbase()) {
            if (!tx.verify_all_signatures(utxo_set)) return false;
        }
    }
    return true;
}

bool Block::validate_merkle_root() const {
    return header.merkle_root == calculate_merkle_root();
}

bool Block::validate_proof_of_work() const {
    return header.meets_difficulty_target();
}

bool Block::apply_to_utxo_set(UTXOSet& utxo_set, uint32_t block_height) const {
    for (const auto& tx : transactions) {
        if (!utxo_set.apply_transaction(tx, block_height)) {
            return false;
        }
    }
    return true;
}

bool Block::rollback_from_utxo_set(UTXOSet& utxo_set) const {
    // Rollback in reverse order
    for (auto it = transactions.rbegin(); it != transactions.rend(); ++it) {
        if (!utxo_set.rollback_transaction(*it)) {
            return false;
        }
    }
    return true;
}

bool Block::mine(uint32_t max_iterations) {
    uint32_t iterations = 0;
    
    while (iterations < max_iterations) {
        header.nonce = iterations;
        
        if (header.meets_difficulty_target()) {
            clear_cache(); // Update cached hash
            return true;
        }
        
        ++iterations;
    }
    
    return false;
}

bool Block::has_coinbase() const {
    return !transactions.empty() && transactions[0].is_coinbase();
}

std::optional<const Transaction*> Block::get_coinbase() const {
    if (has_coinbase()) {
        return &transactions[0];
    }
    return std::nullopt;
}

void Block::clear_cache() const {
    cached_hash.reset();
    cached_merkle.reset();
}

Block Block::create_genesis_block(const std::string& genesis_message) {
    // Create genesis block header
    BlockHeader genesis_header;
    genesis_header.version = 1;
    genesis_header.prev_block_hash = Hash256{}; // All zeros
    genesis_header.timestamp = 1231006505; // Bitcoin genesis timestamp
    genesis_header.difficulty_target = 0x1D00FFFF; // Easy difficulty
    genesis_header.nonce = 0;
    
    // Create coinbase transaction
    std::vector<uint8_t> coinbase_data(genesis_message.begin(), genesis_message.end());
    auto coinbase = Transaction::create_coinbase_transaction(
        validation::GENESIS_BLOCK_REWARD, 
        0, 
        "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa", // Satoshi's address
        coinbase_data
    );
    
    std::vector<Transaction> transactions = { coinbase };
    Block genesis_block(genesis_header, std::move(transactions));
    
    // Update Merkle root
    genesis_block.update_merkle_root();
    
    return genesis_block;
}

Block Block::create_block_template(const Hash256& prev_block_hash, 
                                 const std::vector<Transaction>& txs,
                                 const std::string& miner_address,
                                 uint32_t difficulty_target) {
    // Create block header
    BlockHeader header;
    header.version = 2;
    header.prev_block_hash = prev_block_hash;
    header.timestamp = static_cast<uint32_t>(
        std::chrono::system_clock::to_time_t(std::chrono::system_clock::now())
    );
    header.difficulty_target = difficulty_target;
    
    // Calculate total fees
    // Note: This would need UTXO set in real implementation
    uint64_t total_fees = 0;
    
    // Create coinbase transaction
    auto coinbase = Transaction::create_coinbase_transaction(
        validation::GENESIS_BLOCK_REWARD, // Simplified
        total_fees,
        miner_address
    );
    
    // Combine transactions
    std::vector<Transaction> transactions = { coinbase };
    transactions.insert(transactions.end(), txs.begin(), txs.end());
    
    Block block(header, std::move(transactions));
    block.update_merkle_root();
    
    return block;
}

// Blockchain implementation
Blockchain::Blockchain() : current_difficulty_target(0x1D00FFFF) {}

void Blockchain::initialize_genesis() {
    auto genesis = std::make_unique<Block>(Block::create_genesis_block());
    
    // Apply genesis block to UTXO set
    genesis->apply_to_utxo_set(utxo_set, 0);
    
    blocks.push_back(std::move(genesis));
}

bool Blockchain::add_block(std::unique_ptr<Block> block) {
    std::unique_lock<std::shared_mutex> lock(chain_mutex);
    
    // Validate block
    if (!block->is_valid()) return false;
    
    // Check previous block hash
    if (!blocks.empty()) {
        auto latest = blocks.back().get();
        if (block->header.prev_block_hash != latest->get_hash()) {
            return false;
        }
    }
    
    // Apply to UTXO set
    if (!block->apply_to_utxo_set(utxo_set, static_cast<uint32_t>(blocks.size()))) {
        return false;
    }
    
    blocks.push_back(std::move(block));
    
    // Update difficulty if needed
    if (blocks.size() % DIFFICULTY_ADJUSTMENT_INTERVAL == 0) {
        current_difficulty_target = calculate_next_difficulty();
    }
    
    return true;
}

std::optional<const Block*> Blockchain::get_block(uint32_t height) const {
    std::shared_lock<std::shared_mutex> lock(chain_mutex);
    
    if (height >= blocks.size()) return std::nullopt;
    return blocks[height].get();
}

std::optional<const Block*> Blockchain::get_latest_block() const {
    std::shared_lock<std::shared_mutex> lock(chain_mutex);
    
    if (blocks.empty()) return std::nullopt;
    return blocks.back().get();
}

uint32_t Blockchain::get_height() const {
    std::shared_lock<std::shared_mutex> lock(chain_mutex);
    return static_cast<uint32_t>(blocks.size());
}

const UTXOSet& Blockchain::get_utxo_set() const {
    return utxo_set;
}

UTXOSet& Blockchain::get_utxo_set_mut() {
    return utxo_set;
}

uint64_t Blockchain::get_balance(const std::string& address) const {
    return utxo_set.get_balance(address);
}

uint32_t Blockchain::calculate_next_difficulty() const {
    // Simplified difficulty adjustment
    return current_difficulty_target;
}

uint32_t Blockchain::get_current_difficulty() const {
    return current_difficulty_target;
}

void Blockchain::set_difficulty_target(uint32_t target) {
    current_difficulty_target = target;
}

// Validation namespace
namespace validation {
    bool validate_block_size(const Block& block) {
        return block.get_serialized_size() <= MAX_BLOCK_SIZE;
    }
    
    bool validate_block_weight(const Block& block) {
        return block.get_weight() <= MAX_BLOCK_WEIGHT;
    }
    
    uint64_t calculate_block_reward(uint32_t height) {
        uint64_t reward = GENESIS_BLOCK_REWARD;
        uint32_t halvings = height / HALVING_INTERVAL;
        
        // Apply halvings
        for (uint32_t i = 0; i < halvings && reward > 0; ++i) {
            reward /= 2;
        }
        
        return reward;
    }
    
    bool validate_block(const Block& block, const Block* prev_block, 
                       const UTXOSet& utxo_set, uint32_t height) {
        if (!block.is_valid()) return false;
        if (!validate_block_size(block)) return false;
        if (!validate_block_weight(block)) return false;
        if (!block.validate_transactions(utxo_set)) return false;
        
        return true;
    }
}

// Mining namespace
namespace mining {
    MiningResult mine_block(Block& block, uint32_t max_iterations) {
        MiningResult result = {};
        result.success = false;
        
        auto start_time = std::chrono::high_resolution_clock::now();
        
        for (uint32_t nonce = 0; nonce < max_iterations; ++nonce) {
            block.header.nonce = nonce;
            result.iterations++;
            
            if (block.header.meets_difficulty_target()) {
                result.success = true;
                result.nonce = nonce;
                result.hash = block.get_hash();
                break;
            }
        }
        
        auto end_time = std::chrono::high_resolution_clock::now();
        auto duration = std::chrono::duration_cast<std::chrono::microseconds>(end_time - start_time);
        double time_seconds = duration.count() / 1000000.0;
        
        result.hash_rate = result.iterations / std::max(time_seconds, 0.001);
        
        return result;
    }
    
    bool hash_meets_target(const Hash256& hash, uint32_t difficulty_target) {
        BlockHeader temp_header;
        temp_header.difficulty_target = difficulty_target;
        auto target = temp_header.get_target();
        
        return std::memcmp(hash.data(), target.data(), 32) <= 0;
    }
}

} // namespace block
} // namespace blockchain