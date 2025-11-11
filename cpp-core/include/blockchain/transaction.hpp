#pragma once

#include "blockchain/crypto.hpp"
#include <vector>
#include <string>
#include <cstdint>
#include <memory>
#include <optional>
#include <unordered_map>
#include <shared_mutex>

namespace blockchain {
namespace transaction {

using namespace crypto;

/// Transaction input referencing a previous output
struct TxInput {
    Hash256 prev_tx_hash;      ///< Hash of the previous transaction
    uint32_t prev_output_index; ///< Index of the output in previous transaction
    std::vector<uint8_t> script_sig; ///< Unlocking script (signature + pubkey)
    uint32_t sequence = 0xFFFFFFFF; ///< Sequence number for RBF and timelocks
    
    /// Serialize input to bytes
    std::vector<uint8_t> serialize() const;
    
    /// Deserialize input from bytes
    static std::optional<TxInput> deserialize(const std::vector<uint8_t>& data, size_t& offset);
    
    /// Get serialized size
    size_t get_serialized_size() const;
    
    /// Check if this is a coinbase input (null hash + 0xFFFFFFFF index)
    bool is_coinbase() const;
    
    /// Create coinbase input with custom coinbase data
    static TxInput create_coinbase(const std::vector<uint8_t>& coinbase_data);
};

/// Transaction output defining value and conditions
struct TxOutput {
    uint64_t value;                   ///< Value in satoshis
    std::vector<uint8_t> script_pubkey; ///< Locking script (conditions to spend)
    
    /// Serialize output to bytes
    std::vector<uint8_t> serialize() const;
    
    /// Deserialize output from bytes
    static std::optional<TxOutput> deserialize(const std::vector<uint8_t>& data, size_t& offset);
    
    /// Get serialized size
    size_t get_serialized_size() const;
    
    /// Check if this is a valid output (non-dust, valid script)
    bool is_valid() const;
    
    /// Get address from script_pubkey (if standard format)
    std::optional<std::string> get_address() const;
    
    /// Create P2PKH output to address
    static TxOutput create_p2pkh(uint64_t value, const std::string& address);
    
    /// Create P2SH output to script hash
    static TxOutput create_p2sh(uint64_t value, const Hash256& script_hash);
};

/// Witness data for SegWit transactions
struct TxWitness {
    std::vector<std::vector<uint8_t>> witness_items;
    
    /// Serialize witness to bytes
    std::vector<uint8_t> serialize() const;
    
    /// Deserialize witness from bytes
    static std::optional<TxWitness> deserialize(const std::vector<uint8_t>& data, size_t& offset);
    
    /// Get serialized size
    size_t get_serialized_size() const;
    
    /// Check if witness is empty
    bool is_empty() const;
};

/// Complete transaction structure
class Transaction {
private:
    mutable std::optional<Hash256> cached_hash;     ///< Cached transaction hash
    mutable std::optional<Hash256> cached_wtxid;    ///< Cached witness transaction ID
    
public:
    uint32_t version = 2;                    ///< Transaction version
    std::vector<TxInput> inputs;             ///< Transaction inputs
    std::vector<TxOutput> outputs;           ///< Transaction outputs
    std::vector<TxWitness> witnesses;        ///< Witness data (SegWit)
    uint32_t locktime = 0;                   ///< Transaction locktime
    
    /// Default constructor
    Transaction() = default;
    
    /// Constructor with basic fields
    Transaction(uint32_t version, std::vector<TxInput> inputs, 
               std::vector<TxOutput> outputs, uint32_t locktime = 0);
    
    /// Serialize transaction to bytes
    std::vector<uint8_t> serialize() const;
    
    /// Serialize transaction without witness data (for legacy hash)
    std::vector<uint8_t> serialize_legacy() const;
    
    /// Deserialize transaction from bytes
    static std::optional<Transaction> deserialize(const std::vector<uint8_t>& data);
    
    /// Get serialized size including witness data
    size_t get_serialized_size() const;
    
    /// Get serialized size without witness data
    size_t get_base_size() const;
    
    /// Get transaction weight for fee calculation (base_size * 3 + total_size)
    size_t get_weight() const;
    
    /// Get virtual size for fee calculation (weight / 4, rounded up)
    size_t get_vsize() const;
    
