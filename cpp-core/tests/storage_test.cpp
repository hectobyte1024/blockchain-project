#include "blockchain/storage.hpp"
#include "blockchain/transaction.hpp"
#include "blockchain/block.hpp"
#include "blockchain/crypto.hpp"
#include <gtest/gtest.h>
#include <memory>
#include <vector>
#include <random>

using namespace blockchain;
using namespace blockchain::storage;
using namespace blockchain::crypto;
using namespace blockchain::transaction;
using namespace blockchain::block;

class StorageTest : public ::testing::Test {
protected:
    void SetUp() override {
        // Create test storage
        memory_storage_ = StorageFactory::create_test();
        
        // Create test configuration for LevelDB storage
        config_.data_directory = "./test_blockchain_data";
        config_.cache_size_mb = 16;
        config_.write_buffer_size_mb = 8;
        config_.max_open_files = 100;
        
        leveldb_storage_ = StorageFactory::create(StorageFactory::StorageType::LEVELDB, config_);
        auto* leveldb_impl = static_cast<LevelDBStorage*>(leveldb_storage_.get());
        leveldb_impl->initialize();
        
        // Create test data
        create_test_data();
    }
    
    void TearDown() override {
        // Clean up test data directory
        std::filesystem::remove_all(config_.data_directory);
    }
    
    void create_test_data() {
        // Create test transactions
        test_tx1_ = Transaction{};
        test_tx1_.version = 1;
        test_tx1_.lock_time = 0;
        
        // Add coinbase input
        TxInput coinbase_input;
        coinbase_input.prev_tx_hash = Hash256{}; // All zeros for coinbase
        coinbase_input.prev_output_index = 0xFFFFFFFF;
        coinbase_input.script_sig = "coinbase_data";
        coinbase_input.sequence = 0xFFFFFFFF;
        test_tx1_.inputs.push_back(coinbase_input);
        
        // Add outputs
        TxOutput output1;
        output1.value = 5000000000ULL; // 50 coins
        output1.script_pubkey = "OP_DUP OP_HASH160 <pubkeyhash1> OP_EQUALVERIFY OP_CHECKSIG";
        test_tx1_.outputs.push_back(output1);
        
        TxOutput output2;
        output2.value = 2500000000ULL; // 25 coins
        output2.script_pubkey = "OP_DUP OP_HASH160 <pubkeyhash2> OP_EQUALVERIFY OP_CHECKSIG";
        test_tx1_.outputs.push_back(output2);
        
        // Create second transaction spending from first
        test_tx2_ = Transaction{};
        test_tx2_.version = 1;
        test_tx2_.lock_time = 0;
        
        TxInput input1;
        input1.prev_tx_hash = test_tx1_.calculate_hash();
        input1.prev_output_index = 0;
        input1.script_sig = "<signature1> <pubkey1>";
        input1.sequence = 0xFFFFFFFF;
        test_tx2_.inputs.push_back(input1);
        
        TxOutput output3;
        output3.value = 4900000000ULL; // 49 coins (1 coin fee)
        output3.script_pubkey = "OP_DUP OP_HASH160 <pubkeyhash3> OP_EQUALVERIFY OP_CHECKSIG";
        test_tx2_.outputs.push_back(output3);
        
        // Create test block
        test_block_ = Block{};
        test_block_.header.version = 1;
        test_block_.header.prev_block_hash = Hash256{}; // Genesis block
        test_block_.header.merkle_root = Hash256{}; // Will be calculated
        test_block_.header.timestamp = 1234567890;
        test_block_.header.bits = 0x1d00ffff;
        test_block_.header.nonce = 12345;
        test_block_.header.height = 0;
        
        test_block_.transactions.push_back(test_tx1_);
        test_block_.transactions.push_back(test_tx2_);
        
        // Calculate merkle root
        test_block_.header.merkle_root = test_block_.calculate_merkle_root();
    }
    
    std::unique_ptr<IBlockchainStorage> memory_storage_;
    std::unique_ptr<IBlockchainStorage> leveldb_storage_;
    StorageConfig config_;
    
    Transaction test_tx1_, test_tx2_;
    Block test_block_;
};

