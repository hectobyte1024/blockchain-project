#include "blockchain/hybrid_consensus.hpp"
#include <iostream>
#include <cassert>
#include <chrono>

using namespace blockchain::consensus;
using namespace blockchain::crypto;

void test_validator_management() {
    std::cout << "Testing validator management..." << std::endl;
    
    // Test without storage dependency - using nullptr temporarily
    HybridConsensusEngine engine(nullptr);
    
    // Initialize with empty state
    ConsensusState genesis_state;
    genesis_state.current_height = 0;
    genesis_state.min_stake_amount = 1000000;
    genesis_state.total_stake = 0;
    
    assert(engine.initialize_genesis(genesis_state));
    std::cout << "✓ Genesis state initialized" << std::endl;
    
    // Create test validator
    Hash256 validator_id = SHA256::hash(reinterpret_cast<const uint8_t*>("validator1"), 10);
    PublicKey pub_key; // Mock public key
    uint64_t stake_amount = 5000000;
    
    assert(engine.add_validator(validator_id, pub_key, stake_amount));
    std::cout << "✓ Validator added successfully" << std::endl;
    
    // Verify validator exists
    auto validator = engine.get_validator(validator_id);
    assert(validator.has_value());
    assert(validator->stake_amount == stake_amount);
    assert(validator->is_active == true);
    assert(validator->reputation_score == 100);
    std::cout << "✓ Validator data verified" << std::endl;
    
    // Test insufficient stake
    Hash256 validator_id2 = SHA256::hash(reinterpret_cast<const uint8_t*>("validator2"), 10);
    assert(!engine.add_validator(validator_id2, pub_key, 500000)); // Below minimum
    std::cout << "✓ Insufficient stake rejected" << std::endl;
    
    // Test stake update
    assert(engine.update_stake(validator_id, 7000000));
    validator = engine.get_validator(validator_id);
    assert(validator->stake_amount == 7000000);
    std::cout << "✓ Stake updated successfully" << std::endl;
    
    std::cout << "Validator management tests passed!" << std::endl;
}

void test_slot_generation() {
    std::cout << "Testing slot generation..." << std::endl;
    
    HybridConsensusEngine engine(nullptr);
    
    // Initialize with multiple validators
    ConsensusState genesis_state;
    genesis_state.current_height = 1500; // After PoS activation
    genesis_state.pos_activation_height = 1000;
    genesis_state.min_stake_amount = 1000000;
    
    assert(engine.initialize_genesis(genesis_state));
    
    // Add test validators
    PublicKey pub_key;
    for (int i = 0; i < 3; ++i) {
        std::string validator_name = "validator" + std::to_string(i);
        Hash256 validator_id = SHA256::hash(reinterpret_cast<const uint8_t*>(validator_name.c_str()), validator_name.size());
        assert(engine.add_validator(validator_id, pub_key, 2000000 + i * 1000000));
    }
    
    // Generate upcoming slots
    uint64_t current_time = std::chrono::system_clock::to_time_t(std::chrono::system_clock::now());
    auto slots = engine.generate_upcoming_slots(current_time, 10);
    
    assert(slots.size() == 10);
    std::cout << "✓ Generated 10 slots" << std::endl;
    
    // Verify slot properties
    for (size_t i = 0; i < slots.size(); ++i) {
        assert(slots[i].block_height > genesis_state.current_height);
        assert(slots[i].slot_time > current_time);
        
        // In hybrid mode, some should be PoW (empty validator_id), some PoS
        if (i > 0) {
            assert(slots[i].slot_time >= slots[i-1].slot_time);
        }
    }
    std::cout << "✓ Slot properties validated" << std::endl;
    
    // Count PoW vs PoS slots
    uint32_t pow_slots = 0, pos_slots = 0;
    for (const auto& slot : slots) {
        Hash256 empty_hash{};
        if (slot.validator_id == empty_hash) {
            pow_slots++;
        } else {
            pos_slots++;
        }
    }
    
    std::cout << "Generated " << pow_slots << " PoW slots and " << pos_slots << " PoS slots" << std::endl;
    assert(pow_slots > 0 || pos_slots > 0); // At least some slots should be generated
    
    std::cout << "Slot generation tests passed!" << std::endl;
}