    /// Calculate transaction hash (double SHA-256 of serialized data without witness)
    Hash256 get_hash() const;
    
    /// Calculate transaction hash (alias for get_hash for compatibility)
    Hash256 calculate_hash() const { return get_hash(); }
    
    /// Calculate witness transaction ID (includes witness data)
    Hash256 get_wtxid() const;
    
    /// Get transaction ID as hex string
    std::string get_txid() const;
    
    /// Check if transaction uses SegWit (has witness data)
    bool is_segwit() const;
    
    /// Check if transaction is coinbase
    bool is_coinbase() const;
    
    /// Validate transaction structure and rules
    bool is_valid() const;
    
    /// Get total input value (requires UTXO lookup)
    uint64_t get_total_input_value(const class UTXOSet& utxo_set) const;
    
    /// Get total output value
    uint64_t get_total_output_value() const;
    
    /// Calculate transaction fee (input_value - output_value)
    uint64_t calculate_fee(const class UTXOSet& utxo_set) const;
    
    /// Get fee rate in satoshis per virtual byte
    double get_fee_rate(const class UTXOSet& utxo_set) const;
    
    /// Sign transaction input with private key
    bool sign_input(size_t input_index, const PrivateKey& private_key, 
                   const TxOutput& prev_output, uint32_t sighash_type = 1);
    
    /// Verify signature for specific input
    bool verify_input_signature(size_t input_index, const TxOutput& prev_output, 
                               const PublicKey& public_key) const;
    
    /// Verify all input signatures
    bool verify_all_signatures(const class UTXOSet& utxo_set) const;
    
    /// Create signature hash for input signing
    Hash256 create_signature_hash(size_t input_index, const std::vector<uint8_t>& script_code,
                                 uint64_t amount, uint32_t sighash_type) const;
    
    /// Clear cached hashes (call after modifying transaction)
    void clear_cache() const;
    
    /// Create simple P2PKH transaction
    static Transaction create_p2pkh_transaction(
        const std::vector<std::pair<Hash256, uint32_t>>& inputs,
        const std::vector<std::pair<std::string, uint64_t>>& outputs,
        uint32_t locktime = 0
    );
    
    /// Create coinbase transaction
    static Transaction create_coinbase_transaction(
        uint64_t block_reward,
        uint64_t total_fees,
        const std::string& miner_address,
        const std::vector<uint8_t>& extra_data = {}
    );
};

/// UTXO (Unspent Transaction Output) for tracking spendable outputs
struct UTXO {
    Hash256 tx_hash;           ///< Transaction hash containing this output
    uint32_t output_index;     ///< Index of output in transaction
    TxOutput output;           ///< The actual output data
    uint32_t block_height;     ///< Height of block containing this UTXO
    bool is_coinbase;          ///< Whether this comes from a coinbase transaction
    
    /// Serialize UTXO to bytes
    std::vector<uint8_t> serialize() const;
    
    /// Deserialize UTXO from bytes
    static std::optional<UTXO> deserialize(const std::vector<uint8_t>& data);
    
    /// Get UTXO identifier (tx_hash + output_index)
    std::string get_outpoint() const;
    
    /// Check if UTXO is mature (coinbase requires 100 confirmations)
    bool is_mature(uint32_t current_height) const;
};

/// UTXO set for tracking unspent outputs
class UTXOSet {
private:
    std::unordered_map<std::string, UTXO> utxos; ///< outpoint -> UTXO mapping
    mutable std::shared_mutex mutex;              ///< Thread safety
    
public:
    /// Add UTXO to set
    void add_utxo(const UTXO& utxo);
    
    /// Remove UTXO from set
    bool remove_utxo(const Hash256& tx_hash, uint32_t output_index);
    
    /// Get UTXO by outpoint
    std::optional<UTXO> get_utxo(const Hash256& tx_hash, uint32_t output_index) const;
    
    /// Check if UTXO exists
    bool has_utxo(const Hash256& tx_hash, uint32_t output_index) const;
    
    /// Get all UTXOs for an address
    std::vector<UTXO> get_utxos_for_address(const std::string& address) const;
    
    /// Get total balance for an address
    uint64_t get_balance(const std::string& address) const;
    