// Test basic block storage operations
TEST_F(StorageTest, BlockStorage) {
    std::vector<IBlockchainStorage*> storages = {memory_storage_.get(), leveldb_storage_.get()};
    
    for (auto* storage : storages) {
        Hash256 block_hash = test_block_.calculate_hash();
        
        // Test block doesn't exist initially
        EXPECT_EQ(storage->has_block(block_hash), StorageResult::NOT_FOUND);
        
        // Store block
        EXPECT_EQ(storage->store_block(test_block_), StorageResult::SUCCESS);
        
        // Test block exists now
        EXPECT_EQ(storage->has_block(block_hash), StorageResult::SUCCESS);
        
        // Retrieve block by hash
        Block retrieved_block;
        EXPECT_EQ(storage->get_block(block_hash, retrieved_block), StorageResult::SUCCESS);
        EXPECT_EQ(retrieved_block.header.version, test_block_.header.version);
        EXPECT_EQ(retrieved_block.header.timestamp, test_block_.header.timestamp);
        EXPECT_EQ(retrieved_block.transactions.size(), test_block_.transactions.size());
        
        // Retrieve block by height
        Block retrieved_block_by_height;
        EXPECT_EQ(storage->get_block_by_height(0, retrieved_block_by_height), StorageResult::SUCCESS);
        EXPECT_EQ(retrieved_block_by_height.header.height, 0);
        
        // Test storing duplicate block fails
        EXPECT_EQ(storage->store_block(test_block_), StorageResult::ALREADY_EXISTS);
        
        // Test retrieving non-existent block
        Hash256 fake_hash;
        fake_hash.fill(0xFF);
        Block non_existent_block;
        EXPECT_EQ(storage->get_block(fake_hash, non_existent_block), StorageResult::NOT_FOUND);
        EXPECT_EQ(storage->get_block_by_height(999, non_existent_block), StorageResult::NOT_FOUND);
    }
}

// Test transaction storage operations
TEST_F(StorageTest, TransactionStorage) {
    std::vector<IBlockchainStorage*> storages = {memory_storage_.get(), leveldb_storage_.get()};
    
    for (auto* storage : storages) {
        Hash256 tx_hash = test_tx1_.calculate_hash();
        
        // Test transaction doesn't exist initially
        EXPECT_EQ(storage->has_transaction(tx_hash), StorageResult::NOT_FOUND);
        
        // Store transaction
        EXPECT_EQ(storage->store_transaction(test_tx1_), StorageResult::SUCCESS);
        
        // Test transaction exists now
        EXPECT_EQ(storage->has_transaction(tx_hash), StorageResult::SUCCESS);
        
        // Retrieve transaction
        Transaction retrieved_tx;
        EXPECT_EQ(storage->get_transaction(tx_hash, retrieved_tx), StorageResult::SUCCESS);
        EXPECT_EQ(retrieved_tx.version, test_tx1_.version);
        EXPECT_EQ(retrieved_tx.inputs.size(), test_tx1_.inputs.size());
        EXPECT_EQ(retrieved_tx.outputs.size(), test_tx1_.outputs.size());
        
        // Test storing duplicate transaction fails
        EXPECT_EQ(storage->store_transaction(test_tx1_), StorageResult::ALREADY_EXISTS);
        
        // Test retrieving non-existent transaction
        Hash256 fake_hash;
        fake_hash.fill(0xFF);
        Transaction non_existent_tx;
        EXPECT_EQ(storage->get_transaction(fake_hash, non_existent_tx), StorageResult::NOT_FOUND);
    }
}

