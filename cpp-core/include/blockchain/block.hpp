#pragma once

#include "blockchain/transaction.hpp"
#include "blockchain/crypto.hpp"
#include <vector>
#include <string>
#include <cstdint>
#include <memory>
#include <optional>
#include <chrono>
#include <shared_mutex>
#include <mutex>

namespace blockchain {
namespace block {

using namespace crypto;
using namespace transaction;

/// Block header containing metadata and Merkle root
struct BlockHeader {
    uint32_t version = 1;              ///< Block version
    Hash256 prev_block_hash;           ///< Hash of previous block
    Hash256 merkle_root;               ///< Merkle root of transactions
    uint32_t timestamp;                ///< Block timestamp (Unix time)
    uint32_t difficulty_target;        ///< Difficulty target (nBits format)
    uint32_t nonce = 0;                ///< Proof-of-work nonce
    uint32_t height = 0;               ///< Block height in the chain
    
    /// Default constructor
    BlockHeader() : timestamp(static_cast<uint32_t>(std::chrono::system_clock::to_time_t(std::chrono::system_clock::now()))) {}
    
    /// Constructor with parameters
    BlockHeader(uint32_t version, const Hash256& prev_hash, const Hash256& merkle_root,
               uint32_t timestamp, uint32_t difficulty_target, uint32_t nonce = 0);
    
    /// Serialize header to bytes
    std::vector<uint8_t> serialize() const;
    
    /// Deserialize header from bytes
    static std::optional<BlockHeader> deserialize(const std::vector<uint8_t>& data);
    
    /// Calculate block hash (double SHA-256 of serialized header)
    Hash256 calculate_hash() const;
    
    /// Get block hash as hex string (reversed for display)
    std::string get_hash_string() const;
    
    /// Check if block meets difficulty target
    bool meets_difficulty_target() const;
    
    /// Get difficulty as a double
    double get_difficulty() const;
    
    /// Get target as 256-bit number
    std::array<uint8_t, 32> get_target() const;
    
    /// Convert difficulty target to compact nBits format
    static uint32_t difficulty_to_nbits(double difficulty);
    
    /// Convert compact nBits to difficulty
    static double nbits_to_difficulty(uint32_t nbits);
    
    /// Validate header structure
    bool is_valid() const;
    
    /// Get serialized size (always 80 bytes)
    static constexpr size_t SERIALIZED_SIZE = 80;
};

/// Complete block with header and transactions
class Block {
private:
    mutable std::optional<Hash256> cached_hash;      ///< Cached block hash
    mutable std::optional<Hash256> cached_merkle;    ///< Cached Merkle root
    
public:
    BlockHeader header;                    ///< Block header
    std::vector<Transaction> transactions; ///< Block transactions
    
    /// Default constructor
    Block() = default;
    
    /// Constructor with header and transactions
    Block(const BlockHeader& header, std::vector<Transaction> transactions);
    
    /// Serialize block to bytes
    std::vector<uint8_t> serialize() const;
    
    /// Deserialize block from bytes
    static std::optional<Block> deserialize(const std::vector<uint8_t>& data);
    
    /// Get block hash
    Hash256 get_hash() const;
    
    /// Calculate block hash (alias for get_hash for compatibility)
    Hash256 calculate_hash() const { return get_hash(); }
    
    /// Get block hash as hex string
    std::string get_hash_string() const;
    
    /// Calculate Merkle root from transactions
    Hash256 calculate_merkle_root() const;
    
    /// Update header Merkle root from transactions
    void update_merkle_root();
    
    /// Get serialized size
    size_t get_serialized_size() const;
    
    /// Get block weight for fee calculation
    size_t get_weight() const;
    
    /// Get number of transactions
    size_t get_transaction_count() const;
    
    /// Get total transaction fees
    uint64_t get_total_fees(const UTXOSet& utxo_set) const;
    
    /// Get block reward (coinbase output value)
    uint64_t get_block_reward() const;
    
    /// Check if block is valid
    bool is_valid() const;
    
    /// Validate block structure
    bool validate_structure() const;
    
    /// Validate all transactions
    bool validate_transactions(const UTXOSet& utxo_set) const;
    
    /// Validate Merkle root
    bool validate_merkle_root() const;
    
    /// Validate proof of work
    bool validate_proof_of_work() const;
    
    /// Apply block transactions to UTXO set
    bool apply_to_utxo_set(UTXOSet& utxo_set, uint32_t block_height) const;
    
    /// Rollback block transactions from UTXO set
    bool rollback_from_utxo_set(UTXOSet& utxo_set) const;
    
    /// Mine block (find valid nonce)
    bool mine(uint32_t max_iterations = 1000000);
    
    /// Check if block has coinbase transaction
    bool has_coinbase() const;
    
    /// Get coinbase transaction
    std::optional<const Transaction*> get_coinbase() const;
    
    /// Clear cached values (call after modifying block)
    void clear_cache() const;
    
    /// Create genesis block
    static Block create_genesis_block(const std::string& genesis_message = "Genesis Block");
    
    /// Create new block from template
    static Block create_block_template(const Hash256& prev_block_hash, 
                                     const std::vector<Transaction>& transactions,
                                     const std::string& miner_address,
                                     uint32_t difficulty_target);
};

/// Blockchain state and management
class Blockchain {
private:
    std::vector<std::unique_ptr<Block>> blocks;  ///< Chain of blocks
    UTXOSet utxo_set;                           ///< Current UTXO set
    mutable std::shared_mutex chain_mutex;       ///< Thread safety
    uint32_t current_difficulty_target;          ///< Current mining difficulty
    