    /// Apply transaction to UTXO set (remove inputs, add outputs)
    bool apply_transaction(const Transaction& tx, uint32_t block_height);
    
    /// Rollback transaction from UTXO set (add inputs, remove outputs)
    bool rollback_transaction(const Transaction& tx);
    
    /// Get total UTXO count
    size_t size() const;
    
    /// Get total value in UTXO set
    uint64_t get_total_value() const;
    
    /// Clear all UTXOs
    void clear();
    
    /// Validate UTXO set consistency
    bool validate() const;
    
    /// Serialize UTXO set to bytes
    std::vector<uint8_t> serialize() const;
    
    /// Deserialize UTXO set from bytes
    static std::optional<UTXOSet> deserialize(const std::vector<uint8_t>& data);
};

/// Transaction builder helper class
class TransactionBuilder {
private:
    Transaction tx;
    std::vector<std::pair<PrivateKey, PublicKey>> signing_keys;
    std::vector<TxOutput> prev_outputs;
    uint64_t total_input_value = 0;
    uint64_t fee_rate = 1000; // satoshis per kvB
    
public:
    /// Constructor
    TransactionBuilder(uint32_t version = 2);
    
    /// Add input with signing key
    TransactionBuilder& add_input(const Hash256& prev_tx_hash, uint32_t prev_output_index,
                                 const TxOutput& prev_output, const PrivateKey& signing_key);
    
    /// Add output
    TransactionBuilder& add_output(const std::string& address, uint64_t value);
    
    /// Add P2SH output
    TransactionBuilder& add_p2sh_output(const Hash256& script_hash, uint64_t value);
    
    /// Set fee rate (satoshis per kvB)
    TransactionBuilder& set_fee_rate(uint64_t rate);
    
    /// Set locktime
    TransactionBuilder& set_locktime(uint32_t locktime);
    
    /// Calculate and set appropriate fee, adding change output if needed
    TransactionBuilder& finalize_with_change(const std::string& change_address);
    
    /// Build and sign transaction
    std::optional<Transaction> build();
    
    /// Get estimated transaction size
    size_t estimate_size() const;
    
    /// Get estimated fee
    uint64_t estimate_fee() const;
};

/// Transaction validation rules
namespace validation {
    /// Maximum transaction size in bytes
    constexpr size_t MAX_TRANSACTION_SIZE = 100000;
    
    /// Maximum number of signature operations per transaction
    constexpr size_t MAX_SIGOPS = 20000;
    
    /// Minimum output value (dust threshold)
    constexpr uint64_t DUST_THRESHOLD = 546;
    
    /// Coinbase maturity (blocks before coinbase can be spent)
    constexpr uint32_t COINBASE_MATURITY = 100;
    
    /// Maximum locktime value
    constexpr uint32_t MAX_LOCKTIME = 500000000;
    
    /// Validate transaction size
    bool validate_size(const Transaction& tx);
    
    /// Validate transaction inputs
    bool validate_inputs(const Transaction& tx);
    
    /// Validate transaction outputs
    bool validate_outputs(const Transaction& tx);
    
    /// Validate transaction locktime
    bool validate_locktime(const Transaction& tx, uint32_t block_height, uint32_t block_time);
    
    /// Validate transaction fees
    bool validate_fees(const Transaction& tx, const UTXOSet& utxo_set);
    
    /// Full transaction validation
    bool validate_transaction(const Transaction& tx, const UTXOSet& utxo_set, 
                            uint32_t block_height, uint32_t block_time);
}

/// Transaction utilities
namespace utils {
    /// Convert transaction to JSON for debugging
    std::string transaction_to_json(const Transaction& tx);
    
    /// Parse transaction from hex string
    std::optional<Transaction> parse_transaction_hex(const std::string& hex);
    
    /// Convert transaction to hex string
    std::string transaction_to_hex(const Transaction& tx);
    
    /// Calculate optimal fee for confirmation target
    uint64_t calculate_optimal_fee(size_t tx_size, uint32_t confirmation_blocks);
    
    /// Estimate transaction confirmation time
    uint32_t estimate_confirmation_time(uint64_t fee_rate);
}

} // namespace transaction
} // namespace blockchain