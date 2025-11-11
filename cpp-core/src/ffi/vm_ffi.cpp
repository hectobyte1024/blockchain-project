#include "ffi/blockchain_ffi.h"
#include "blockchain/vm.hpp"
#include <memory>

using namespace blockchain;

extern "C" {

// VM engine lifecycle
::VMEngine* vm_engine_new(void) {
    try {
        return reinterpret_cast<::VMEngine*>(new blockchain::VMEngine());
    } catch (...) {
        return nullptr;
    }
}

void vm_engine_destroy(::VMEngine* engine) {
    delete reinterpret_cast<blockchain::VMEngine*>(engine);
}

// Script execution
BlockchainResult vm_execute_script(
    ::VMEngine* engine,
    const uint8_t* script,
    size_t script_len,
    const ::Transaction* transaction,
    size_t input_index,
    bool* result
) {
    if (!engine || !script || !transaction || !result) {
        return BLOCKCHAIN_ERROR_INVALID_PARAMETER;
    }

    try {
        auto* vm_engine = reinterpret_cast<blockchain::VMEngine*>(engine);
        std::vector<uint8_t> script_vec(script, script + script_len);
        
        // Convert C transaction to C++ transaction
        blockchain::transaction::Transaction cpp_tx;
        // For demo purposes, create a simple valid result
        *result = true;
        
        return BLOCKCHAIN_SUCCESS;
    } catch (const std::exception& e) {
        return BLOCKCHAIN_ERROR_VM_ERROR;
    } catch (...) {
        return BLOCKCHAIN_ERROR_UNKNOWN;
    }
}

// Script validation
BlockchainResult vm_validate_script_syntax(
    const uint8_t* script,
    size_t script_len,
    bool* is_valid
) {
    if (!script || !is_valid) {
        return BLOCKCHAIN_ERROR_INVALID_PARAMETER;
    }

    try {
        blockchain::VMEngine engine;
        std::vector<uint8_t> script_vec(script, script + script_len);
        *is_valid = script_len > 0; // Simple validation for demo
        return BLOCKCHAIN_SUCCESS;
    } catch (const std::exception& e) {
        return BLOCKCHAIN_ERROR_VM_ERROR;
    } catch (...) {
        return BLOCKCHAIN_ERROR_UNKNOWN;
    }
}

BlockchainResult vm_calculate_script_hash(
    const uint8_t* script,
    size_t script_len,
    ::Hash256* script_hash
) {
    if (!script || !script_hash) {
        return BLOCKCHAIN_ERROR_INVALID_PARAMETER;
    }

    try {
        blockchain::VMEngine engine;
        std::vector<uint8_t> script_vec(script, script + script_len);
        
        // Calculate simple hash for demo
        crypto::Hash256 hash = crypto::SHA256::hash(script, script_len);
        
        // Copy hash data to C structure
        std::copy(hash.begin(), hash.end(), script_hash->data);
        return BLOCKCHAIN_SUCCESS;
    } catch (const std::exception& e) {
        return BLOCKCHAIN_ERROR_VM_ERROR;
    } catch (...) {
        return BLOCKCHAIN_ERROR_UNKNOWN;
    }
}

// Standard script templates
BlockchainResult vm_create_p2pkh_script(
    const ::Hash160* pubkey_hash,
    ::ByteBuffer* script
) {
    if (!pubkey_hash || !script) {
        return BLOCKCHAIN_ERROR_INVALID_PARAMETER;
    }

    try {
        blockchain::VMEngine engine;
        
        // Convert C hash160 to C++ hash160
        crypto::Hash160 cpp_hash;
        std::copy(pubkey_hash->data, pubkey_hash->data + 20, cpp_hash.begin());
        
        // Create P2PKH script (simple template for demo)
        std::vector<uint8_t> script_bytes = {
            0x76, 0xA9, 0x14  // OP_DUP OP_HASH160 <20 bytes>
        };
        script_bytes.insert(script_bytes.end(), cpp_hash.begin(), cpp_hash.end());
        script_bytes.push_back(0x88); // OP_EQUALVERIFY
        script_bytes.push_back(0xAC); // OP_CHECKSIG
        
        // Ensure buffer has enough capacity
        if (script->capacity < script_bytes.size()) {
            return BLOCKCHAIN_ERROR_BUFFER_TOO_SMALL;
        }
        
        std::copy(script_bytes.begin(), script_bytes.end(), script->data);
        script->size = script_bytes.size();
        
        return BLOCKCHAIN_SUCCESS;
    } catch (const std::exception& e) {
        return BLOCKCHAIN_ERROR_VM_ERROR;
    } catch (...) {
        return BLOCKCHAIN_ERROR_UNKNOWN;
    }
}

BlockchainResult vm_create_p2sh_script(
    const ::Hash256* script_hash,
    ::ByteBuffer* script
) {
    if (!script_hash || !script) {
        return BLOCKCHAIN_ERROR_INVALID_PARAMETER;
    }

    try {
        blockchain::VMEngine engine;
        
        // Convert C hash256 to C++ hash256
        crypto::Hash256 cpp_hash;
        std::copy(script_hash->data, script_hash->data + 32, cpp_hash.begin());
        
        // Create P2SH script (simple template for demo)
        std::vector<uint8_t> script_bytes = {
            0xA9, 0x14  // OP_HASH160 <20 bytes>
        };
        
        // Use first 20 bytes of hash for P2SH (normally this would be RIPEMD160)
        script_bytes.insert(script_bytes.end(), cpp_hash.begin(), cpp_hash.begin() + 20);
        script_bytes.push_back(0x87); // OP_EQUAL
        
        // Ensure buffer has enough capacity
        if (script->capacity < script_bytes.size()) {
            return BLOCKCHAIN_ERROR_BUFFER_TOO_SMALL;
        }
        
        std::copy(script_bytes.begin(), script_bytes.end(), script->data);
        script->size = script_bytes.size();
        
        return BLOCKCHAIN_SUCCESS;
    } catch (const std::exception& e) {
        return BLOCKCHAIN_ERROR_VM_ERROR;
    } catch (...) {
        return BLOCKCHAIN_ERROR_UNKNOWN;
    }
}

BlockchainResult vm_create_multisig_script(
    const ::PublicKey* pubkeys,
    size_t pubkey_count,
    size_t required_sigs,
    ::ByteBuffer* script
) {
    if (!pubkeys || !script || pubkey_count == 0 || required_sigs == 0 || required_sigs > pubkey_count) {
        return BLOCKCHAIN_ERROR_INVALID_PARAMETER;
    }

    try {
        blockchain::VMEngine engine;
        
        // Convert C pubkeys to C++ pubkeys
        std::vector<crypto::PublicKey> cpp_pubkeys;
        for (size_t i = 0; i < pubkey_count; ++i) {
            crypto::PublicKey cpp_pubkey;
            std::copy(pubkeys[i].data, pubkeys[i].data + 33, cpp_pubkey.begin());
            cpp_pubkeys.push_back(cpp_pubkey);
        }
        
        // Create multisig script (simple template for demo)
        std::vector<uint8_t> script_bytes;
        
        // Push required signatures count
        script_bytes.push_back(0x50 + required_sigs); // OP_1 to OP_16
        
        // Push public keys
        for (const auto& pubkey : cpp_pubkeys) {
            script_bytes.push_back(33); // Push 33 bytes
            script_bytes.insert(script_bytes.end(), pubkey.begin(), pubkey.end());
        }
        
        // Push total pubkey count and OP_CHECKMULTISIG
        script_bytes.push_back(0x50 + pubkey_count); // OP_1 to OP_16
        script_bytes.push_back(0xAE); // OP_CHECKMULTISIG
        
        if (script_bytes.empty()) {
            return BLOCKCHAIN_ERROR_VM_ERROR;
        }
        
        // Ensure buffer has enough capacity
        if (script->capacity < script_bytes.size()) {
            return BLOCKCHAIN_ERROR_BUFFER_TOO_SMALL;
        }
        
        std::copy(script_bytes.begin(), script_bytes.end(), script->data);
        script->size = script_bytes.size();
        
        return BLOCKCHAIN_SUCCESS;
    } catch (const std::exception& e) {
        return BLOCKCHAIN_ERROR_VM_ERROR;
    } catch (...) {
        return BLOCKCHAIN_ERROR_UNKNOWN;
    }
}

} // extern "C"