    /// Difficulty adjustment parameters
    static constexpr uint32_t DIFFICULTY_ADJUSTMENT_INTERVAL = 2016; // blocks
    static constexpr uint32_t TARGET_BLOCK_TIME = 600; // 10 minutes in seconds
    static constexpr uint32_t MAX_DIFFICULTY_ADJUSTMENT = 4; // 4x up or 1/4x down
    
public:
    /// Constructor
    Blockchain();
    
    /// Initialize with genesis block
    void initialize_genesis();
    
    /// Add block to chain
    bool add_block(std::unique_ptr<Block> block);
    
    /// Get block by height
    std::optional<const Block*> get_block(uint32_t height) const;
    
    /// Get block by hash
    std::optional<const Block*> get_block_by_hash(const Hash256& hash) const;
    
    /// Get latest block
    std::optional<const Block*> get_latest_block() const;
    
    /// Get chain height (number of blocks)
    uint32_t get_height() const;
    
    /// Get total work (sum of difficulties)
    double get_total_work() const;
    
    /// Validate entire chain
    bool validate_chain() const;
    
    /// Get UTXO set
    const UTXOSet& get_utxo_set() const;
    
    /// Get mutable UTXO set (for testing)
    UTXOSet& get_utxo_set_mut();
    
    /// Find transaction by hash
    std::optional<std::pair<const Transaction*, uint32_t>> find_transaction(const Hash256& tx_hash) const;
    
    /// Get balance for address
    uint64_t get_balance(const std::string& address) const;
    
    /// Get transaction history for address
    std::vector<std::pair<const Transaction*, uint32_t>> get_transaction_history(const std::string& address) const;
    
    /// Calculate next difficulty target
    uint32_t calculate_next_difficulty() const;
    
    /// Get current difficulty target
    uint32_t get_current_difficulty() const;
    
    /// Set difficulty target (for testing)
    void set_difficulty_target(uint32_t target);
    
    /// Reorganize chain (handle forks)
    bool reorganize_chain(const std::vector<std::unique_ptr<Block>>& new_chain);
    
    /// Serialize entire blockchain
    std::vector<uint8_t> serialize() const;
    
    /// Deserialize blockchain
    static std::optional<Blockchain> deserialize(const std::vector<uint8_t>& data);
    
    /// Export chain to JSON
    std::string to_json() const;
    
    /// Get chain statistics
    struct ChainStats {
        uint32_t height;
        uint32_t total_transactions;
        uint64_t total_value;
        double average_block_time;
        double current_difficulty;
        size_t utxo_count;
    };
    
    ChainStats get_statistics() const;
};

/// Block validation rules
namespace validation {
    /// Maximum block size in bytes
    constexpr size_t MAX_BLOCK_SIZE = 4000000; // 4 MB
    
    /// Maximum block weight
    constexpr size_t MAX_BLOCK_WEIGHT = 4000000;
    
    /// Maximum transactions per block
    constexpr size_t MAX_TRANSACTIONS_PER_BLOCK = 10000;
    
    /// Genesis block reward (50 BTC in satoshis)
    constexpr uint64_t GENESIS_BLOCK_REWARD = 5000000000ULL;
    
    /// Block reward halving interval
    constexpr uint32_t HALVING_INTERVAL = 210000;
    
    /// Maximum timestamp drift (2 hours)
    constexpr uint32_t MAX_TIMESTAMP_DRIFT = 7200;
    
    /// Validate block size
    bool validate_block_size(const Block& block);
    
    /// Validate block weight
    bool validate_block_weight(const Block& block);
    
    /// Validate block timestamp
    bool validate_timestamp(const Block& block, const Block* prev_block);
    
    /// Validate difficulty target
    bool validate_difficulty(const Block& block, uint32_t expected_target);
    
    /// Calculate block reward for height
    uint64_t calculate_block_reward(uint32_t height);
    
    /// Validate coinbase transaction
    bool validate_coinbase(const Transaction& coinbase, uint32_t height, uint64_t total_fees);
    
    /// Full block validation
    bool validate_block(const Block& block, const Block* prev_block, 
                       const UTXOSet& utxo_set, uint32_t height);
}

/// Mining utilities
namespace mining {
    /// Mining result
    struct MiningResult {
        bool success;
        uint32_t nonce;
        Hash256 hash;
        uint64_t iterations;
        double hash_rate; // hashes per second
    };
    
    /// Mine block with specific parameters
    MiningResult mine_block(Block& block, uint32_t max_iterations = UINT32_MAX);
    
    /// Check if hash meets difficulty target
    bool hash_meets_target(const Hash256& hash, uint32_t difficulty_target);
    
    /// Calculate hash rate from mining result
    double calculate_hash_rate(uint64_t iterations, double time_seconds);
    
    /// Estimate mining time
    double estimate_mining_time(uint32_t difficulty_target, double hash_rate);
    
    /// Create mining template
    Block create_mining_template(const Blockchain& chain, 
                               const std::vector<Transaction>& mempool_txs,
                               const std::string& miner_address);
}

/// Block utilities
namespace utils {
    /// Convert block to JSON for debugging
    std::string block_to_json(const Block& block);
    
    /// Parse block from hex string
    std::optional<Block> parse_block_hex(const std::string& hex);
    
    /// Convert block to hex string
    std::string block_to_hex(const Block& block);
    
    /// Calculate block subsidy for height
    uint64_t calculate_subsidy(uint32_t height);
    
    /// Format hash for display (reversed hex)
    std::string format_hash(const Hash256& hash);
    
    /// Get human-readable block size
    std::string format_size(size_t bytes);
    
    /// Get human-readable timestamp
    std::string format_timestamp(uint32_t timestamp);
}

} // namespace block
} // namespace blockchain