// Test UTXO operations
TEST_F(StorageTest, UTXOOperations) {
    std::vector<IBlockchainStorage*> storages = {memory_storage_.get(), leveldb_storage_.get()};
    
    for (auto* storage : storages) {
        Hash256 tx_hash = test_tx1_.calculate_hash();
        uint32_t output_index = 0;
        
        // Create UTXO entry
        UTXOEntry utxo;
        utxo.tx_hash = tx_hash;
        utxo.output_index = output_index;
        utxo.output = test_tx1_.outputs[output_index];
        utxo.block_height = 0;
        utxo.is_coinbase = true;
        
        // Test UTXO doesn't exist initially
        EXPECT_EQ(storage->has_utxo(tx_hash, output_index), StorageResult::NOT_FOUND);
        
        // Add UTXO
        EXPECT_EQ(storage->add_utxo(tx_hash, output_index, utxo), StorageResult::SUCCESS);
        
        // Test UTXO exists now
        EXPECT_EQ(storage->has_utxo(tx_hash, output_index), StorageResult::SUCCESS);
        
        // Retrieve UTXO
        UTXOEntry retrieved_utxo;
        EXPECT_EQ(storage->get_utxo(tx_hash, output_index, retrieved_utxo), StorageResult::SUCCESS);
        EXPECT_EQ(retrieved_utxo.tx_hash, utxo.tx_hash);
        EXPECT_EQ(retrieved_utxo.output_index, utxo.output_index);
        EXPECT_EQ(retrieved_utxo.output.value, utxo.output.value);
        EXPECT_EQ(retrieved_utxo.block_height, utxo.block_height);
        EXPECT_EQ(retrieved_utxo.is_coinbase, utxo.is_coinbase);
        
        // Test adding duplicate UTXO fails
        EXPECT_EQ(storage->add_utxo(tx_hash, output_index, utxo), StorageResult::ALREADY_EXISTS);
        
        // Remove UTXO
        EXPECT_EQ(storage->remove_utxo(tx_hash, output_index), StorageResult::SUCCESS);
        
        // Test UTXO doesn't exist after removal
        EXPECT_EQ(storage->has_utxo(tx_hash, output_index), StorageResult::NOT_FOUND);
        
        // Test removing non-existent UTXO
        EXPECT_EQ(storage->remove_utxo(tx_hash, output_index), StorageResult::NOT_FOUND);
    }
}

// Test batch operations
TEST_F(StorageTest, BatchOperations) {
    std::vector<IBlockchainStorage*> storages = {memory_storage_.get(), leveldb_storage_.get()};
    
    for (auto* storage : storages) {
        // Begin batch
        EXPECT_EQ(storage->begin_batch(), StorageResult::SUCCESS);
        
        // Perform multiple operations in batch
        EXPECT_EQ(storage->store_block(test_block_), StorageResult::SUCCESS);
        EXPECT_EQ(storage->store_transaction(test_tx1_), StorageResult::SUCCESS);
        EXPECT_EQ(storage->store_transaction(test_tx2_), StorageResult::SUCCESS);
        
        // Commit batch
        EXPECT_EQ(storage->commit_batch(), StorageResult::SUCCESS);
        
        // Verify operations were committed
        Hash256 block_hash = test_block_.calculate_hash();
        EXPECT_EQ(storage->has_block(block_hash), StorageResult::SUCCESS);
        
        Hash256 tx1_hash = test_tx1_.calculate_hash();
        EXPECT_EQ(storage->has_transaction(tx1_hash), StorageResult::SUCCESS);
        
        Hash256 tx2_hash = test_tx2_.calculate_hash();
        EXPECT_EQ(storage->has_transaction(tx2_hash), StorageResult::SUCCESS);
    }
}

// Test metadata operations
TEST_F(StorageTest, MetadataOperations) {
    std::vector<IBlockchainStorage*> storages = {memory_storage_.get(), leveldb_storage_.get()};
    
    for (auto* storage : storages) {
        Hash256 block_hash = test_block_.calculate_hash();
        
        // Create block metadata
        BlockMetadata metadata;
        metadata.block_hash = block_hash;
        metadata.prev_block_hash = test_block_.header.prev_block_hash;
        metadata.height = test_block_.header.height;
        metadata.timestamp = test_block_.header.timestamp;
        metadata.tx_count = static_cast<uint32_t>(test_block_.transactions.size());
        metadata.total_work = 0x1000; // Example work
        metadata.file_position = 0;
        metadata.block_size = test_block_.get_serialized_size();
        
        // Store metadata
        EXPECT_EQ(storage->store_block_metadata(metadata), StorageResult::SUCCESS);
        
        // Retrieve metadata by hash
        BlockMetadata retrieved_metadata;
        EXPECT_EQ(storage->get_block_metadata(block_hash, retrieved_metadata), StorageResult::SUCCESS);
        EXPECT_EQ(retrieved_metadata.block_hash, metadata.block_hash);
        EXPECT_EQ(retrieved_metadata.height, metadata.height);
        EXPECT_EQ(retrieved_metadata.tx_count, metadata.tx_count);
        EXPECT_EQ(retrieved_metadata.total_work, metadata.total_work);
        
        // Retrieve metadata by height
        BlockMetadata retrieved_metadata_by_height;
        EXPECT_EQ(storage->get_block_metadata_by_height(0, retrieved_metadata_by_height), StorageResult::SUCCESS);
        EXPECT_EQ(retrieved_metadata_by_height.height, 0);
    }
}

