#pragma once

#include <vector>
#include <stack>
#include <memory>
#include <unordered_map>
#include <cstdint>

#include "core.hpp"
#include "transaction.hpp"
#include "crypto.hpp"

// Forward declarations and type aliases
namespace blockchain {
    using Hash160 = crypto::Hash160;
    using Hash256 = crypto::Hash256;
    using PublicKey = crypto::PublicKey;
}

namespace blockchain {

// Script opcodes based on Bitcoin script system
enum class Opcode : uint8_t {
    // Constants
    OP_0 = 0x00,
    OP_FALSE = 0x00,
    OP_PUSHDATA1 = 0x4c,
    OP_PUSHDATA2 = 0x4d,
    OP_PUSHDATA4 = 0x4e,
    OP_1NEGATE = 0x4f,
    OP_1 = 0x51,
    OP_TRUE = 0x51,
    OP_2 = 0x52,
    OP_3 = 0x53,
    OP_4 = 0x54,
    OP_5 = 0x55,
    OP_6 = 0x56,
    OP_7 = 0x57,
    OP_8 = 0x58,
    OP_9 = 0x59,
    OP_10 = 0x5a,
    OP_11 = 0x5b,
    OP_12 = 0x5c,
    OP_13 = 0x5d,
    OP_14 = 0x5e,
    OP_15 = 0x5f,
    OP_16 = 0x60,

    // Control flow
    OP_NOP = 0x61,
    OP_IF = 0x63,
    OP_NOTIF = 0x64,
    OP_ELSE = 0x67,
    OP_ENDIF = 0x68,
    OP_VERIFY = 0x69,
    OP_RETURN = 0x6a,

    // Stack operations
    OP_TOALTSTACK = 0x6b,
    OP_FROMALTSTACK = 0x6c,
    OP_IFDUP = 0x73,
    OP_DEPTH = 0x74,
    OP_DROP = 0x75,
    OP_DUP = 0x76,
    OP_NIP = 0x77,
    OP_OVER = 0x78,
    OP_PICK = 0x79,
    OP_ROLL = 0x7a,
    OP_ROT = 0x7b,
    OP_SWAP = 0x7c,
    OP_TUCK = 0x7d,
    OP_2DROP = 0x6d,
    OP_2DUP = 0x6e,
    OP_3DUP = 0x6f,
    OP_2OVER = 0x70,
    OP_2ROT = 0x71,
    OP_2SWAP = 0x72,

    // Arithmetic
    OP_1ADD = 0x8b,
    OP_1SUB = 0x8c,
    OP_NEGATE = 0x8f,
    OP_ABS = 0x90,
    OP_NOT = 0x91,
    OP_0NOTEQUAL = 0x92,
    OP_ADD = 0x93,
    OP_SUB = 0x94,
    OP_MUL = 0x95,
    OP_DIV = 0x96,
    OP_MOD = 0x97,
    OP_LSHIFT = 0x98,
    OP_RSHIFT = 0x99,
    OP_BOOLAND = 0x9a,
    OP_BOOLOR = 0x9b,
    OP_NUMEQUAL = 0x9c,
    OP_NUMEQUALVERIFY = 0x9d,
    OP_NUMNOTEQUAL = 0x9e,
    OP_LESSTHAN = 0x9f,
    OP_GREATERTHAN = 0xa0,
    OP_LESSTHANOREQUAL = 0xa1,
    OP_GREATERTHANOREQUAL = 0xa2,
    OP_MIN = 0xa3,
    OP_MAX = 0xa4,
    OP_WITHIN = 0xa5,

    // Crypto
    OP_RIPEMD160 = 0xa6,
    OP_SHA1 = 0xa7,
    OP_SHA256 = 0xa8,
    OP_HASH160 = 0xa9,
    OP_HASH256 = 0xaa,
    OP_CODESEPARATOR = 0xab,
    OP_CHECKSIG = 0xac,
    OP_CHECKSIGVERIFY = 0xad,
    OP_CHECKMULTISIG = 0xae,
    OP_CHECKMULTISIGVERIFY = 0xaf,

    // Educational blockchain specific opcodes
    OP_EDU_PRINT = 0xf0,
    OP_EDU_LOG = 0xf1,
    OP_EDU_TIMESTAMP = 0xf2,
    OP_EDU_BLOCKHASH = 0xf3,
    OP_EDU_TXHASH = 0xf4,