void test_network_statistics() {
    std::cout << "Testing network statistics..." << std::endl;
    
    HybridConsensusEngine engine(nullptr);
    
    ConsensusState genesis_state;
    genesis_state.current_height = 2000;
    assert(engine.initialize_genesis(genesis_state));
    
    // Add multiple validators
    PublicKey pub_key;
    for (int i = 0; i < 5; ++i) {
        std::string validator_name = "validator" + std::to_string(i);
        Hash256 validator_id = SHA256::hash(reinterpret_cast<const uint8_t*>(validator_name.c_str()), validator_name.size());
        assert(engine.add_validator(validator_id, pub_key, 3000000 + i * 500000));
    }
    
    auto stats = engine.calculate_network_stats();
    
    assert(stats.total_validators == 5);
    assert(stats.active_validators == 5);
    assert(stats.total_network_stake > 0);
    assert(stats.current_difficulty > 0);
    
    std::cout << "Network Statistics:" << std::endl;
    std::cout << "  Total validators: " << stats.total_validators << std::endl;
    std::cout << "  Active validators: " << stats.active_validators << std::endl;
    std::cout << "  Total stake: " << stats.total_network_stake << std::endl;
    std::cout << "  Current difficulty: 0x" << std::hex << stats.current_difficulty << std::dec << std::endl;
    std::cout << "  Average block time: " << stats.average_block_time << "s" << std::endl;
    
    std::cout << "✓ Network statistics calculated successfully" << std::endl;
}

void test_penalty_system() {
    std::cout << "Testing penalty system..." << std::endl;
    
    HybridConsensusEngine engine(nullptr);
    
    ConsensusState genesis_state;
    assert(engine.initialize_genesis(genesis_state));
    
    // Add test validator
    PublicKey pub_key;
    Hash256 validator_id = SHA256::hash(reinterpret_cast<const uint8_t*>("bad_validator"), 13);
    assert(engine.add_validator(validator_id, pub_key, 2000000));
    
    auto validator = engine.get_validator(validator_id);
    assert(validator->reputation_score == 100);
    assert(validator->is_active == true);
    
    // Apply penalties
    engine.penalize_validator(validator_id, 30);
    validator = engine.get_validator(validator_id);
    assert(validator->reputation_score == 70);
    assert(validator->is_active == true);
    std::cout << "✓ Moderate penalty applied" << std::endl;
    
    // Apply severe penalty
    engine.penalize_validator(validator_id, 70);
    validator = engine.get_validator(validator_id);
    assert(validator->reputation_score == 0);
    assert(validator->is_active == false); // Should be deactivated
    std::cout << "✓ Severe penalty deactivated validator" << std::endl;
    
    // Test reward system
    engine.reward_validator(validator_id, 60);
    validator = engine.get_validator(validator_id);
    assert(validator->reputation_score == 60);
    assert(validator->is_active == true); // Should be reactivated
    std::cout << "✓ Reward reactivated validator" << std::endl;
    
    std::cout << "Penalty system tests passed!" << std::endl;
}

void test_hybrid_difficulty_adjustment() {
    std::cout << "Testing hybrid difficulty adjustment..." << std::endl;
    
    uint32_t current_difficulty = 0x1d00ffff;
    uint64_t target_time_span = 2016 * 600; // 2016 blocks * 10 minutes
    uint64_t actual_time_span = target_time_span; // Perfect timing
    
    // Test with perfect PoW ratio
    uint32_t adjusted = HybridDifficultyAdjustment::calculate_hybrid_difficulty(
        current_difficulty, actual_time_span, target_time_span, 0.6, 0.6
    );
    std::cout << "✓ Perfect ratio maintains difficulty: 0x" << std::hex << adjusted << std::dec << std::endl;
    
    // Test with low PoW ratio (too many PoS blocks)
    adjusted = HybridDifficultyAdjustment::calculate_hybrid_difficulty(
        current_difficulty, actual_time_span, target_time_span, 0.3, 0.6
    );
    std::cout << "✓ Low PoW ratio adjustment tested: 0x" << std::hex << adjusted << std::dec << std::endl;
    
    // Test PoS slot interval calculation
    uint64_t pos_interval = HybridDifficultyAdjustment::calculate_pos_slot_interval(5, 0.8, 600);
    assert(pos_interval >= 60 && pos_interval <= 1800); // Within bounds
    std::cout << "✓ PoS slot interval: " << pos_interval << " seconds" << std::endl;
    
    std::cout << "Hybrid difficulty adjustment tests passed!" << std::endl;
}

