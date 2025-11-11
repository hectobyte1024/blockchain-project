#pragma once

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// Forward declarations for opaque types
typedef struct CryptoEngine CryptoEngine;
typedef struct ConsensusEngine ConsensusEngine;
typedef struct StorageEngine StorageEngine;
typedef struct VMEngine VMEngine;

// Result codes for cross-language error handling
typedef enum {
    BLOCKCHAIN_SUCCESS = 0,
    BLOCKCHAIN_ERROR_INVALID_INPUT = 1,
    BLOCKCHAIN_ERROR_INVALID_TRANSACTION = 2,
    BLOCKCHAIN_ERROR_INVALID_BLOCK = 3,
    BLOCKCHAIN_ERROR_INVALID_SIGNATURE = 4,
    BLOCKCHAIN_ERROR_STORAGE_ERROR = 5,
    BLOCKCHAIN_ERROR_CONSENSUS_ERROR = 6,
    BLOCKCHAIN_ERROR_VM_ERROR = 7,
    BLOCKCHAIN_ERROR_OUT_OF_MEMORY = 8,
    BLOCKCHAIN_ERROR_INVALID_PARAMETER = 9,
    BLOCKCHAIN_ERROR_BUFFER_TOO_SMALL = 10,
    BLOCKCHAIN_ERROR_UNKNOWN = 99,
} BlockchainResult;

// Shared data structures for zero-copy exchange
typedef struct {
    uint8_t data[32];
} Hash256;

typedef struct {
    uint8_t data[20];
} Hash160;

typedef struct {
    uint8_t data[32];
} PrivateKey;

typedef struct {
    uint8_t data[33];
} PublicKey;

typedef struct {
    uint8_t data[64];
} Signature;

typedef struct {
    uint8_t* data;
    size_t size;
    size_t capacity;
} ByteBuffer;

typedef struct {
    Hash256 txid;
    uint32_t vout;
} OutPoint;

typedef struct {
    OutPoint previous_output;
    ByteBuffer script_sig;
    uint32_t sequence;
} TransactionInput;

typedef struct {
    uint64_t value;
    ByteBuffer script_pubkey;
} TransactionOutput;

typedef struct {
    uint32_t version;
    TransactionInput* inputs;
    size_t input_count;
    TransactionOutput* outputs;
    size_t output_count;
    uint32_t lock_time;
} Transaction;

typedef struct {
    uint32_t version;
    Hash256 previous_block_hash;
    Hash256 merkle_root;
    uint64_t timestamp;
    uint32_t difficulty_target;
    uint64_t nonce;
} BlockHeader;

typedef struct {
    BlockHeader header;
    Transaction* transactions;
    size_t transaction_count;
} Block;

// =============================================================================
// CRYPTO ENGINE FFI
// =============================================================================

// Crypto engine lifecycle
CryptoEngine* crypto_engine_new(void);
void crypto_engine_destroy(CryptoEngine* engine);

// Hash functions
BlockchainResult crypto_sha256(const uint8_t* input, size_t input_len, Hash256* output);
BlockchainResult crypto_double_sha256(const uint8_t* input, size_t input_len, Hash256* output);
BlockchainResult crypto_ripemd160(const uint8_t* input, size_t input_len, Hash160* output);

// Key generation and management
BlockchainResult crypto_generate_private_key(PrivateKey* private_key);
BlockchainResult crypto_derive_public_key(const PrivateKey* private_key, PublicKey* public_key);
bool crypto_is_valid_private_key(const PrivateKey* private_key);
bool crypto_is_valid_public_key(const PublicKey* public_key);

// Digital signatures  
BlockchainResult crypto_sign_message(const PrivateKey* private_key, const Hash256* message_hash, Signature* signature);
BlockchainResult crypto_verify_signature(const PublicKey* public_key, const Hash256* message_hash, const Signature* signature, bool* is_valid);

// Merkle tree operations
BlockchainResult crypto_calculate_merkle_root(const Hash256* leaf_hashes, size_t leaf_count, Hash256* root);
BlockchainResult crypto_verify_merkle_proof(const Hash256* leaf_hash, const Hash256* proof, size_t proof_length, 
                                          const Hash256* root, size_t leaf_index, size_t tree_size, bool* is_valid);

// =============================================================================
// CONSENSUS ENGINE FFI  
// =============================================================================

// Consensus engine lifecycle
ConsensusEngine* consensus_engine_new(void);
void consensus_engine_destroy(ConsensusEngine* engine);