    // Invalid opcode
    OP_INVALIDOPCODE = 0xff,
};

// Forward declare Transaction to avoid circular dependency
namespace transaction {
    class Transaction;
}
using Transaction = transaction::Transaction;

// Script execution context
struct ExecutionContext {
    std::stack<std::vector<uint8_t>> main_stack;
    std::stack<std::vector<uint8_t>> alt_stack;
    const transaction::Transaction* transaction;
    size_t input_index;
    uint64_t gas_used;
    uint64_t gas_limit;
    bool debug_mode;
    std::vector<std::string> debug_log;
};

// VM execution result
struct VMResult {
    bool success;
    std::string error_message;
    uint64_t gas_used;
    std::vector<std::string> debug_log;
};

// Virtual Machine Engine
class VMEngine {
public:
    VMEngine();
    ~VMEngine() = default;

    // Script execution
    VMResult execute_script(
        const std::vector<uint8_t>& script,
        const transaction::Transaction& transaction,
        size_t input_index,
        uint64_t gas_limit = 1000000
    );

    // Script validation
    bool validate_script_syntax(const std::vector<uint8_t>& script);
    crypto::Hash256 calculate_script_hash(const std::vector<uint8_t>& script);

    // Standard script templates
    std::vector<uint8_t> create_p2pkh_script(const crypto::Hash160& pubkey_hash);
    std::vector<uint8_t> create_p2sh_script(const crypto::Hash256& script_hash);
    std::vector<uint8_t> create_multisig_script(
        const std::vector<crypto::PublicKey>& pubkeys,
        size_t required_sigs
    );

    // Script analysis
    std::vector<Opcode> parse_script(const std::vector<uint8_t>& script);
    bool is_standard_script(const std::vector<uint8_t>& script);

    // Educational features
    void set_debug_mode(bool enabled) { debug_mode_ = enabled; }
    std::vector<std::string> get_debug_log() const { return debug_log_; }

private:
    // Core execution functions
    bool execute_opcode(Opcode opcode, ExecutionContext& ctx);
    bool execute_push_data(const uint8_t* data, size_t len, ExecutionContext& ctx);
    
    // Stack operations
    bool op_dup(ExecutionContext& ctx);
    bool op_drop(ExecutionContext& ctx);
    bool op_swap(ExecutionContext& ctx);
    bool op_over(ExecutionContext& ctx);
    bool op_pick(ExecutionContext& ctx);
    bool op_roll(ExecutionContext& ctx);
    bool op_rot(ExecutionContext& ctx);
    
    // Arithmetic operations
    bool op_add(ExecutionContext& ctx);
    bool op_sub(ExecutionContext& ctx);
    bool op_mul(ExecutionContext& ctx);
    bool op_div(ExecutionContext& ctx);
    bool op_mod(ExecutionContext& ctx);
    bool op_equal(ExecutionContext& ctx);
    bool op_lessthan(ExecutionContext& ctx);
    bool op_greaterthan(ExecutionContext& ctx);
    
    // Cryptographic operations
    bool op_hash160(ExecutionContext& ctx);
    bool op_hash256(ExecutionContext& ctx);
    bool op_checksig(ExecutionContext& ctx);
    bool op_checkmultisig(ExecutionContext& ctx);
    
    // Educational operations
    bool op_edu_print(ExecutionContext& ctx);
    bool op_edu_log(ExecutionContext& ctx);
    bool op_edu_timestamp(ExecutionContext& ctx);
    
    // Control flow
    bool op_if(ExecutionContext& ctx);
    bool op_else(ExecutionContext& ctx);
    bool op_endif(ExecutionContext& ctx);
    bool op_verify(ExecutionContext& ctx);
    
    // Helper functions
    std::vector<uint8_t> pop_stack(ExecutionContext& ctx);
    void push_stack(const std::vector<uint8_t>& data, ExecutionContext& ctx);
    bool cast_to_bool(const std::vector<uint8_t>& data);
    int64_t cast_to_number(const std::vector<uint8_t>& data);
    
public:
    // Public helper for ScriptBuilder
    std::vector<uint8_t> number_to_bytes(int64_t number);
    
private:
    
    // Gas calculation
    uint64_t calculate_opcode_gas(Opcode opcode);
    bool consume_gas(ExecutionContext& ctx, uint64_t gas);
    
    // Script validation helpers
    bool is_valid_opcode(uint8_t opcode);
    bool validate_push_data(const uint8_t* script, size_t script_len, size_t& pos);

private:
    bool debug_mode_;
    std::vector<std::string> debug_log_;
};

// Script builder utility class
class ScriptBuilder {
public:
    ScriptBuilder& add_opcode(Opcode opcode);
    ScriptBuilder& add_data(const std::vector<uint8_t>& data);
    ScriptBuilder& add_number(int64_t number);
    ScriptBuilder& add_hash160(const crypto::Hash160& hash);
    ScriptBuilder& add_hash256(const crypto::Hash256& hash);
    ScriptBuilder& add_pubkey(const crypto::PublicKey& pubkey);
    
    std::vector<uint8_t> build();
    void clear();

private:
    std::vector<uint8_t> script_;
};

} // namespace blockchain