// Test UTXOManager
TEST_F(StorageTest, UTXOManager) {
    auto storage = StorageFactory::create_test();
    auto utxo_manager = std::make_shared<UTXOManager>(storage);
    
    // Add UTXO from transaction
    EXPECT_TRUE(utxo_manager->add_utxo(test_tx1_, 0, 0));
    EXPECT_TRUE(utxo_manager->add_utxo(test_tx1_, 1, 0));
    
    // Check UTXOs exist
    Hash256 tx_hash = test_tx1_.calculate_hash();
    EXPECT_TRUE(utxo_manager->has_utxo(tx_hash, 0));
    EXPECT_TRUE(utxo_manager->has_utxo(tx_hash, 1));
    
    // Retrieve UTXOs
    auto utxo0 = utxo_manager->get_utxo(tx_hash, 0);
    ASSERT_TRUE(utxo0.has_value());
    EXPECT_EQ(utxo0->output.value, test_tx1_.outputs[0].value);
    EXPECT_EQ(utxo0->is_coinbase, true);
    
    auto utxo1 = utxo_manager->get_utxo(tx_hash, 1);
    ASSERT_TRUE(utxo1.has_value());
    EXPECT_EQ(utxo1->output.value, test_tx1_.outputs[1].value);
    
    // Remove UTXO
    EXPECT_TRUE(utxo_manager->remove_utxo(tx_hash, 0));
    EXPECT_FALSE(utxo_manager->has_utxo(tx_hash, 0));
    EXPECT_TRUE(utxo_manager->has_utxo(tx_hash, 1)); // Other UTXO still exists
    
    // Get UTXO count
    EXPECT_EQ(utxo_manager->get_utxo_count(), 1);
}

// Test BlockchainStorageManager
TEST_F(StorageTest, BlockchainStorageManager) {
    StorageConfig config;
    config.data_directory = "./test_manager_data";
    
    BlockchainStorageManager manager(config);
    
    // Initialize manager
    EXPECT_TRUE(manager.initialize());
    
    // Store block atomically
    EXPECT_TRUE(manager.store_block_atomic(test_block_, 0));
    
    // Get blockchain statistics
    auto stats = manager.get_blockchain_stats();
    EXPECT_EQ(stats.height, 0);
    
    // Cleanup
    manager.shutdown();
    std::filesystem::remove_all(config.data_directory);
}