// Block validation
BlockchainResult consensus_validate_block_header(ConsensusEngine* engine, const BlockHeader* header, bool* is_valid);
BlockchainResult consensus_validate_transaction(ConsensusEngine* engine, const Transaction* transaction, bool* is_valid);
BlockchainResult consensus_validate_block(ConsensusEngine* engine, const Block* block, bool* is_valid);

// Proof of Work
BlockchainResult consensus_check_proof_of_work(const BlockHeader* header, bool* meets_target);
BlockchainResult consensus_calculate_difficulty_adjustment(ConsensusEngine* engine, uint64_t current_height, 
                                                         uint64_t current_timestamp, uint32_t* new_target);

// Chain operations
BlockchainResult consensus_get_block_reward(uint64_t height, uint64_t* reward);
BlockchainResult consensus_get_next_difficulty_target(ConsensusEngine* engine, uint64_t height, uint32_t* target);

// =============================================================================
// STORAGE ENGINE FFI
// =============================================================================

// Storage engine lifecycle
StorageEngine* storage_engine_new(const char* database_path);
void storage_engine_destroy(StorageEngine* engine);

// Block storage
BlockchainResult storage_store_block(StorageEngine* engine, const Block* block);
BlockchainResult storage_get_block_by_hash(StorageEngine* engine, const Hash256* block_hash, Block* block);
BlockchainResult storage_get_block_by_height(StorageEngine* engine, uint64_t height, Block* block);
BlockchainResult storage_has_block(StorageEngine* engine, const Hash256* block_hash, bool* exists);

// Transaction storage  
BlockchainResult storage_store_transaction(StorageEngine* engine, const Transaction* transaction);
BlockchainResult storage_get_transaction(StorageEngine* engine, const Hash256* txid, Transaction* transaction);
BlockchainResult storage_has_transaction(StorageEngine* engine, const Hash256* txid, bool* exists);

// UTXO management
BlockchainResult storage_add_utxo(StorageEngine* engine, const OutPoint* outpoint, const TransactionOutput* output);
BlockchainResult storage_remove_utxo(StorageEngine* engine, const OutPoint* outpoint);
BlockchainResult storage_get_utxo(StorageEngine* engine, const OutPoint* outpoint, TransactionOutput* output, bool* exists);
BlockchainResult storage_get_utxo_count(StorageEngine* engine, size_t* count);

// Chain state
BlockchainResult storage_get_chain_tip(StorageEngine* engine, Hash256* tip_hash, uint64_t* tip_height);
BlockchainResult storage_set_chain_tip(StorageEngine* engine, const Hash256* tip_hash, uint64_t tip_height);

// =============================================================================
// VM ENGINE FFI
// =============================================================================

// VM engine lifecycle  
VMEngine* vm_engine_new(void);
void vm_engine_destroy(VMEngine* engine);

// Script execution
BlockchainResult vm_execute_script(VMEngine* engine, const uint8_t* script, size_t script_len,
                                 const Transaction* transaction, size_t input_index, bool* result);

// Script validation
BlockchainResult vm_validate_script_syntax(const uint8_t* script, size_t script_len, bool* is_valid);
BlockchainResult vm_calculate_script_hash(const uint8_t* script, size_t script_len, Hash256* script_hash);

// Standard script templates
BlockchainResult vm_create_p2pkh_script(const Hash160* pubkey_hash, ByteBuffer* script);
BlockchainResult vm_create_p2sh_script(const Hash256* script_hash, ByteBuffer* script);
BlockchainResult vm_create_multisig_script(const PublicKey* pubkeys, size_t pubkey_count, 
                                         size_t required_sigs, ByteBuffer* script);

// =============================================================================
// UTILITY FUNCTIONS
// =============================================================================

// Memory management for ByteBuffer
ByteBuffer* byte_buffer_new(size_t capacity);
void byte_buffer_destroy(ByteBuffer* buffer);
BlockchainResult byte_buffer_resize(ByteBuffer* buffer, size_t new_size);
BlockchainResult byte_buffer_append(ByteBuffer* buffer, const uint8_t* data, size_t len);

// Serialization helpers
BlockchainResult serialize_transaction(const Transaction* transaction, ByteBuffer* output);
BlockchainResult deserialize_transaction(const uint8_t* data, size_t len, Transaction* transaction);
BlockchainResult serialize_block(const Block* block, ByteBuffer* output);
BlockchainResult deserialize_block(const uint8_t* data, size_t len, Block* block);

// Hex encoding/decoding
BlockchainResult hex_encode(const uint8_t* data, size_t len, char* output, size_t output_size);
BlockchainResult hex_decode(const char* hex_string, uint8_t* output, size_t* output_len);

#ifdef __cplusplus
}
#endif