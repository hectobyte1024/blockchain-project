#include "blockchain/transaction.hpp"
#include "blockchain/crypto.hpp"
#include <algorithm>
#include <sstream>
#include <iomanip>
#include <shared_mutex>
#include <set>

namespace blockchain {
namespace transaction {

using namespace crypto;

// Helper functions for serialization
namespace {
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
    
    void write_uint32_le(std::vector<uint8_t>& data, uint32_t value) {
        for (int i = 0; i < 4; ++i) {
            data.push_back((value >> (i * 8)) & 0xFF);
        }
    }
    
    void write_uint64_le(std::vector<uint8_t>& data, uint64_t value) {
        for (int i = 0; i < 8; ++i) {
            data.push_back((value >> (i * 8)) & 0xFF);
        }
    }
    
    void write_bytes(std::vector<uint8_t>& data, const std::vector<uint8_t>& bytes) {
        write_varint(data, bytes.size());
        data.insert(data.end(), bytes.begin(), bytes.end());
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
    
    std::optional<uint32_t> read_uint32_le(const std::vector<uint8_t>& data, size_t& offset) {
        if (offset + 4 > data.size()) return std::nullopt;
        uint32_t value = 0;
        for (int i = 0; i < 4; ++i) {
            value |= (static_cast<uint32_t>(data[offset + i]) << (i * 8));
        }
        offset += 4;
        return value;
    }
    
    std::optional<uint64_t> read_uint64_le(const std::vector<uint8_t>& data, size_t& offset) {
        if (offset + 8 > data.size()) return std::nullopt;
        uint64_t value = 0;
        for (int i = 0; i < 8; ++i) {
            value |= (static_cast<uint64_t>(data[offset + i]) << (i * 8));
        }
        offset += 8;
        return value;
    }
    
    std::optional<std::vector<uint8_t>> read_bytes(const std::vector<uint8_t>& data, size_t& offset) {
        auto length = read_varint(data, offset);
        if (!length || offset + *length > data.size()) return std::nullopt;
        
        std::vector<uint8_t> bytes(data.begin() + offset, data.begin() + offset + *length);
        offset += *length;
        return bytes;
    }
}

// TxInput implementation
std::vector<uint8_t> TxInput::serialize() const {
    std::vector<uint8_t> data;
    
    // Previous transaction hash (32 bytes)
    data.insert(data.end(), prev_tx_hash.begin(), prev_tx_hash.end());
    
    // Previous output index (4 bytes, little endian)
    write_uint32_le(data, prev_output_index);
    
    // Script signature
    write_bytes(data, script_sig);
    
    // Sequence number (4 bytes, little endian)
    write_uint32_le(data, sequence);
    
    return data;
}

std::optional<TxInput> TxInput::deserialize(const std::vector<uint8_t>& data, size_t& offset) {
    TxInput input;
    
    // Previous transaction hash
    if (offset + 32 > data.size()) return std::nullopt;
    std::copy(data.begin() + offset, data.begin() + offset + 32, input.prev_tx_hash.begin());
    offset += 32;
    
    // Previous output index
    auto prev_index = read_uint32_le(data, offset);
    if (!prev_index) return std::nullopt;
    input.prev_output_index = *prev_index;
    
    // Script signature
    auto script = read_bytes(data, offset);
    if (!script) return std::nullopt;
    input.script_sig = *script;
    
    // Sequence number
    auto seq = read_uint32_le(data, offset);
    if (!seq) return std::nullopt;
    input.sequence = *seq;
    
    return input;
}

size_t TxInput::get_serialized_size() const {
    size_t size = 32 + 4 + 4; // hash + index + sequence
    size += script_sig.size();
    if (script_sig.size() < 0xFD) size += 1;
    else if (script_sig.size() <= 0xFFFF) size += 3;
    else if (script_sig.size() <= 0xFFFFFFFF) size += 5;
    else size += 9;
    return size;
}

bool TxInput::is_coinbase() const {
    static const Hash256 null_hash = {};
    return prev_tx_hash == null_hash && prev_output_index == 0xFFFFFFFF;
}

TxInput TxInput::create_coinbase(const std::vector<uint8_t>& coinbase_data) {
    TxInput input;
    input.prev_tx_hash = {};  // Null hash
    input.prev_output_index = 0xFFFFFFFF;
    input.script_sig = coinbase_data;
    input.sequence = 0xFFFFFFFF;
    return input;
}

// TxOutput implementation
std::vector<uint8_t> TxOutput::serialize() const {
    std::vector<uint8_t> data;
    
    // Value (8 bytes, little endian)
    write_uint64_le(data, value);
    
    // Script pubkey
    write_bytes(data, script_pubkey);
    
    return data;
}

std::optional<TxOutput> TxOutput::deserialize(const std::vector<uint8_t>& data, size_t& offset) {
    TxOutput output;
    
    // Value
    auto val = read_uint64_le(data, offset);
    if (!val) return std::nullopt;
    output.value = *val;
    
    // Script pubkey
    auto script = read_bytes(data, offset);
    if (!script) return std::nullopt;
    output.script_pubkey = *script;
    
    return output;
}

size_t TxOutput::get_serialized_size() const {
    size_t size = 8; // value
    size += script_pubkey.size();
    if (script_pubkey.size() < 0xFD) size += 1;
    else if (script_pubkey.size() <= 0xFFFF) size += 3;
    else if (script_pubkey.size() <= 0xFFFFFFFF) size += 5;
    else size += 9;
    return size;
}

bool TxOutput::is_valid() const {
    return value > 0 && value >= validation::DUST_THRESHOLD && !script_pubkey.empty();
}

std::optional<std::string> TxOutput::get_address() const {
    // Simple P2PKH detection (OP_DUP OP_HASH160 <20 bytes> OP_EQUALVERIFY OP_CHECKSIG)
    if (script_pubkey.size() == 25 && 
        script_pubkey[0] == 0x76 && // OP_DUP
        script_pubkey[1] == 0xa9 && // OP_HASH160
        script_pubkey[2] == 0x14 && // Push 20 bytes
        script_pubkey[23] == 0x88 && // OP_EQUALVERIFY
        script_pubkey[24] == 0xac) { // OP_CHECKSIG
        
        // Extract 20-byte hash160
        std::vector<uint8_t> hash160(script_pubkey.begin() + 3, script_pubkey.begin() + 23);
        
        // Add version byte for mainnet P2PKH (0x00)
        std::vector<uint8_t> versioned_hash = {0x00};
        versioned_hash.insert(versioned_hash.end(), hash160.begin(), hash160.end());
        
        return Base58::encode_check(versioned_hash);
    }
    
    // Simple P2SH detection (OP_HASH160 <20 bytes> OP_EQUAL)
    if (script_pubkey.size() == 23 &&
        script_pubkey[0] == 0xa9 && // OP_HASH160
        script_pubkey[1] == 0x14 && // Push 20 bytes
        script_pubkey[22] == 0x87) { // OP_EQUAL
        
        // Extract 20-byte hash160
        std::vector<uint8_t> hash160(script_pubkey.begin() + 2, script_pubkey.begin() + 22);
        
        // Add version byte for mainnet P2SH (0x05)
        std::vector<uint8_t> versioned_hash = {0x05};
        versioned_hash.insert(versioned_hash.end(), hash160.begin(), hash160.end());
        
        return Base58::encode_check(versioned_hash);
    }
    
    return std::nullopt;
}

TxOutput TxOutput::create_p2pkh(uint64_t value, const std::string& address) {
    TxOutput output;
    output.value = value;
    
    // Decode address
    auto decoded = Base58::decode_check(address);
    if (!decoded || decoded->size() != 21 || (*decoded)[0] != 0x00) {
        // Invalid P2PKH address
        return output;
    }
    
    // Extract hash160
    std::vector<uint8_t> hash160(decoded->begin() + 1, decoded->end());
    
    // Create P2PKH script: OP_DUP OP_HASH160 <hash160> OP_EQUALVERIFY OP_CHECKSIG
    output.script_pubkey = {0x76, 0xa9, 0x14}; // OP_DUP OP_HASH160 Push20
    output.script_pubkey.insert(output.script_pubkey.end(), hash160.begin(), hash160.end());
    output.script_pubkey.insert(output.script_pubkey.end(), {0x88, 0xac}); // OP_EQUALVERIFY OP_CHECKSIG
    
    return output;
}

TxOutput TxOutput::create_p2sh(uint64_t value, const Hash256& script_hash) {
    TxOutput output;
    output.value = value;
    
    // Take first 20 bytes of script hash
    std::vector<uint8_t> hash160(script_hash.begin(), script_hash.begin() + 20);
    
    // Create P2SH script: OP_HASH160 <hash160> OP_EQUAL
    output.script_pubkey = {0xa9, 0x14}; // OP_HASH160 Push20
    output.script_pubkey.insert(output.script_pubkey.end(), hash160.begin(), hash160.end());
    output.script_pubkey.push_back(0x87); // OP_EQUAL
    
    return output;
}

// TxWitness implementation
std::vector<uint8_t> TxWitness::serialize() const {
    std::vector<uint8_t> data;
    
    write_varint(data, witness_items.size());
    for (const auto& item : witness_items) {
        write_bytes(data, item);
    }
    
    return data;
}

std::optional<TxWitness> TxWitness::deserialize(const std::vector<uint8_t>& data, size_t& offset) {
    TxWitness witness;
    
    auto item_count = read_varint(data, offset);
    if (!item_count) return std::nullopt;
    
    witness.witness_items.reserve(*item_count);
    for (uint64_t i = 0; i < *item_count; ++i) {
        auto item = read_bytes(data, offset);
        if (!item) return std::nullopt;
        witness.witness_items.push_back(*item);
    }
    
    return witness;
}

size_t TxWitness::get_serialized_size() const {
    size_t size = 0;
    
    // Item count varint
    if (witness_items.size() < 0xFD) size += 1;
    else if (witness_items.size() <= 0xFFFF) size += 3;
    else if (witness_items.size() <= 0xFFFFFFFF) size += 5;
    else size += 9;
    
    // Each item
    for (const auto& item : witness_items) {
        size += item.size();
        if (item.size() < 0xFD) size += 1;
        else if (item.size() <= 0xFFFF) size += 3;
        else if (item.size() <= 0xFFFFFFFF) size += 5;
        else size += 9;
    }
    
    return size;
}

bool TxWitness::is_empty() const {
    return witness_items.empty() || 
           std::all_of(witness_items.begin(), witness_items.end(), 
                      [](const std::vector<uint8_t>& item) { return item.empty(); });
}

// Transaction implementation
Transaction::Transaction(uint32_t version, std::vector<TxInput> inputs, 
                        std::vector<TxOutput> outputs, uint32_t locktime)
    : version(version), inputs(std::move(inputs)), outputs(std::move(outputs)), locktime(locktime) {
    // Initialize witnesses for SegWit compatibility
    witnesses.resize(this->inputs.size());
}

std::vector<uint8_t> Transaction::serialize() const {
    std::vector<uint8_t> data;
    
    // Version
    write_uint32_le(data, version);
    
    // Check if SegWit
    bool has_witness = is_segwit();
    
    if (has_witness) {
        // SegWit marker and flag
        data.push_back(0x00); // marker
        data.push_back(0x01); // flag
    }
    
    // Input count
    write_varint(data, inputs.size());
    
    // Inputs
    for (const auto& input : inputs) {
        auto input_data = input.serialize();
        data.insert(data.end(), input_data.begin(), input_data.end());
    }
    
    // Output count
    write_varint(data, outputs.size());
    
    // Outputs
    for (const auto& output : outputs) {
        auto output_data = output.serialize();
        data.insert(data.end(), output_data.begin(), output_data.end());
    }
    
    if (has_witness) {
        // Witness data
        for (const auto& witness : witnesses) {
            auto witness_data = witness.serialize();
            data.insert(data.end(), witness_data.begin(), witness_data.end());
        }
    }
    
    // Locktime
    write_uint32_le(data, locktime);
    
    return data;
}

std::vector<uint8_t> Transaction::serialize_legacy() const {
    std::vector<uint8_t> data;
    
    // Version
    write_uint32_le(data, version);
    
    // Input count
    write_varint(data, inputs.size());
    
    // Inputs
    for (const auto& input : inputs) {
        auto input_data = input.serialize();
        data.insert(data.end(), input_data.begin(), input_data.end());
    }
    
    // Output count
    write_varint(data, outputs.size());
    
    // Outputs
    for (const auto& output : outputs) {
        auto output_data = output.serialize();
        data.insert(data.end(), output_data.begin(), output_data.end());
    }
    
    // Locktime
    write_uint32_le(data, locktime);
    
    return data;
}

Hash256 Transaction::get_hash() const {
    if (!cached_hash) {
        auto data = serialize_legacy();
        cached_hash = SHA256::double_hash(data);
    }
    return *cached_hash;
}

Hash256 Transaction::get_wtxid() const {
    if (!cached_wtxid) {
        auto data = serialize();
        cached_wtxid = SHA256::double_hash(data);
    }
    return *cached_wtxid;
}

std::string Transaction::get_txid() const {
    auto hash = get_hash();
    // Reverse bytes for display (Bitcoin convention)
    std::reverse(hash.begin(), hash.end());
    return crypto::utils::to_hex(hash);
}

bool Transaction::is_segwit() const {
    return !witnesses.empty() && 
           std::any_of(witnesses.begin(), witnesses.end(),
                      [](const TxWitness& w) { return !w.is_empty(); });
}

bool Transaction::is_coinbase() const {
    return inputs.size() == 1 && inputs[0].is_coinbase();
}

bool Transaction::is_valid() const {
    // Basic structure validation
    if (inputs.empty() || outputs.empty()) return false;
    if (get_serialized_size() > validation::MAX_TRANSACTION_SIZE) return false;
    
    // Validate inputs
    for (const auto& input : inputs) {
        if (!is_coinbase() && input.is_coinbase()) return false;
    }
    
    // Validate outputs
    for (const auto& output : outputs) {
        if (!output.is_valid()) return false;
    }
    
    // Check witness count matches input count
    if (!witnesses.empty() && witnesses.size() != inputs.size()) return false;
    
    return true;
}

uint64_t Transaction::get_total_output_value() const {
    uint64_t total = 0;
    for (const auto& output : outputs) {
        total += output.value;
        // Check for overflow
        if (total < output.value) return UINT64_MAX;
    }
    return total;
}

size_t Transaction::get_serialized_size() const {
    return serialize().size();
}

size_t Transaction::get_base_size() const {
    return serialize_legacy().size();
}

size_t Transaction::get_weight() const {
    return get_base_size() * 3 + get_serialized_size();
}

size_t Transaction::get_vsize() const {
    return (get_weight() + 3) / 4; // Round up
}

void Transaction::clear_cache() const {
    cached_hash.reset();
    cached_wtxid.reset();
}

Transaction Transaction::create_coinbase_transaction(
    uint64_t block_reward,
    uint64_t total_fees,
    const std::string& miner_address,
    const std::vector<uint8_t>& extra_data) {
    
    Transaction tx;
    tx.version = 2;
    
    // Create coinbase input
    tx.inputs.push_back(TxInput::create_coinbase(extra_data));
    
    // Create output to miner
    uint64_t total_reward = block_reward + total_fees;
    tx.outputs.push_back(TxOutput::create_p2pkh(total_reward, miner_address));
    
    tx.locktime = 0;
    tx.witnesses.resize(1); // Empty witness for coinbase
    
    return tx;
}

// Validation namespace implementation
namespace validation {
    bool validate_size(const Transaction& tx) {
        return tx.get_serialized_size() <= MAX_TRANSACTION_SIZE;
    }
    
    bool validate_inputs(const Transaction& tx) {
        if (tx.inputs.empty()) return false;
        
        // Check for duplicate inputs (except coinbase)
        if (!tx.is_coinbase()) {
            std::set<std::pair<Hash256, uint32_t>> seen_inputs;
            for (const auto& input : tx.inputs) {
                auto outpoint = std::make_pair(input.prev_tx_hash, input.prev_output_index);
                if (seen_inputs.count(outpoint)) return false;
                seen_inputs.insert(outpoint);
            }
        }
        
        return true;
    }
    
    bool validate_outputs(const Transaction& tx) {
        if (tx.outputs.empty()) return false;
        
        for (const auto& output : tx.outputs) {
            if (!output.is_valid()) return false;
        }
        
        return true;
    }
    
    bool validate_transaction(const Transaction& tx, const UTXOSet& utxo_set, 
                            uint32_t block_height, uint32_t block_time) {
        if (!tx.is_valid()) return false;
        if (!validate_size(tx)) return false;
        if (!validate_inputs(tx)) return false;
        if (!validate_outputs(tx)) return false;
        
        return true;
    }
}

} // namespace transaction
} // namespace blockchain