#pragma once

#include "crypto.hpp"
#include <vector>
#include <string>
#include <cstdint>
#include <chrono>
#include <memory>
#include <optional>

namespace blockchain::core {

using Timestamp = uint64_t;
using BlockHeight = uint64_t;
using Amount = uint64_t; // Satoshi precision (1e-8)
using Nonce = uint64_t;
using Difficulty = uint32_t;

/**
 * @brief Transaction input referencing a previous transaction output
 */
struct TransactionInput {
    crypto::Hash256 previous_transaction_hash;
    uint32_t output_index;
    std::vector<uint8_t> script_sig; // Unlocking script
    uint32_t sequence;
    
    // Serialization
    std::vector<uint8_t> serialize() const;
    static std::optional<TransactionInput> deserialize(const std::vector<uint8_t>& data, size_t& offset);
    
    bool operator==(const TransactionInput& other) const;
    size_t serialized_size() const;
};

/**
 * @brief Transaction output with value and locking script
 */
struct TransactionOutput {
    Amount value;
    std::vector<uint8_t> script_pubkey; // Locking script
    
    // Serialization
    std::vector<uint8_t> serialize() const;
    static std::optional<TransactionOutput> deserialize(const std::vector<uint8_t>& data, size_t& offset);
    
    bool operator==(const TransactionOutput& other) const;
    size_t serialized_size() const;
};

/**
 * @brief Individual transaction within the blockchain
 */
class Transaction {
public:
    uint32_t version;
    std::vector<TransactionInput> inputs;
    std::vector<TransactionOutput> outputs;
    uint32_t lock_time;
    
    Transaction() : version(1), lock_time(0) {}
    
    // Transaction hash calculation
    crypto::Hash256 get_hash() const;
    crypto::Hash256 get_witness_hash() const; // For SegWit support
    
    // Validation
    bool is_coinbase() const;
    Amount get_input_value() const; // Requires UTXO set lookup
    Amount get_output_value() const;
    Amount get_fee() const; // input_value - output_value
    
    // Serialization
    std::vector<uint8_t> serialize() const;
    static std::optional<Transaction> deserialize(const std::vector<uint8_t>& data);
    
    // Size calculations
    size_t get_size() const;
    size_t get_virtual_size() const; // For fee calculation with SegWit
    
    bool operator==(const Transaction& other) const;
};

/**
 * @brief Block header containing metadata and Merkle root
 */
struct BlockHeader {
    uint32_t version;
    crypto::Hash256 previous_block_hash;
    crypto::Hash256 merkle_root;
    Timestamp timestamp;
    Difficulty difficulty_target;
    Nonce nonce;
    
    BlockHeader() : version(1), timestamp(0), difficulty_target(0), nonce(0) {}
    
    // Header hash for Proof of Work
    crypto::Hash256 get_hash() const;
    
    // Proof of Work validation
    bool meets_difficulty_target() const;
    double get_difficulty() const;
    
    // Serialization
    std::vector<uint8_t> serialize() const;
    static std::optional<BlockHeader> deserialize(const std::vector<uint8_t>& data);
    
    bool operator==(const BlockHeader& other) const;
    
    static constexpr size_t SERIALIZED_SIZE = 80;
};

/**
 * @brief Complete block with header and transactions
 */
class Block {
public:
    BlockHeader header;
    std::vector<Transaction> transactions;
    
    Block() = default;
    explicit Block(const BlockHeader& header) : header(header) {}
    
    // Block hash (same as header hash)
    crypto::Hash256 get_hash() const { return header.get_hash(); }
    
    // Merkle tree operations
    crypto::Hash256 calculate_merkle_root() const;
    std::vector<crypto::Hash256> get_transaction_hashes() const;
    
    // Block validation
    bool is_valid() const;
    bool validate_merkle_root() const;
    bool validate_transactions() const;
    
    // Block properties
    size_t get_size() const;
    Amount get_total_fees() const;
    Amount get_block_reward(BlockHeight height) const;
    size_t get_transaction_count() const { return transactions.size(); }
    
    // Coinbase transaction
    const Transaction& get_coinbase_transaction() const;
    bool has_valid_coinbase() const;
    
    // Serialization
    std::vector<uint8_t> serialize() const;
    static std::optional<Block> deserialize(const std::vector<uint8_t>& data);
    
    bool operator==(const Block& other) const;
};

/**
 * @brief UTXO (Unspent Transaction Output) for efficient validation
 */
struct UTXO {
    TransactionOutput output;
    BlockHeight block_height;
    bool is_coinbase;
    
    UTXO(const TransactionOutput& out, BlockHeight height, bool coinbase)
        : output(out), block_height(height), is_coinbase(coinbase) {}
    
    // Serialization
    std::vector<uint8_t> serialize() const;
    static std::optional<UTXO> deserialize(const std::vector<uint8_t>& data);
    
    bool operator==(const UTXO& other) const;
};

/**
 * @brief Outpoint uniquely identifying a transaction output
 */
struct OutPoint {
    crypto::Hash256 transaction_hash;
    uint32_t output_index;
    
    OutPoint() : output_index(0) {}
    OutPoint(const crypto::Hash256& hash, uint32_t index) 
        : transaction_hash(hash), output_index(index) {}
    
    bool operator==(const OutPoint& other) const;
    bool operator<(const OutPoint& other) const;
    
    std::string to_string() const;
    
    // Serialization
    std::vector<uint8_t> serialize() const;
    static std::optional<OutPoint> deserialize(const std::vector<uint8_t>& data);
};

/**
 * @brief Chain parameters and constants
 */
struct ChainParams {
    // Genesis block
    static Block create_genesis_block();
    
    // Consensus parameters
    static constexpr BlockHeight COINBASE_MATURITY = 100;
    static constexpr Amount INITIAL_BLOCK_REWARD = 50ULL * 100000000ULL; // 50 coins
    static constexpr BlockHeight HALVING_INTERVAL = 210000;
    static constexpr Timestamp TARGET_BLOCK_TIME = 600; // 10 minutes
    static constexpr BlockHeight DIFFICULTY_ADJUSTMENT_INTERVAL = 2016;
    
    // Maximum values
    static constexpr size_t MAX_BLOCK_SIZE = 1000000; // 1MB
    static constexpr size_t MAX_TRANSACTION_SIZE = 100000;
    static constexpr Amount MAX_MONEY = 21000000ULL * 100000000ULL; // 21M coins
    
    // Script validation
    static constexpr size_t MAX_SCRIPT_SIZE = 10000;
    static constexpr size_t MAX_SCRIPT_ELEMENT_SIZE = 520;
    static constexpr size_t MAX_SCRIPT_OPCODES = 201;
    
    // Network magic bytes for message framing
    static constexpr uint32_t MAINNET_MAGIC = 0xD9B4BEF9;
    static constexpr uint32_t TESTNET_MAGIC = 0xDAB5BFFA;
    static constexpr uint32_t REGTEST_MAGIC = 0xFABFB5DA;
};

} // namespace blockchain::core