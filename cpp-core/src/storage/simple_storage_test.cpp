#include "blockchain/storage.hpp"
#include "blockchain/storage_config.hpp"
#include <iostream>
#include <cassert>
#include <chrono>

using namespace blockchain::storage;

void test_utxo_serialization() {
    std::cout << "Testing UTXO serialization..." << std::endl;
    
    // Create a test UTXO entry
    UTXOEntry utxo;
    utxo.output_index = 1;
    utxo.output.value = 50000000; // 0.5 BTC in satoshis
    utxo.output.script_pubkey = {0x76, 0xa9, 0x14}; // Simple script
    utxo.block_height = 100;
    utxo.is_coinbase = false;
    
    // Serialize
    auto serialized = utxo.serialize();
    assert(!serialized.empty());
    std::cout << "✓ UTXO serialized: " << serialized.size() << " bytes" << std::endl;
    
    // Deserialize
    auto deserialized = UTXOEntry::deserialize(serialized);
    assert(deserialized.has_value());
    
    // Verify data integrity
    assert(deserialized->output_index == utxo.output_index);
    assert(deserialized->output.value == utxo.output.value);
    assert(deserialized->output.script_pubkey == utxo.output.script_pubkey);
    assert(deserialized->block_height == utxo.block_height);
    assert(deserialized->is_coinbase == utxo.is_coinbase);
    
    std::cout << "✓ UTXO deserialized and verified" << std::endl;
}

void test_block_metadata_serialization() {
    std::cout << "Testing BlockMetadata serialization..." << std::endl;
    
    BlockMetadata metadata;
    metadata.height = 12345;
    metadata.timestamp = 1640995200; // Jan 1, 2022
    metadata.tx_count = 150;
    metadata.total_work = 9876543210;
    metadata.file_position = 1024;
    metadata.block_size = 2048;
    
    // Serialize
    auto serialized = metadata.serialize();
    assert(!serialized.empty());
    std::cout << "✓ BlockMetadata serialized: " << serialized.size() << " bytes" << std::endl;
    
    // Deserialize
    auto deserialized = BlockMetadata::deserialize(serialized);
    assert(deserialized.has_value());
    
    // Verify
    assert(deserialized->height == metadata.height);
    assert(deserialized->timestamp == metadata.timestamp);
    assert(deserialized->tx_count == metadata.tx_count);
    assert(deserialized->total_work == metadata.total_work);
    assert(deserialized->file_position == metadata.file_position);
    assert(deserialized->block_size == metadata.block_size);
    
    std::cout << "✓ BlockMetadata deserialized and verified" << std::endl;
}

void test_transaction_metadata_serialization() {
    std::cout << "Testing TransactionMetadata serialization..." << std::endl;
    
    TransactionMetadata metadata;
    metadata.block_height = 54321;
    metadata.tx_index = 42;
    metadata.file_position = 4096;
    metadata.tx_size = 250;
    
    // Serialize
    auto serialized = metadata.serialize();
    assert(!serialized.empty());
    std::cout << "✓ TransactionMetadata serialized: " << serialized.size() << " bytes" << std::endl;
    
    // Deserialize
    auto deserialized = TransactionMetadata::deserialize(serialized);
    assert(deserialized.has_value());
    
    // Verify
    assert(deserialized->block_height == metadata.block_height);
    assert(deserialized->tx_index == metadata.tx_index);
    assert(deserialized->file_position == metadata.file_position);
    assert(deserialized->tx_size == metadata.tx_size);
    
    std::cout << "✓ TransactionMetadata deserialized and verified" << std::endl;
}

void performance_benchmark() {
    std::cout << "Running performance benchmark..." << std::endl;
    
    const int NUM_UTXOS = 10000;
    auto start = std::chrono::high_resolution_clock::now();
    
    for (int i = 0; i < NUM_UTXOS; ++i) {
        UTXOEntry utxo;
        utxo.output_index = i;
        utxo.output.value = 100000 + i;
        utxo.output.script_pubkey = {0x76, 0xa9, 0x14, static_cast<uint8_t>(i & 0xFF)};
        utxo.block_height = i / 10;
        utxo.is_coinbase = (i % 100) == 0;
        
        auto serialized = utxo.serialize();
        auto deserialized = UTXOEntry::deserialize(serialized);
        assert(deserialized.has_value());
    }
    
    auto end = std::chrono::high_resolution_clock::now();
    auto duration = std::chrono::duration_cast<std::chrono::microseconds>(end - start);
    
    std::cout << "Processed " << NUM_UTXOS << " UTXOs in " << duration.count() << " microseconds" << std::endl;
    std::cout << "Serialization rate: " << (static_cast<uint64_t>(NUM_UTXOS) * 1000000) / duration.count() << " UTXOs/second" << std::endl;
}

int main() {
    std::cout << "=== Storage Layer Test Suite ===" << std::endl;
    
    try {
        test_utxo_serialization();
        test_block_metadata_serialization();
        test_transaction_metadata_serialization();
        performance_benchmark();
        
        std::cout << "\n✅ All storage tests passed!" << std::endl;
        return 0;
    } catch (const std::exception& e) {
        std::cerr << "❌ Test failed: " << e.what() << std::endl;
        return 1;
    }
}