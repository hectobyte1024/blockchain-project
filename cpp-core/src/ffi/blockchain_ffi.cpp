#include "ffi/blockchain_ffi.h"
#include "blockchain/crypto.hpp"
#include <memory>
#include <cstring>
#include <stdexcept>

// Don't use 'using namespace' to avoid conflicts
// Use qualified names instead

// Opaque structures for FFI
struct CryptoEngine {
    // For now, this is stateless, but could hold context in the future
    bool initialized = true;
};

struct ConsensusEngine {
    // Consensus state would go here
    bool initialized = true;
};

struct StorageEngine {
    std::string database_path;
    bool initialized = true;
    
    StorageEngine(const char* path) : database_path(path) {}
};

struct VMEngine {
    // VM state would go here
    bool initialized = true;
};

// Helper macro for exception handling in C interface
#define HANDLE_EXCEPTIONS(code) \
    try { \
        code \
    } catch (const std::exception& e) { \
        return BLOCKCHAIN_ERROR_UNKNOWN; \
    } catch (...) { \
        return BLOCKCHAIN_ERROR_UNKNOWN; \
    }

// =============================================================================
// CRYPTO ENGINE FFI IMPLEMENTATION
// =============================================================================

extern "C" {

CryptoEngine* crypto_engine_new() {
    try {
        return new CryptoEngine();
    } catch (...) {
        return nullptr;
    }
}

void crypto_engine_destroy(CryptoEngine* engine) {
    delete engine;
}

BlockchainResult crypto_sha256(const uint8_t* input, size_t input_len, Hash256* output) {
    HANDLE_EXCEPTIONS({
        if (!input || !output || input_len == 0) {
            return BLOCKCHAIN_ERROR_INVALID_INPUT;
        }
        
        std::vector<uint8_t> data(input, input + input_len);
        auto result = blockchain::crypto::SHA256::hash(data);
        std::memcpy(output->data, result.data(), 32);
        
        return BLOCKCHAIN_SUCCESS;
    });
}

BlockchainResult crypto_double_sha256(const uint8_t* input, size_t input_len, Hash256* output) {
    HANDLE_EXCEPTIONS({
        if (!input || !output || input_len == 0) {
            return BLOCKCHAIN_ERROR_INVALID_INPUT;
        }
        
        std::vector<uint8_t> data(input, input + input_len);
        auto result = blockchain::crypto::SHA256::double_hash(data);
        std::memcpy(output->data, result.data(), 32);
        
        return BLOCKCHAIN_SUCCESS;
    });
}

BlockchainResult crypto_ripemd160(const uint8_t* input, size_t input_len, Hash160* output) {
    HANDLE_EXCEPTIONS({
        if (!input || !output || input_len == 0) {
            return BLOCKCHAIN_ERROR_INVALID_INPUT;
        }
        
        std::vector<uint8_t> data(input, input + input_len);
        auto result = blockchain::crypto::RIPEMD160::hash(data);
        std::memcpy(output->data, result.data(), 20);
        
        return BLOCKCHAIN_SUCCESS;
    });
}

BlockchainResult crypto_generate_private_key(PrivateKey* private_key) {
    HANDLE_EXCEPTIONS({
        if (!private_key) {
            return BLOCKCHAIN_ERROR_INVALID_INPUT;
        }
        
        auto result = blockchain::crypto::ECDSA::generate_private_key();
        std::memcpy(private_key->data, result.data(), 32);
        
        return BLOCKCHAIN_SUCCESS;
    });
}

BlockchainResult crypto_derive_public_key(const PrivateKey* private_key, PublicKey* public_key) {
    HANDLE_EXCEPTIONS({
        if (!private_key || !public_key) {
            return BLOCKCHAIN_ERROR_INVALID_INPUT;
        }
        
        // Convert C struct to C++ array
        blockchain::crypto::PrivateKey cpp_private_key;
        std::memcpy(cpp_private_key.data(), private_key->data, 32);
        
        auto result = blockchain::crypto::ECDSA::derive_public_key(cpp_private_key);
        if (!result) {
            return BLOCKCHAIN_ERROR_INVALID_INPUT;
        }
        
        std::memcpy(public_key->data, result->data(), 33);
        return BLOCKCHAIN_SUCCESS;
    });
}

bool crypto_is_valid_private_key(const PrivateKey* private_key) {
    if (!private_key) {
        return false;
    }
    
    try {
        blockchain::crypto::PrivateKey cpp_private_key;
        std::memcpy(cpp_private_key.data(), private_key->data, 32);
        return blockchain::crypto::ECDSA::is_valid_private_key(cpp_private_key);
    } catch (...) {
        return false;
    }
}

bool crypto_is_valid_public_key(const PublicKey* public_key) {
    if (!public_key) {
        return false;
    }
    
    try {
        blockchain::crypto::PublicKey cpp_public_key;
        std::memcpy(cpp_public_key.data(), public_key->data, 33);
        return blockchain::crypto::ECDSA::is_valid_public_key(cpp_public_key);
    } catch (...) {
        return false;
    }
}

BlockchainResult crypto_sign_message(const PrivateKey* private_key, const Hash256* message_hash, Signature* signature) {
    HANDLE_EXCEPTIONS({
        if (!private_key || !message_hash || !signature) {
            return BLOCKCHAIN_ERROR_INVALID_INPUT;
        }
        
        // Convert C structs to C++ arrays
        blockchain::crypto::PrivateKey cpp_private_key;
        blockchain::crypto::Hash256 cpp_hash;
        std::memcpy(cpp_private_key.data(), private_key->data, 32);
        std::memcpy(cpp_hash.data(), message_hash->data, 32);
        
        auto result = blockchain::crypto::ECDSA::sign(cpp_hash, cpp_private_key);
        if (!result) {
            return BLOCKCHAIN_ERROR_INVALID_SIGNATURE;
        }
        
        std::memcpy(signature->data, result->data(), 64);
        return BLOCKCHAIN_SUCCESS;
    });
}

BlockchainResult crypto_verify_signature(const PublicKey* public_key, const Hash256* message_hash, const Signature* signature, bool* is_valid) {
    HANDLE_EXCEPTIONS({
        if (!public_key || !message_hash || !signature || !is_valid) {
            return BLOCKCHAIN_ERROR_INVALID_INPUT;
        }
        
        // Convert C structs to C++ arrays
        blockchain::crypto::PublicKey cpp_public_key;
        blockchain::crypto::Hash256 cpp_hash;
        blockchain::crypto::Signature cpp_signature;
        std::memcpy(cpp_public_key.data(), public_key->data, 33);
        std::memcpy(cpp_hash.data(), message_hash->data, 32);
        std::memcpy(cpp_signature.data(), signature->data, 64);
        
        *is_valid = blockchain::crypto::ECDSA::verify(cpp_hash, cpp_signature, cpp_public_key);
        return BLOCKCHAIN_SUCCESS;
    });
}

BlockchainResult crypto_calculate_merkle_root(const Hash256* leaf_hashes, size_t leaf_count, Hash256* root) {
    HANDLE_EXCEPTIONS({
        if (!leaf_hashes || !root || leaf_count == 0) {
            return BLOCKCHAIN_ERROR_INVALID_INPUT;
        }
        
        // Convert C array to C++ vector
        std::vector<blockchain::crypto::Hash256> cpp_hashes(leaf_count);
        for (size_t i = 0; i < leaf_count; ++i) {
            std::memcpy(cpp_hashes[i].data(), leaf_hashes[i].data, 32);
        }
        
        blockchain::crypto::MerkleTree tree(cpp_hashes);
        auto result = tree.get_root();
        std::memcpy(root->data, result.data(), 32);
        
        return BLOCKCHAIN_SUCCESS;
    });
}

BlockchainResult crypto_verify_merkle_proof(const Hash256* leaf_hash, const Hash256* proof, size_t proof_length,
                                          const Hash256* root, size_t leaf_index, size_t tree_size, bool* is_valid) {
    try {
        if (!leaf_hash || !proof || !root || !is_valid) {
            return BLOCKCHAIN_ERROR_INVALID_INPUT;
        }
        
        // Convert C structs to C++ types
        blockchain::crypto::Hash256 cpp_leaf_hash, cpp_root;
        std::memcpy(cpp_leaf_hash.data(), leaf_hash->data, 32);
        std::memcpy(cpp_root.data(), root->data, 32);
        
        std::vector<blockchain::crypto::Hash256> cpp_proof(proof_length);
        for (size_t i = 0; i < proof_length; ++i) {
            std::memcpy(cpp_proof[i].data(), proof[i].data, 32);
        }
        
        *is_valid = blockchain::crypto::MerkleTree::verify_proof(cpp_leaf_hash, cpp_proof, cpp_root, leaf_index, tree_size);
        return BLOCKCHAIN_SUCCESS;
    } catch (...) {
        return BLOCKCHAIN_ERROR_UNKNOWN;
    }
}

// =============================================================================
// CONSENSUS ENGINE FFI IMPLEMENTATION
// =============================================================================

ConsensusEngine* consensus_engine_new() {
    try {
        return new ConsensusEngine();
    } catch (...) {
        return nullptr;
    }
}

void consensus_engine_destroy(ConsensusEngine* engine) {
    delete engine;
}

BlockchainResult consensus_validate_block_header(ConsensusEngine* engine, const BlockHeader* header, bool* is_valid) {
    HANDLE_EXCEPTIONS({
        if (!engine || !header || !is_valid) {
            return BLOCKCHAIN_ERROR_INVALID_INPUT;
        }
        
        // Basic header validation (timestamp, difficulty, etc.)
        // For now, just check that nonce is not zero (placeholder validation)
        *is_valid = (header->nonce != 0);
        
        return BLOCKCHAIN_SUCCESS;
    });
}

BlockchainResult consensus_check_proof_of_work(const BlockHeader* header, bool* meets_target) {
    HANDLE_EXCEPTIONS({
        if (!header || !meets_target) {
            return BLOCKCHAIN_ERROR_INVALID_INPUT;
        }
        
        // Calculate block hash and check against difficulty target
        // This is a simplified implementation - real PoW validation would be more complex
        
        // Serialize header for hashing
        std::vector<uint8_t> header_data(80); // Block header is 80 bytes
        size_t offset = 0;
        
        // Pack header fields (little-endian)
        std::memcpy(&header_data[offset], &header->version, 4); offset += 4;
        std::memcpy(&header_data[offset], header->previous_block_hash.data, 32); offset += 32;
        std::memcpy(&header_data[offset], header->merkle_root.data, 32); offset += 32;
        std::memcpy(&header_data[offset], &header->timestamp, 8); offset += 8;
        std::memcpy(&header_data[offset], &header->difficulty_target, 4); offset += 4;
        std::memcpy(&header_data[offset], &header->nonce, 8); offset += 8;
        
        // Double SHA-256 hash
        auto block_hash = blockchain::crypto::SHA256::double_hash(header_data);
        
        // Check if hash meets difficulty target (leading zeros)
        // This is simplified - real Bitcoin uses target comparison
        uint32_t leading_zero_bits = 0;
        for (size_t i = 0; i < 32; ++i) {
            if (block_hash[i] == 0) {
                leading_zero_bits += 8;
            } else {
                // Count leading zero bits in this byte
                uint8_t byte = block_hash[i];
                while ((byte & 0x80) == 0 && leading_zero_bits < 256) {
                    leading_zero_bits++;
                    byte <<= 1;
                }
                break;
            }
        }
        
        // Check if we have enough leading zero bits for the difficulty
        *meets_target = leading_zero_bits >= header->difficulty_target;
        
        return BLOCKCHAIN_SUCCESS;
    });
}

BlockchainResult consensus_validate_transaction(ConsensusEngine* engine, const Transaction* transaction, bool* is_valid) {
    HANDLE_EXCEPTIONS({
        if (!engine || !transaction || !is_valid) {
            return BLOCKCHAIN_ERROR_INVALID_INPUT;
        }
        
        // Basic transaction validation
        *is_valid = (transaction->input_count > 0 && 
                    transaction->output_count > 0 && 
                    transaction->lock_time >= 0);
        
        return BLOCKCHAIN_SUCCESS;
    });
}

BlockchainResult consensus_validate_block(ConsensusEngine* engine, const Block* block, bool* is_valid) {
    HANDLE_EXCEPTIONS({
        if (!engine || !block || !is_valid) {
            return BLOCKCHAIN_ERROR_INVALID_INPUT;
        }
        
        // Basic block validation
        bool header_valid = false;
        BlockchainResult header_result = consensus_validate_block_header(engine, &block->header, &header_valid);
        if (header_result != BLOCKCHAIN_SUCCESS) {
            return header_result;
        }
        
        *is_valid = header_valid && (block->transaction_count > 0);
        return BLOCKCHAIN_SUCCESS;
    });
}

BlockchainResult consensus_calculate_difficulty_adjustment(ConsensusEngine* engine, uint64_t current_height, 
                                                         uint64_t current_timestamp, uint32_t* new_target) {
    HANDLE_EXCEPTIONS({
        if (!engine || !new_target) {
            return BLOCKCHAIN_ERROR_INVALID_INPUT;
        }
        
        // Simple difficulty adjustment based on time
        // In real implementation, this would use block history
        *new_target = 0x1e00ffff; // Default difficulty
        
        return BLOCKCHAIN_SUCCESS;
    });
}

BlockchainResult consensus_get_next_difficulty_target(ConsensusEngine* engine, uint64_t height, uint32_t* target) {
    HANDLE_EXCEPTIONS({
        if (!engine || !target) {
            return BLOCKCHAIN_ERROR_INVALID_INPUT;
        }
        
        // Simple difficulty calculation
        *target = 0x1e00ffff; // Default difficulty
        
        return BLOCKCHAIN_SUCCESS;
    });
}

BlockchainResult consensus_get_block_reward(uint64_t height, uint64_t* reward) {
    HANDLE_EXCEPTIONS({
        if (!reward) {
            return BLOCKCHAIN_ERROR_INVALID_INPUT;
        }
        
        // Bitcoin-style halving every 210,000 blocks
        const uint64_t initial_reward = 50 * 100000000; // 50 BTC in satoshis
        const uint64_t halving_interval = 210000;
        
        uint64_t halvings = height / halving_interval;
        
        if (halvings >= 64) {
            // After 64 halvings, reward becomes 0
            *reward = 0;
        } else {
            *reward = initial_reward >> halvings; // Right shift is equivalent to division by 2^halvings
        }
        
        return BLOCKCHAIN_SUCCESS;
    });
}

// =============================================================================
// STORAGE ENGINE FFI IMPLEMENTATION  
// =============================================================================

StorageEngine* storage_engine_new(const char* database_path) {
    try {
        if (!database_path) {
            return nullptr;
        }
        return new StorageEngine(database_path);
    } catch (...) {
        return nullptr;
    }
}

void storage_engine_destroy(StorageEngine* engine) {
    delete engine;
}

BlockchainResult storage_has_block(StorageEngine* engine, const Hash256* block_hash, bool* exists) {
    HANDLE_EXCEPTIONS({
        if (!engine || !block_hash || !exists) {
            return BLOCKCHAIN_ERROR_INVALID_INPUT;
        }
        
        // Simple hash comparison - for basic functionality
        // TODO: Implement full storage integration
        *exists = false;
        
        return BLOCKCHAIN_SUCCESS;
    });
}

BlockchainResult storage_has_transaction(StorageEngine* engine, const Hash256* txid, bool* exists) {
    HANDLE_EXCEPTIONS({
        if (!engine || !txid || !exists) {
            return BLOCKCHAIN_ERROR_INVALID_INPUT;
        }
        
        // Simple transaction lookup - for basic functionality
        // TODO: Implement full storage integration
        *exists = false;
        
        return BLOCKCHAIN_SUCCESS;
    });
}

BlockchainResult storage_get_utxo_count(StorageEngine* engine, size_t* count) {
    HANDLE_EXCEPTIONS({
        if (!engine || !count) {
            return BLOCKCHAIN_ERROR_INVALID_INPUT;
        }
        
        // Return basic UTXO count - for basic functionality
        // TODO: Implement full storage integration
        *count = 0;
        
        return BLOCKCHAIN_SUCCESS;
    });
}

BlockchainResult storage_get_chain_tip(StorageEngine* engine, Hash256* tip_hash, uint64_t* tip_height) {
    HANDLE_EXCEPTIONS({
        if (!engine || !tip_hash || !tip_height) {
            return BLOCKCHAIN_ERROR_INVALID_INPUT;
        }
        
        // Return basic chain tip - for basic functionality
        // TODO: Implement full storage integration
        std::memset(tip_hash->data, 0, 32);
        *tip_height = 0;
    });
}

BlockchainResult storage_set_chain_tip(StorageEngine* engine, const Hash256* tip_hash, uint64_t tip_height) {
    HANDLE_EXCEPTIONS({
        if (!engine || !tip_hash) {
            return BLOCKCHAIN_ERROR_INVALID_INPUT;
        }
        
        // Set chain tip - for basic functionality
        // TODO: Implement full storage integration
        return BLOCKCHAIN_SUCCESS;
    });
}

BlockchainResult storage_store_block(StorageEngine* engine, const Block* block) {
    HANDLE_EXCEPTIONS({
        if (!engine || !block) {
            return BLOCKCHAIN_ERROR_INVALID_INPUT;
        }
        
        // Store block - placeholder implementation
        // TODO: Implement full storage integration
        return BLOCKCHAIN_SUCCESS;
    });
}

BlockchainResult storage_get_block_by_hash(StorageEngine* engine, const Hash256* block_hash, Block* block) {
    HANDLE_EXCEPTIONS({
        if (!engine || !block_hash || !block) {
            return BLOCKCHAIN_ERROR_INVALID_INPUT;
        }
        
        // Get block by hash - placeholder implementation
        // TODO: Implement full storage integration
        return BLOCKCHAIN_ERROR_STORAGE_ERROR;
    });
}

BlockchainResult storage_get_block_by_height(StorageEngine* engine, uint64_t height, Block* block) {
    HANDLE_EXCEPTIONS({
        if (!engine || !block) {
            return BLOCKCHAIN_ERROR_INVALID_INPUT;
        }
        
        // Get block by height - placeholder implementation
        // TODO: Implement full storage integration
        return BLOCKCHAIN_ERROR_STORAGE_ERROR;
    });
}

BlockchainResult storage_store_transaction(StorageEngine* engine, const Transaction* transaction) {
    HANDLE_EXCEPTIONS({
        if (!engine || !transaction) {
            return BLOCKCHAIN_ERROR_INVALID_INPUT;
        }
        
        // Store transaction - placeholder implementation
        // TODO: Implement full storage integration
        return BLOCKCHAIN_SUCCESS;
    });
}

BlockchainResult storage_get_transaction(StorageEngine* engine, const Hash256* txid, Transaction* transaction) {
    HANDLE_EXCEPTIONS({
        if (!engine || !txid || !transaction) {
            return BLOCKCHAIN_ERROR_INVALID_INPUT;
        }
        
        // Get transaction - placeholder implementation
        // TODO: Implement full storage integration
        return BLOCKCHAIN_ERROR_STORAGE_ERROR;
    });
}

BlockchainResult storage_add_utxo(StorageEngine* engine, const OutPoint* outpoint, const TransactionOutput* output) {
    HANDLE_EXCEPTIONS({
        if (!engine || !outpoint || !output) {
            return BLOCKCHAIN_ERROR_INVALID_INPUT;
        }
        
        // Add UTXO - placeholder implementation
        // TODO: Implement full storage integration
        return BLOCKCHAIN_SUCCESS;
    });
}

BlockchainResult storage_remove_utxo(StorageEngine* engine, const OutPoint* outpoint) {
    HANDLE_EXCEPTIONS({
        if (!engine || !outpoint) {
            return BLOCKCHAIN_ERROR_INVALID_INPUT;
        }
        
        // Remove UTXO - placeholder implementation
        // TODO: Implement full storage integration
        return BLOCKCHAIN_SUCCESS;
    });
}

BlockchainResult storage_get_utxo(StorageEngine* engine, const OutPoint* outpoint, TransactionOutput* output, bool* exists) {
    HANDLE_EXCEPTIONS({
        if (!engine || !outpoint || !output || !exists) {
            return BLOCKCHAIN_ERROR_INVALID_INPUT;
        }
        
        // Get UTXO - placeholder implementation
        // TODO: Implement full storage integration
        *exists = false;
        return BLOCKCHAIN_SUCCESS;
    });
}

// =============================================================================
// VM ENGINE FFI IMPLEMENTATION
// =============================================================================

VMEngine* vm_engine_new() {
    try {
        return new VMEngine();
    } catch (...) {
        return nullptr;
    }
}

void vm_engine_destroy(VMEngine* engine) {
    delete engine;
}

BlockchainResult vm_validate_script_syntax(const uint8_t* script, size_t script_len, bool* is_valid) {
    HANDLE_EXCEPTIONS({
        if (!script || !is_valid) {
            return BLOCKCHAIN_ERROR_INVALID_INPUT;
        }
        
        // Basic script validation - check for valid opcodes
        *is_valid = true; // Simplified validation
        
        for (size_t i = 0; i < script_len; ++i) {
            uint8_t opcode = script[i];
            
            // Check for invalid opcodes (this is very simplified)
            if (opcode > 0xFA) { // Above OP_INVALIDOPCODE
                *is_valid = false;
                break;
            }
        }
        
        return BLOCKCHAIN_SUCCESS;
    });
}

// =============================================================================
// UTILITY FUNCTIONS
// =============================================================================

ByteBuffer* byte_buffer_new(size_t capacity) {
    try {
        auto* buffer = new ByteBuffer();
        buffer->data = new uint8_t[capacity];
        buffer->size = 0;
        buffer->capacity = capacity;
        return buffer;
    } catch (...) {
        return nullptr;
    }
}

void byte_buffer_destroy(ByteBuffer* buffer) {
    if (buffer) {
        delete[] buffer->data;
        delete buffer;
    }
}

BlockchainResult byte_buffer_resize(ByteBuffer* buffer, size_t new_size) {
    HANDLE_EXCEPTIONS({
        if (!buffer) {
            return BLOCKCHAIN_ERROR_INVALID_INPUT;
        }
        
        if (new_size > buffer->capacity) {
            // Need to reallocate
            uint8_t* new_data = new uint8_t[new_size];
            if (buffer->data) {
                std::memcpy(new_data, buffer->data, buffer->size);
                delete[] buffer->data;
            }
            buffer->data = new_data;
            buffer->capacity = new_size;
        }
        
        buffer->size = new_size;
        return BLOCKCHAIN_SUCCESS;
    });
}

BlockchainResult byte_buffer_append(ByteBuffer* buffer, const uint8_t* data, size_t len) {
    HANDLE_EXCEPTIONS({
        if (!buffer || !data) {
            return BLOCKCHAIN_ERROR_INVALID_INPUT;
        }
        
        size_t new_length = buffer->size + len;
        if (new_length > buffer->capacity) {
            // Grow buffer by 1.5x or required size, whichever is larger
            size_t new_capacity = std::max(new_length, buffer->capacity + buffer->capacity / 2);
            
            uint8_t* new_data = new uint8_t[new_capacity];
            if (buffer->data) {
                std::memcpy(new_data, buffer->data, buffer->size);
                delete[] buffer->data;
            }
            buffer->data = new_data;
            buffer->capacity = new_capacity;
        }
        
        std::memcpy(buffer->data + buffer->size, data, len);
        buffer->size = new_length;
        
        return BLOCKCHAIN_SUCCESS;
    });
}

BlockchainResult hex_encode(const uint8_t* data, size_t len, char* output, size_t output_size) {
    HANDLE_EXCEPTIONS({
        if (!data || !output || output_size < (len * 2 + 1)) {
            return BLOCKCHAIN_ERROR_INVALID_INPUT;
        }
        
        std::vector<uint8_t> input_data(data, data + len);
        std::string hex = blockchain::crypto::utils::to_hex(input_data);
        
        std::strncpy(output, hex.c_str(), output_size - 1);
        output[output_size - 1] = '\0';
        
        return BLOCKCHAIN_SUCCESS;
    });
}

BlockchainResult serialize_transaction(const Transaction* transaction, ByteBuffer* output) {
    HANDLE_EXCEPTIONS({
        if (!transaction || !output) {
            return BLOCKCHAIN_ERROR_INVALID_INPUT;
        }
        
        // Serialize transaction - placeholder implementation
        // TODO: Implement proper serialization
        output->size = 0;
        return BLOCKCHAIN_SUCCESS;
    });
}

BlockchainResult deserialize_transaction(const uint8_t* data, size_t len, Transaction* transaction) {
    HANDLE_EXCEPTIONS({
        if (!data || !transaction || len == 0) {
            return BLOCKCHAIN_ERROR_INVALID_INPUT;
        }
        
        // Deserialize transaction - placeholder implementation
        // TODO: Implement proper deserialization
        return BLOCKCHAIN_SUCCESS;
    });
}

BlockchainResult serialize_block(const Block* block, ByteBuffer* output) {
    HANDLE_EXCEPTIONS({
        if (!block || !output) {
            return BLOCKCHAIN_ERROR_INVALID_INPUT;
        }
        
        // Serialize block - placeholder implementation
        // TODO: Implement proper serialization
        output->size = 0;
        return BLOCKCHAIN_SUCCESS;
    });
}

BlockchainResult deserialize_block(const uint8_t* data, size_t len, Block* block) {
    HANDLE_EXCEPTIONS({
        if (!data || !block || len == 0) {
            return BLOCKCHAIN_ERROR_INVALID_INPUT;
        }
        
        // Deserialize block - placeholder implementation
        // TODO: Implement proper deserialization
        return BLOCKCHAIN_SUCCESS;
    });
}

BlockchainResult hex_decode(const char* hex_string, uint8_t* output, size_t* output_len) {
    try {
        if (!hex_string || !output || !output_len) {
            return BLOCKCHAIN_ERROR_INVALID_INPUT;
        }
        
        // Simple hex decode implementation
        size_t hex_len = strlen(hex_string);
        if (hex_len % 2 != 0) {
            return BLOCKCHAIN_ERROR_INVALID_INPUT;
        }
        
        size_t decode_len = hex_len / 2;
        if (decode_len > *output_len) {
            return BLOCKCHAIN_ERROR_BUFFER_TOO_SMALL;
        }
        
        for (size_t i = 0; i < decode_len; ++i) {
            char byte_str[3] = {hex_string[i*2], hex_string[i*2+1], '\0'};
            output[i] = (uint8_t)strtol(byte_str, nullptr, 16);
        }
        
        *output_len = decode_len;
        return BLOCKCHAIN_SUCCESS;
    } catch (...) {
        return BLOCKCHAIN_ERROR_UNKNOWN;
    }
}

} // extern "C"