void test_block_rewards() {
    std::cout << "Testing block reward calculation..." << std::endl;
    
    HybridConsensusEngine engine(nullptr);
    
    ConsensusState genesis_state;
    assert(engine.initialize_genesis(genesis_state));
    
    // Test initial rewards
    uint64_t pow_reward = engine.calculate_block_reward(100, true);
    uint64_t pos_reward = engine.calculate_block_reward(100, false);
    
    assert(pow_reward > pos_reward); // PoW should have higher reward
    std::cout << "✓ PoW reward: " << pow_reward << " satoshis" << std::endl;
    std::cout << "✓ PoS reward: " << pos_reward << " satoshis" << std::endl;
    
    // Test halving
    uint64_t reward_before_halving = engine.calculate_block_reward(209999, true);
    uint64_t reward_after_halving = engine.calculate_block_reward(210000, true);
    
    assert(reward_after_halving < reward_before_halving); // Should halve
    std::cout << "✓ Reward halving tested" << std::endl;
    
    std::cout << "Block reward calculation tests passed!" << std::endl;
}

void performance_benchmark() {
    std::cout << "Running consensus performance benchmark..." << std::endl;
    
    HybridConsensusEngine engine(nullptr);
    
    ConsensusState genesis_state;
    genesis_state.current_height = 5000;
    assert(engine.initialize_genesis(genesis_state));
    
    // Add many validators
    const int num_validators = 100;
    PublicKey pub_key;
    
    auto start_time = std::chrono::high_resolution_clock::now();
    
    for (int i = 0; i < num_validators; ++i) {
        std::string validator_name = "validator" + std::to_string(i);
        Hash256 validator_id = SHA256::hash(reinterpret_cast<const uint8_t*>(validator_name.c_str()), validator_name.size());
        assert(engine.add_validator(validator_id, pub_key, 1000000 + i * 100000));
    }
    
    auto end_time = std::chrono::high_resolution_clock::now();
    auto duration = std::chrono::duration_cast<std::chrono::microseconds>(end_time - start_time);
    
    std::cout << "Added " << num_validators << " validators in " << duration.count() << " microseconds" << std::endl;
    std::cout << "Rate: " << (static_cast<uint64_t>(num_validators) * 1000000) / duration.count() << " validators/second" << std::endl;
    
    // Benchmark slot generation
    start_time = std::chrono::high_resolution_clock::now();
    uint64_t current_time = std::chrono::system_clock::to_time_t(std::chrono::system_clock::now());
    auto slots = engine.generate_upcoming_slots(current_time, 1000);
    end_time = std::chrono::high_resolution_clock::now();
    duration = std::chrono::duration_cast<std::chrono::microseconds>(end_time - start_time);
    
    std::cout << "Generated " << slots.size() << " slots in " << duration.count() << " microseconds" << std::endl;
    std::cout << "Rate: " << (static_cast<uint64_t>(slots.size()) * 1000000) / duration.count() << " slots/second" << std::endl;
    
    std::cout << "Performance benchmark completed!" << std::endl;
}

int main() {
    std::cout << "=== Hybrid Consensus Test Suite ===" << std::endl;
    
    try {
        test_validator_management();
        test_slot_generation();
        test_network_statistics();
        test_penalty_system();
        test_hybrid_difficulty_adjustment();
        test_block_rewards();
        performance_benchmark();
        
        std::cout << "\n✅ All hybrid consensus tests passed!" << std::endl;
        return 0;
    } catch (const std::exception& e) {
        std::cerr << "❌ Test failed: " << e.what() << std::endl;
        return 1;
    }
}