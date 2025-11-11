#include "blockchain/vm.hpp"
#include "blockchain/crypto.hpp"
#include "blockchain/transaction.hpp"
#include <algorithm>
#include <sstream>
#include <iomanip>
#include <chrono>

namespace blockchain {

VMEngine::VMEngine() : debug_mode_(false) {}

VMResult VMEngine::execute_script(
    const std::vector<uint8_t>& script,
    const transaction::Transaction& transaction,
    size_t input_index,
    uint64_t gas_limit
) {
    ExecutionContext ctx;
    ctx.transaction = &transaction;
    ctx.input_index = input_index;
    ctx.gas_used = 0;
    ctx.gas_limit = gas_limit;
    ctx.debug_mode = debug_mode_;
    
    VMResult result;
    result.success = false;
    result.gas_used = 0;
    
    // Validate input parameters
    if (input_index >= transaction.inputs.size()) {
        result.error_message = "Invalid input index";
        return result;
    }
    
    size_t pc = 0; // Program counter
    
    try {
        while (pc < script.size()) {
            uint8_t opcode_byte = script[pc];
            pc++;
            
            // Check for push data operations (1-75 bytes)
            if (opcode_byte >= 1 && opcode_byte <= 75) {
                if (pc + opcode_byte > script.size()) {
                    result.error_message = "Script ends unexpectedly during push data";
                    return result;
                }
                
                if (!execute_push_data(&script[pc], opcode_byte, ctx)) {
                    result.error_message = "Failed to push data to stack";
                    return result;
                }
                
                pc += opcode_byte;
                continue;
            }
            
            // Execute opcode
            Opcode opcode = static_cast<Opcode>(opcode_byte);
            
            // Consume gas for this operation
            uint64_t gas_cost = calculate_opcode_gas(opcode);
            if (!consume_gas(ctx, gas_cost)) {
                result.error_message = "Out of gas";
                result.gas_used = ctx.gas_used;
                return result;
            }
            
            if (!execute_opcode(opcode, ctx)) {
                result.error_message = "Failed to execute opcode: " + std::to_string(static_cast<int>(opcode));
                result.gas_used = ctx.gas_used;
                return result;
            }
        }
        
        // Script execution successful if stack has at least one element and top is true
        if (ctx.main_stack.empty()) {
            result.error_message = "Stack is empty after execution";
        } else {
            std::vector<uint8_t> top = ctx.main_stack.top();
            result.success = cast_to_bool(top);
            if (!result.success) {
                result.error_message = "Script returned false";
            }
        }
        
    } catch (const std::exception& e) {
        result.error_message = "Runtime error: " + std::string(e.what());
    }
    
    result.gas_used = ctx.gas_used;
    result.debug_log = ctx.debug_log;
    debug_log_ = ctx.debug_log;
    
    return result;
}

bool VMEngine::execute_opcode(Opcode opcode, ExecutionContext& ctx) {
    switch (opcode) {
        // Constants
        case Opcode::OP_0:
        // case Opcode::OP_FALSE: // Same as OP_0, handled above
            push_stack(std::vector<uint8_t>(), ctx);
            return true;
            
        case Opcode::OP_1:
        // case Opcode::OP_TRUE: // Same as OP_1, handled above
            push_stack({0x01}, ctx);
            return true;
            
        case Opcode::OP_2:
            push_stack({0x02}, ctx);
            return true;
            
        case Opcode::OP_3:
            push_stack({0x03}, ctx);
            return true;
            
        // Stack operations
        case Opcode::OP_DUP:
            return op_dup(ctx);
            
        case Opcode::OP_DROP:
            return op_drop(ctx);
            
        case Opcode::OP_SWAP:
            return op_swap(ctx);
            
        case Opcode::OP_OVER:
            return op_over(ctx);
            
        case Opcode::OP_PICK:
            return op_pick(ctx);
            
        case Opcode::OP_ROLL:
            return op_roll(ctx);
            
        case Opcode::OP_ROT:
            return op_rot(ctx);
            
        // Arithmetic operations
        case Opcode::OP_ADD:
            return op_add(ctx);
            
        case Opcode::OP_SUB:
            return op_sub(ctx);
            
        case Opcode::OP_MUL:
            return op_mul(ctx);
            
        case Opcode::OP_DIV:
            return op_div(ctx);
            
        case Opcode::OP_MOD:
            return op_mod(ctx);
            
        case Opcode::OP_NUMEQUAL:
            return op_equal(ctx);
            
        case Opcode::OP_LESSTHAN:
            return op_lessthan(ctx);
            
        case Opcode::OP_GREATERTHAN:
            return op_greaterthan(ctx);
            
        // Cryptographic operations
        case Opcode::OP_HASH160:
            return op_hash160(ctx);
            
        case Opcode::OP_HASH256:
            return op_hash256(ctx);
            
        case Opcode::OP_CHECKSIG:
            return op_checksig(ctx);
            
        case Opcode::OP_CHECKMULTISIG:
            return op_checkmultisig(ctx);
            
        // Control flow
        case Opcode::OP_VERIFY:
            return op_verify(ctx);
            
        case Opcode::OP_RETURN:
            return false; // OP_RETURN always fails
            
        // Educational opcodes
        case Opcode::OP_EDU_PRINT:
            return op_edu_print(ctx);
            
        case Opcode::OP_EDU_LOG:
            return op_edu_log(ctx);
            
        case Opcode::OP_EDU_TIMESTAMP:
            return op_edu_timestamp(ctx);
            
        case Opcode::OP_NOP:
            return true; // No operation
            
        default:
            return false; // Unknown opcode
    }
}

bool VMEngine::execute_push_data(const uint8_t* data, size_t len, ExecutionContext& ctx) {
    std::vector<uint8_t> push_data(data, data + len);
    push_stack(push_data, ctx);
    return true;
}

// Stack operations implementation
bool VMEngine::op_dup(ExecutionContext& ctx) {
    if (ctx.main_stack.empty()) return false;
    
    std::vector<uint8_t> top = ctx.main_stack.top();
    ctx.main_stack.push(top);
    return true;
}

bool VMEngine::op_drop(ExecutionContext& ctx) {
    if (ctx.main_stack.empty()) return false;
    
    ctx.main_stack.pop();
    return true;
}

bool VMEngine::op_swap(ExecutionContext& ctx) {
    if (ctx.main_stack.size() < 2) return false;
    
    std::vector<uint8_t> a = pop_stack(ctx);
    std::vector<uint8_t> b = pop_stack(ctx);
    
    push_stack(a, ctx);
    push_stack(b, ctx);
    return true;
}

bool VMEngine::op_over(ExecutionContext& ctx) {
    if (ctx.main_stack.size() < 2) return false;
    
    std::vector<uint8_t> a = pop_stack(ctx);
    std::vector<uint8_t> b = pop_stack(ctx);
    
    push_stack(b, ctx);
    push_stack(a, ctx);
    push_stack(b, ctx);
    return true;
}

bool VMEngine::op_pick(ExecutionContext& ctx) {
    if (ctx.main_stack.empty()) return false;
    
    std::vector<uint8_t> n_bytes = pop_stack(ctx);
    int64_t n = cast_to_number(n_bytes);
    
    if (n < 0 || static_cast<size_t>(n) >= ctx.main_stack.size()) return false;
    
    // Get the nth item from the stack (0 = top)
    std::stack<std::vector<uint8_t>> temp_stack;
    for (int64_t i = 0; i <= n; ++i) {
        temp_stack.push(ctx.main_stack.top());
        ctx.main_stack.pop();
    }
    
    std::vector<uint8_t> picked = temp_stack.top();
    
    // Restore stack
    while (!temp_stack.empty()) {
        ctx.main_stack.push(temp_stack.top());
        temp_stack.pop();
    }
    
    push_stack(picked, ctx);
    return true;
}

bool VMEngine::op_roll(ExecutionContext& ctx) {
    if (ctx.main_stack.empty()) return false;
    
    std::vector<uint8_t> n_bytes = pop_stack(ctx);
    int64_t n = cast_to_number(n_bytes);
    
    if (n < 0 || static_cast<size_t>(n) >= ctx.main_stack.size()) return false;
    
    if (n == 0) return true; // No operation needed
    
    // Remove the nth item and put it on top
    std::stack<std::vector<uint8_t>> temp_stack;
    for (int64_t i = 0; i < n; ++i) {
        temp_stack.push(ctx.main_stack.top());
        ctx.main_stack.pop();
    }
    
    std::vector<uint8_t> rolled = ctx.main_stack.top();
    ctx.main_stack.pop();
    
    // Restore stack
    while (!temp_stack.empty()) {
        ctx.main_stack.push(temp_stack.top());
        temp_stack.pop();
    }
    
    push_stack(rolled, ctx);
    return true;
}

bool VMEngine::op_rot(ExecutionContext& ctx) {
    if (ctx.main_stack.size() < 3) return false;
    
    std::vector<uint8_t> a = pop_stack(ctx);
    std::vector<uint8_t> b = pop_stack(ctx);
    std::vector<uint8_t> c = pop_stack(ctx);
    
    push_stack(b, ctx);
    push_stack(a, ctx);
    push_stack(c, ctx);
    return true;
}

// Arithmetic operations
bool VMEngine::op_add(ExecutionContext& ctx) {
    if (ctx.main_stack.size() < 2) return false;
    
    int64_t a = cast_to_number(pop_stack(ctx));
    int64_t b = cast_to_number(pop_stack(ctx));
    
    push_stack(number_to_bytes(a + b), ctx);
    return true;
}

bool VMEngine::op_sub(ExecutionContext& ctx) {
    if (ctx.main_stack.size() < 2) return false;
    
    int64_t a = cast_to_number(pop_stack(ctx));
    int64_t b = cast_to_number(pop_stack(ctx));
    
    push_stack(number_to_bytes(b - a), ctx);
    return true;
}

bool VMEngine::op_mul(ExecutionContext& ctx) {
    if (ctx.main_stack.size() < 2) return false;
    
    int64_t a = cast_to_number(pop_stack(ctx));
    int64_t b = cast_to_number(pop_stack(ctx));
    
    push_stack(number_to_bytes(a * b), ctx);
    return true;
}

bool VMEngine::op_div(ExecutionContext& ctx) {
    if (ctx.main_stack.size() < 2) return false;
    
    int64_t a = cast_to_number(pop_stack(ctx));
    int64_t b = cast_to_number(pop_stack(ctx));
    
    if (a == 0) return false; // Division by zero
    
    push_stack(number_to_bytes(b / a), ctx);
    return true;
}

bool VMEngine::op_mod(ExecutionContext& ctx) {
    if (ctx.main_stack.size() < 2) return false;
    
    int64_t a = cast_to_number(pop_stack(ctx));
    int64_t b = cast_to_number(pop_stack(ctx));
    
    if (a == 0) return false; // Modulo by zero
    
    push_stack(number_to_bytes(b % a), ctx);
    return true;
}

bool VMEngine::op_equal(ExecutionContext& ctx) {
    if (ctx.main_stack.size() < 2) return false;
    
    int64_t a = cast_to_number(pop_stack(ctx));
    int64_t b = cast_to_number(pop_stack(ctx));
    
    push_stack(number_to_bytes(a == b ? 1 : 0), ctx);
    return true;
}

bool VMEngine::op_lessthan(ExecutionContext& ctx) {
    if (ctx.main_stack.size() < 2) return false;
    
    int64_t a = cast_to_number(pop_stack(ctx));
    int64_t b = cast_to_number(pop_stack(ctx));
    
    push_stack(number_to_bytes(b < a ? 1 : 0), ctx);
    return true;
}

bool VMEngine::op_greaterthan(ExecutionContext& ctx) {
    if (ctx.main_stack.size() < 2) return false;
    
    int64_t a = cast_to_number(pop_stack(ctx));
    int64_t b = cast_to_number(pop_stack(ctx));
    
    push_stack(number_to_bytes(b > a ? 1 : 0), ctx);
    return true;
}

// Cryptographic operations
bool VMEngine::op_hash160(ExecutionContext& ctx) {
    if (ctx.main_stack.empty()) return false;
    
    std::vector<uint8_t> data = pop_stack(ctx);
    crypto::Hash160 hash = crypto::RIPEMD160::hash(data.data(), data.size());
    
    std::vector<uint8_t> hash_bytes(hash.data(), hash.data() + 20);
    push_stack(hash_bytes, ctx);
    return true;
}

bool VMEngine::op_hash256(ExecutionContext& ctx) {
    if (ctx.main_stack.empty()) return false;
    
    std::vector<uint8_t> data = pop_stack(ctx);
    crypto::Hash256 hash = crypto::SHA256::hash(data.data(), data.size());
    
    std::vector<uint8_t> hash_bytes(hash.data(), hash.data() + 32);
    push_stack(hash_bytes, ctx);
    return true;
}

bool VMEngine::op_checksig(ExecutionContext& ctx) {
    if (ctx.main_stack.size() < 2) return false;
    
    std::vector<uint8_t> pubkey_bytes = pop_stack(ctx);
    std::vector<uint8_t> signature_bytes = pop_stack(ctx);
    
    // Create PublicKey from bytes
    if (pubkey_bytes.size() != 33 && pubkey_bytes.size() != 65) {
        push_stack({0}, ctx); // Invalid pubkey
        return true;
    }
    
    crypto::PublicKey pubkey;
    std::copy(pubkey_bytes.begin(), pubkey_bytes.end(), pubkey.data());
    
    // Create signature from bytes (simplified)
    if (signature_bytes.size() < 64) {
        push_stack({0}, ctx); // Invalid signature
        return true;
    }
    
    // For educational purposes, we'll do a simplified signature check
    // In production, this would involve proper ECDSA verification
    crypto::Hash256 tx_hash = ctx.transaction->get_hash();
    
    // Create signature from bytes - for now simplified verification
    bool is_valid = false;
    if (signature_bytes.size() >= 64) {
        crypto::Signature sig;
        std::copy(signature_bytes.begin(), signature_bytes.begin() + 64, sig.data());
        is_valid = crypto::ECDSA::verify(tx_hash, sig, pubkey);
    }
    
    push_stack(is_valid ? std::vector<uint8_t>{1} : std::vector<uint8_t>{0}, ctx);
    return true;
}

bool VMEngine::op_checkmultisig(ExecutionContext& ctx) {
    if (ctx.main_stack.empty()) return false;
    
    int64_t n = cast_to_number(pop_stack(ctx));
    if (n < 0 || n > 20 || static_cast<size_t>(n) > ctx.main_stack.size()) return false;
    
    std::vector<std::vector<uint8_t>> pubkeys;
    for (int i = 0; i < n; ++i) {
        pubkeys.push_back(pop_stack(ctx));
    }
    
    if (ctx.main_stack.empty()) return false;
    int64_t m = cast_to_number(pop_stack(ctx));
    if (m < 0 || m > n || static_cast<size_t>(m) > ctx.main_stack.size()) return false;
    
    std::vector<std::vector<uint8_t>> signatures;
    for (int i = 0; i < m; ++i) {
        signatures.push_back(pop_stack(ctx));
    }
    
    // Simplified multisig check for educational purposes
    int valid_sigs = 0;
    crypto::Hash256 tx_hash = ctx.transaction->get_hash();
    
    for (const auto& sig_bytes : signatures) {
        for (const auto& pubkey_bytes : pubkeys) {
            if (pubkey_bytes.size() != 33 && pubkey_bytes.size() != 65) continue;
            if (sig_bytes.size() < 64) continue;
            
            crypto::PublicKey pubkey;
            std::copy(pubkey_bytes.begin(), pubkey_bytes.end(), pubkey.data());
            
            if (sig_bytes.size() >= 64) {
                crypto::Signature sig;
                std::copy(sig_bytes.begin(), sig_bytes.begin() + 64, sig.data());
                if (crypto::ECDSA::verify(tx_hash, sig, pubkey)) {
                    valid_sigs++;
                    break;
                }
            }
        }
    }
    
    push_stack(valid_sigs >= m ? std::vector<uint8_t>{1} : std::vector<uint8_t>{0}, ctx);
    return true;
}

// Control flow operations
bool VMEngine::op_verify(ExecutionContext& ctx) {
    if (ctx.main_stack.empty()) return false;
    
    std::vector<uint8_t> top = pop_stack(ctx);
    return cast_to_bool(top);
}

// Educational operations
bool VMEngine::op_edu_print(ExecutionContext& ctx) {
    if (ctx.main_stack.empty()) return false;
    
    std::vector<uint8_t> data = ctx.main_stack.top();
    std::stringstream ss;
    ss << "EDU_PRINT: ";
    for (uint8_t byte : data) {
        ss << std::hex << std::setfill('0') << std::setw(2) << static_cast<int>(byte);
    }
    
    if (ctx.debug_mode) {
        ctx.debug_log.push_back(ss.str());
    }
    
    return true;
}

bool VMEngine::op_edu_log(ExecutionContext& ctx) {
    if (ctx.main_stack.empty()) return false;
    
    std::vector<uint8_t> data = pop_stack(ctx);
    std::string message(data.begin(), data.end());
    
    if (ctx.debug_mode) {
        ctx.debug_log.push_back("EDU_LOG: " + message);
    }
    
    return true;
}

bool VMEngine::op_edu_timestamp(ExecutionContext& ctx) {
    auto now = std::chrono::system_clock::now();
    auto timestamp = std::chrono::duration_cast<std::chrono::seconds>(now.time_since_epoch()).count();
    
    push_stack(number_to_bytes(timestamp), ctx);
    return true;
}

// Helper functions
std::vector<uint8_t> VMEngine::pop_stack(ExecutionContext& ctx) {
    if (ctx.main_stack.empty()) return {};
    
    std::vector<uint8_t> result = ctx.main_stack.top();
    ctx.main_stack.pop();
    return result;
}

void VMEngine::push_stack(const std::vector<uint8_t>& data, ExecutionContext& ctx) {
    ctx.main_stack.push(data);
}

bool VMEngine::cast_to_bool(const std::vector<uint8_t>& data) {
    if (data.empty()) return false;
    
    for (size_t i = 0; i < data.size(); ++i) {
        if (data[i] != 0) {
            // Check for negative zero
            if (i == data.size() - 1 && data[i] == 0x80) {
                return false;
            }
            return true;
        }
    }
    return false;
}

int64_t VMEngine::cast_to_number(const std::vector<uint8_t>& data) {
    if (data.empty()) return 0;
    
    int64_t result = 0;
    bool negative = false;
    
    // Check for sign bit in the last byte
    if (!data.empty() && (data.back() & 0x80)) {
        negative = true;
    }
    
    // Convert little-endian bytes to number
    for (size_t i = 0; i < data.size(); ++i) {
        uint8_t byte = data[i];
        if (i == data.size() - 1 && negative) {
            byte &= 0x7f; // Remove sign bit
        }
        result |= static_cast<int64_t>(byte) << (i * 8);
    }
    
    return negative ? -result : result;
}

std::vector<uint8_t> VMEngine::number_to_bytes(int64_t number) {
    if (number == 0) return {};
    
    std::vector<uint8_t> result;
    bool negative = number < 0;
    if (negative) number = -number;
    
    while (number > 0) {
        result.push_back(number & 0xff);
        number >>= 8;
    }
    
    // Add sign bit if necessary
    if (negative) {
        if (result.back() & 0x80) {
            result.push_back(0x80);
        } else {
            result.back() |= 0x80;
        }
    } else if (result.back() & 0x80) {
        result.push_back(0x00);
    }
    
    return result;
}

uint64_t VMEngine::calculate_opcode_gas(Opcode opcode) {
    switch (opcode) {
        case Opcode::OP_CHECKSIG:
            return 100;
        case Opcode::OP_CHECKMULTISIG:
            return 200;
        case Opcode::OP_HASH160:
        case Opcode::OP_HASH256:
            return 50;
        case Opcode::OP_MUL:
        case Opcode::OP_DIV:
        case Opcode::OP_MOD:
            return 10;
        default:
            return 1;
    }
}

bool VMEngine::consume_gas(ExecutionContext& ctx, uint64_t gas) {
    if (ctx.gas_used + gas > ctx.gas_limit) {
        return false;
    }
    ctx.gas_used += gas;
    return true;
}

// Script validation and utility functions
bool VMEngine::validate_script_syntax(const std::vector<uint8_t>& script) {
    size_t pc = 0;
    
    while (pc < script.size()) {
        uint8_t opcode_byte = script[pc];
        pc++;
        
        // Check push data operations
        if (opcode_byte >= 1 && opcode_byte <= 75) {
            if (pc + opcode_byte > script.size()) {
                return false; // Script ends unexpectedly
            }
            pc += opcode_byte;
            continue;
        }
        
        // Check for valid opcodes
        if (!is_valid_opcode(opcode_byte)) {
            return false;
        }
        
        // Handle PUSHDATA operations
        if (opcode_byte == static_cast<uint8_t>(Opcode::OP_PUSHDATA1)) {
            if (pc >= script.size()) return false;
            uint8_t len = script[pc++];
            if (pc + len > script.size()) return false;
            pc += len;
        } else if (opcode_byte == static_cast<uint8_t>(Opcode::OP_PUSHDATA2)) {
            if (pc + 1 >= script.size()) return false;
            uint16_t len = script[pc] | (script[pc + 1] << 8);
            pc += 2;
            if (pc + len > script.size()) return false;
            pc += len;
        }
    }
    
    return true;
}

crypto::Hash256 VMEngine::calculate_script_hash(const std::vector<uint8_t>& script) {
    return crypto::SHA256::hash(script.data(), script.size());
}

std::vector<uint8_t> VMEngine::create_p2pkh_script(const crypto::Hash160& pubkey_hash) {
    ScriptBuilder builder;
    return builder
        .add_opcode(Opcode::OP_DUP)
        .add_opcode(Opcode::OP_HASH160)
        .add_hash160(pubkey_hash)
        .add_opcode(Opcode::OP_NUMEQUAL)
        .add_opcode(Opcode::OP_VERIFY)
        .add_opcode(Opcode::OP_CHECKSIG)
        .build();
}

std::vector<uint8_t> VMEngine::create_p2sh_script(const crypto::Hash256& script_hash) {
    ScriptBuilder builder;
    return builder
        .add_opcode(Opcode::OP_HASH256)
        .add_hash256(script_hash)
        .add_opcode(Opcode::OP_NUMEQUAL)
        .build();
}

std::vector<uint8_t> VMEngine::create_multisig_script(
    const std::vector<crypto::PublicKey>& pubkeys,
    size_t required_sigs
) {
    if (pubkeys.empty() || required_sigs == 0 || required_sigs > pubkeys.size() || pubkeys.size() > 16) {
        return {};
    }
    
    ScriptBuilder builder;
    
    // Add required signatures count
    builder.add_number(required_sigs);
    
    // Add public keys
    for (const auto& pubkey : pubkeys) {
        builder.add_pubkey(pubkey);
    }
    
    // Add total public key count
    builder.add_number(pubkeys.size());
    
    // Add CHECKMULTISIG opcode
    builder.add_opcode(Opcode::OP_CHECKMULTISIG);
    
    return builder.build();
}

bool VMEngine::is_valid_opcode(uint8_t opcode) {
    return opcode <= static_cast<uint8_t>(Opcode::OP_INVALIDOPCODE);
}

std::vector<Opcode> VMEngine::parse_script(const std::vector<uint8_t>& script) {
    std::vector<Opcode> opcodes;
    size_t pc = 0;
    
    while (pc < script.size()) {
        uint8_t opcode_byte = script[pc];
        pc++;
        
        // Handle push data operations
        if (opcode_byte >= 1 && opcode_byte <= 75) {
            pc += opcode_byte;
            continue;
        }
        
        opcodes.push_back(static_cast<Opcode>(opcode_byte));
    }
    
    return opcodes;
}

bool VMEngine::is_standard_script(const std::vector<uint8_t>& script) {
    if (script.empty()) return false;
    
    // Check for standard P2PKH pattern
    if (script.size() == 25 &&
        script[0] == static_cast<uint8_t>(Opcode::OP_DUP) &&
        script[1] == static_cast<uint8_t>(Opcode::OP_HASH160) &&
        script[2] == 20 &&
        script[23] == static_cast<uint8_t>(Opcode::OP_NUMEQUAL) &&
        script[24] == static_cast<uint8_t>(Opcode::OP_CHECKSIG)) {
        return true;
    }
    
    // Check for standard P2SH pattern
    if (script.size() == 23 &&
        script[0] == static_cast<uint8_t>(Opcode::OP_HASH256) &&
        script[1] == 32 &&
        script[34] == static_cast<uint8_t>(Opcode::OP_NUMEQUAL)) {
        return true;
    }
    
    // Check for multisig pattern
    if (script.size() >= 4) {
        uint8_t first = script[0];
        uint8_t last_two = script[script.size() - 1];
        uint8_t second_last = script[script.size() - 2];
        
        if (first >= static_cast<uint8_t>(Opcode::OP_1) &&
            first <= static_cast<uint8_t>(Opcode::OP_16) &&
            second_last >= static_cast<uint8_t>(Opcode::OP_1) &&
            second_last <= static_cast<uint8_t>(Opcode::OP_16) &&
            last_two == static_cast<uint8_t>(Opcode::OP_CHECKMULTISIG)) {
            return true;
        }
    }
    
    return false;
}

// ScriptBuilder implementation
ScriptBuilder& ScriptBuilder::add_opcode(Opcode opcode) {
    script_.push_back(static_cast<uint8_t>(opcode));
    return *this;
}

ScriptBuilder& ScriptBuilder::add_data(const std::vector<uint8_t>& data) {
    if (data.size() <= 75) {
        script_.push_back(static_cast<uint8_t>(data.size()));
        script_.insert(script_.end(), data.begin(), data.end());
    } else if (data.size() <= 0xff) {
        script_.push_back(static_cast<uint8_t>(Opcode::OP_PUSHDATA1));
        script_.push_back(static_cast<uint8_t>(data.size()));
        script_.insert(script_.end(), data.begin(), data.end());
    } else {
        script_.push_back(static_cast<uint8_t>(Opcode::OP_PUSHDATA2));
        script_.push_back(static_cast<uint8_t>(data.size() & 0xff));
        script_.push_back(static_cast<uint8_t>((data.size() >> 8) & 0xff));
        script_.insert(script_.end(), data.begin(), data.end());
    }
    return *this;
}

ScriptBuilder& ScriptBuilder::add_number(int64_t number) {
    if (number == 0) {
        return add_opcode(Opcode::OP_0);
    } else if (number >= 1 && number <= 16) {
        return add_opcode(static_cast<Opcode>(static_cast<uint8_t>(Opcode::OP_1) + number - 1));
    } else {
        VMEngine vm;
        return add_data(vm.number_to_bytes(number));
    }
}

ScriptBuilder& ScriptBuilder::add_hash160(const crypto::Hash160& hash) {
    std::vector<uint8_t> hash_bytes(hash.data(), hash.data() + 20);
    return add_data(hash_bytes);
}

ScriptBuilder& ScriptBuilder::add_hash256(const crypto::Hash256& hash) {
    std::vector<uint8_t> hash_bytes(hash.data(), hash.data() + 32);
    return add_data(hash_bytes);
}

ScriptBuilder& ScriptBuilder::add_pubkey(const crypto::PublicKey& pubkey) {
    std::vector<uint8_t> pubkey_bytes(pubkey.data(), pubkey.data() + 33);
    return add_data(pubkey_bytes);
}

std::vector<uint8_t> ScriptBuilder::build() {
    return script_;
}

void ScriptBuilder::clear() {
    script_.clear();
}

} // namespace blockchain