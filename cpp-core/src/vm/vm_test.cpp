#include <iostream>
#include <cassert>
#include "blockchain/vm.hpp"
#include "blockchain/transaction.hpp"

using namespace blockchain;
using namespace blockchain::transaction;

void test_basic_vm_operations() {
    std::cout << "Testing basic VM operations..." << std::endl;
    
    VMEngine vm;
    
    // Test script validation
    std::vector<uint8_t> valid_script = {
        static_cast<uint8_t>(Opcode::OP_1),
        static_cast<uint8_t>(Opcode::OP_2),
        static_cast<uint8_t>(Opcode::OP_ADD)
    };
    
    bool is_valid = vm.validate_script_syntax(valid_script);
    assert(is_valid);
    std::cout << "✓ Script validation passed" << std::endl;
    
    // Test script hash calculation
    Hash256 script_hash = vm.calculate_script_hash(valid_script);
    assert(script_hash.data[0] != 0 || script_hash.data[1] != 0); // Hash should not be all zeros
    std::cout << "✓ Script hash calculation passed" << std::endl;
    
    std::cout << "Basic VM operations test completed successfully!" << std::endl;
}

void test_script_templates() {
    std::cout << "Testing script templates..." << std::endl;
    
    VMEngine vm;
    
    // Test P2PKH script creation
    crypto::Hash160 pubkey_hash = {};
    for (int i = 0; i < 20; i++) {
        pubkey_hash[i] = static_cast<uint8_t>(i);
    }
    
    std::vector<uint8_t> p2pkh_script = vm.create_p2pkh_script(pubkey_hash);
    assert(!p2pkh_script.empty());
    assert(vm.is_standard_script(p2pkh_script));
    std::cout << "✓ P2PKH script creation passed" << std::endl;
    
    // Test P2SH script creation
    crypto::Hash256 script_hash = {};
    for (int i = 0; i < 32; i++) {
        script_hash[i] = static_cast<uint8_t>(i);
    }
    
    std::vector<uint8_t> p2sh_script = vm.create_p2sh_script(script_hash);
    assert(!p2sh_script.empty());
    std::cout << "✓ P2SH script creation passed" << std::endl;
    
    // Test multisig script creation
    std::vector<crypto::PublicKey> pubkeys;
    crypto::PublicKey pk1 = {}, pk2 = {}, pk3 = {};
    
    // Initialize with dummy data
    for (int i = 0; i < 33; i++) {
        pk1[i] = static_cast<uint8_t>(i);
        pk2[i] = static_cast<uint8_t>(i + 33);
        pk3[i] = static_cast<uint8_t>(i + 66);
    }
    
    pubkeys.push_back(pk1);
    pubkeys.push_back(pk2);
    pubkeys.push_back(pk3);
    
    std::vector<uint8_t> multisig_script = vm.create_multisig_script(pubkeys, 2);
    assert(!multisig_script.empty());
    std::cout << "✓ Multisig script creation passed" << std::endl;
    
    std::cout << "Script templates test completed successfully!" << std::endl;
}

void test_script_execution() {
    std::cout << "Testing script execution..." << std::endl;
    
    VMEngine vm;
    vm.set_debug_mode(true);
    
    // Create a simple test transaction
    Transaction tx;
    tx.version = 1;
    tx.locktime = 0;
    
    // Add dummy input
    TxInput input;
    input.prev_tx_hash = {};
    input.prev_output_index = 0;
    input.script_sig = {};
    input.sequence = 0xffffffff;
    tx.inputs.push_back(input);
    
    // Add dummy output
    TxOutput output;
    output.value = 5000000000; // 50 EDU
    output.script_pubkey = {};
    tx.outputs.push_back(output);
    
    // Test simple arithmetic script: 1 + 2 = 3
    std::vector<uint8_t> arithmetic_script = {
        static_cast<uint8_t>(Opcode::OP_1),
        static_cast<uint8_t>(Opcode::OP_2),
        static_cast<uint8_t>(Opcode::OP_ADD),
        static_cast<uint8_t>(Opcode::OP_3),
        static_cast<uint8_t>(Opcode::OP_NUMEQUAL)
    };
    
    VMResult result = vm.execute_script(arithmetic_script, tx, 0);
    assert(result.success);
    std::cout << "✓ Arithmetic script execution passed" << std::endl;
    
    // Test stack operations
    std::vector<uint8_t> stack_script = {
        static_cast<uint8_t>(Opcode::OP_1),
        static_cast<uint8_t>(Opcode::OP_DUP),
        static_cast<uint8_t>(Opcode::OP_NUMEQUAL)
    };
    
    result = vm.execute_script(stack_script, tx, 0);
    assert(result.success);
    std::cout << "✓ Stack operations script execution passed" << std::endl;
    
    // Test educational opcodes
    std::vector<uint8_t> edu_script = {
        static_cast<uint8_t>(Opcode::OP_EDU_TIMESTAMP),
        static_cast<uint8_t>(Opcode::OP_EDU_PRINT),
        static_cast<uint8_t>(Opcode::OP_1)
    };
    
    result = vm.execute_script(edu_script, tx, 0);
    assert(result.success);
    std::cout << "✓ Educational opcodes execution passed" << std::endl;
    
    // Print debug log
    std::vector<std::string> debug_log = vm.get_debug_log();
    for (const auto& log_entry : debug_log) {
        std::cout << "DEBUG: " << log_entry << std::endl;
    }
    
    std::cout << "Script execution test completed successfully!" << std::endl;
}

void test_script_builder() {
    std::cout << "Testing ScriptBuilder..." << std::endl;
    
    ScriptBuilder builder;
    
    // Build a simple script: PUSH 42, DUP, EQUAL
    std::vector<uint8_t> script = builder
        .add_number(42)
        .add_opcode(Opcode::OP_DUP)
        .add_opcode(Opcode::OP_NUMEQUAL)
        .build();
    
    assert(!script.empty());
    
    VMEngine vm;
    bool is_valid = vm.validate_script_syntax(script);
    assert(is_valid);
    
    std::cout << "✓ ScriptBuilder test passed" << std::endl;
}

int main() {
    try {
        std::cout << "=== C++ VM Engine Test Suite ===" << std::endl;
        
        test_basic_vm_operations();
        test_script_templates();
        test_script_execution();
        test_script_builder();
        
        std::cout << "=== All VM tests passed! ===" << std::endl;
        return 0;
        
    } catch (const std::exception& e) {
        std::cerr << "Test failed with exception: " << e.what() << std::endl;
        return 1;
    } catch (...) {
        std::cerr << "Test failed with unknown exception" << std::endl;
        return 1;
    }
}