// Test serialization/deserialization
TEST_F(StorageTest, Serialization) {
    // Test UTXOEntry serialization
    Hash256 tx_hash = test_tx1_.calculate_hash();
    UTXOEntry original_utxo;
    original_utxo.tx_hash = tx_hash;
    original_utxo.output_index = 0;
    original_utxo.output = test_tx1_.outputs[0];
    original_utxo.block_height = 123;
    original_utxo.is_coinbase = true;
    
    // Serialize and deserialize
    std::vector<uint8_t> serialized = original_utxo.serialize();
    auto deserialized_utxo = UTXOEntry::deserialize(serialized);
    
    ASSERT_TRUE(deserialized_utxo.has_value());
    EXPECT_EQ(deserialized_utxo->tx_hash, original_utxo.tx_hash);
    EXPECT_EQ(deserialized_utxo->output_index, original_utxo.output_index);
    EXPECT_EQ(deserialized_utxo->output.value, original_utxo.output.value);
    EXPECT_EQ(deserialized_utxo->output.script_pubkey, original_utxo.output.script_pubkey);
    EXPECT_EQ(deserialized_utxo->block_height, original_utxo.block_height);
    EXPECT_EQ(deserialized_utxo->is_coinbase, original_utxo.is_coinbase);
    
    // Test BlockMetadata serialization
    BlockMetadata original_metadata;
    original_metadata.block_hash = test_block_.calculate_hash();
    original_metadata.prev_block_hash = test_block_.header.prev_block_hash;
    original_metadata.height = 456;
    original_metadata.timestamp = 1234567890;
    original_metadata.tx_count = 2;
    original_metadata.total_work = 0x12345678;
    original_metadata.file_position = 1024;
    original_metadata.block_size = 2048;
    
    std::vector<uint8_t> metadata_serialized = original_metadata.serialize();
    auto deserialized_metadata = BlockMetadata::deserialize(metadata_serialized);
    
    ASSERT_TRUE(deserialized_metadata.has_value());
    EXPECT_EQ(deserialized_metadata->block_hash, original_metadata.block_hash);
    EXPECT_EQ(deserialized_metadata->prev_block_hash, original_metadata.prev_block_hash);
    EXPECT_EQ(deserialized_metadata->height, original_metadata.height);
    EXPECT_EQ(deserialized_metadata->timestamp, original_metadata.timestamp);
    EXPECT_EQ(deserialized_metadata->tx_count, original_metadata.tx_count);
    EXPECT_EQ(deserialized_metadata->total_work, original_metadata.total_work);
    EXPECT_EQ(deserialized_metadata->file_position, original_metadata.file_position);
    EXPECT_EQ(deserialized_metadata->block_size, original_metadata.block_size);
}

// Performance test
TEST_F(StorageTest, Performance) {
    auto storage = StorageFactory::create_test();
    auto utxo_manager = std::make_shared<UTXOManager>(storage);
    
    // Generate test data
    const int NUM_TRANSACTIONS = 1000;
    std::vector<Transaction> transactions;
    std::random_device rd;
    std::mt19937 gen(rd());
    std::uniform_int_distribution<> dis(1, 100);
    
    for (int i = 0; i < NUM_TRANSACTIONS; ++i) {
        Transaction tx;
        tx.version = 1;
        tx.lock_time = 0;
        
        // Add coinbase input for simplicity
        TxInput input;
        input.prev_tx_hash = Hash256{};
        input.prev_output_index = 0xFFFFFFFF;
        input.script_sig = "coinbase_" + std::to_string(i);
        input.sequence = 0xFFFFFFFF;
        tx.inputs.push_back(input);
        
        // Add random number of outputs
        int num_outputs = dis(gen);
        for (int j = 0; j < num_outputs; ++j) {
            TxOutput output;
            output.value = dis(gen) * 100000000ULL;
            output.script_pubkey = "script_" + std::to_string(i) + "_" + std::to_string(j);
            tx.outputs.push_back(output);
        }
        
        transactions.push_back(tx);
    }
    
    // Measure UTXO operations performance
    auto start = std::chrono::high_resolution_clock::now();
    
    for (size_t i = 0; i < transactions.size(); ++i) {
        const auto& tx = transactions[i];
        for (size_t j = 0; j < tx.outputs.size(); ++j) {
            utxo_manager->add_utxo(tx, static_cast<uint32_t>(j), static_cast<uint32_t>(i));
        }
    }
    
    auto end = std::chrono::high_resolution_clock::now();
    auto duration = std::chrono::duration_cast<std::chrono::microseconds>(end - start);
    
    std::cout << "Added " << utxo_manager->get_utxo_count() 
              << " UTXOs in " << duration.count() << " microseconds" << std::endl;
    std::cout << "Rate: " << (utxo_manager->get_utxo_count() * 1000000) / duration.count() 
              << " UTXOs per second" << std::endl;
    
    // Verify all UTXOs can be retrieved
    size_t found_count = 0;
    for (const auto& tx : transactions) {
        Hash256 tx_hash = tx.calculate_hash();
        for (size_t j = 0; j < tx.outputs.size(); ++j) {
            if (utxo_manager->has_utxo(tx_hash, static_cast<uint32_t>(j))) {
                found_count++;
            }
        }
    }
    
    EXPECT_EQ(found_count, utxo_manager->get_utxo_count());
}

int main(int argc, char** argv) {
    ::testing::InitGoogleTest(&argc, argv);
    return RUN_ALL_TESTS();
}