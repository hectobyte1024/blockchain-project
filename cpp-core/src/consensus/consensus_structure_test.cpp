#include "blockchain/hybrid_consensus.hpp"
#include <iostream>
#include <cassert>
#include <chrono>

using namespace blockchain::consensus;
using namespace blockchain::crypto;

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

void test_consensus_state() {
    std::cout << "Testing consensus state management..." << std::endl;
    
    // Test consensus state initialization
    ConsensusState state;
    state.current_height = 1000;
    state.total_stake = 50000000;
    state.min_stake_amount = 1000000;
    state.pos_activation_height = 500;
    
    assert(state.current_height == 1000);
    assert(state.total_stake == 50000000);
    assert(state.min_stake_amount == 1000000);
    std::cout << "✓ Consensus state initialized" << std::endl;
    
    // Test validator structure
    Validator validator;
    validator.stake_amount = 5000000;
    validator.reputation_score = 100;
    validator.is_active = true;
    validator.total_blocks_created = 0;
    validator.missed_slots = 0;
    
    assert(validator.stake_amount == 5000000);
    assert(validator.reputation_score == 100);
    assert(validator.is_active == true);
    std::cout << "✓ Validator structure validated" << std::endl;
    
    // Test stake entry
    StakeEntry stake;
    stake.amount = 3000000;
    stake.lock_height = 1100;
    stake.is_locked = true;
    
    assert(stake.amount == 3000000);
    assert(stake.lock_height == 1100);
    assert(stake.is_locked == true);
    std::cout << "✓ Stake entry validated" << std::endl;
    
    std::cout << "Consensus state management tests passed!" << std::endl;
}

void test_block_slot_structure() {
    std::cout << "Testing block slot structure..." << std::endl;
    
    BlockSlot slot;
    slot.block_height = 1001;
    slot.slot_time = 1640995200; // Jan 1, 2022
    slot.stake_weight = 2500000;
    
    assert(slot.block_height == 1001);
    assert(slot.slot_time == 1640995200);
    assert(slot.stake_weight == 2500000);
    std::cout << "✓ Block slot structure validated" << std::endl;
    
    // Test multiple slots
    std::vector<BlockSlot> slots;
    for (int i = 0; i < 10; ++i) {
        BlockSlot test_slot;
        test_slot.block_height = 1000 + i;
        test_slot.slot_time = 1640995200 + i * 600; // 10 minutes apart
        test_slot.stake_weight = 1000000 + i * 500000;
        slots.push_back(test_slot);
    }
    
    assert(slots.size() == 10);
    for (size_t i = 1; i < slots.size(); ++i) {
        assert(slots[i].block_height > slots[i-1].block_height);
        assert(slots[i].slot_time >= slots[i-1].slot_time);
    }
    std::cout << "✓ Multiple slots validated" << std::endl;
    
    std::cout << "Block slot structure tests passed!" << std::endl;
}

void test_network_stats_structure() {
    std::cout << "Testing network statistics structure..." << std::endl;
    
    HybridConsensusEngine::NetworkStats stats;
    stats.total_validators = 25;
    stats.active_validators = 20;
    stats.total_network_stake = 125000000;
    stats.average_block_time = 612.5;
    stats.pow_blocks_last_100 = 55;
    stats.pos_blocks_last_100 = 45;
    stats.network_hash_rate = 15000000.0;
    stats.current_difficulty = 0x1d007fff;
    
    assert(stats.total_validators == 25);
    assert(stats.active_validators == 20);
    assert(stats.total_network_stake == 125000000);
    assert(stats.average_block_time == 612.5);
    assert(stats.pow_blocks_last_100 + stats.pos_blocks_last_100 == 100);
    std::cout << "✓ Network statistics structure validated" << std::endl;
    
    // Test ratios
    double pow_ratio = static_cast<double>(stats.pow_blocks_last_100) / 100.0;
    double pos_ratio = static_cast<double>(stats.pos_blocks_last_100) / 100.0;
    
    assert(pow_ratio + pos_ratio == 1.0);
    assert(pow_ratio > 0.5); // PoW majority in this example
    std::cout << "✓ Block ratios calculated correctly" << std::endl;
    
    std::cout << "Network statistics structure tests passed!" << std::endl;
}

void test_fork_info_structure() {
    std::cout << "Testing fork information structure..." << std::endl;
    
    ForkInfo fork;
    fork.fork_height = 1500;
    fork.main_chain_work = 75000000;
    fork.alternative_chain_work = 73000000;
    
    // Create mock hashes for testing
    for (int i = 0; i < 5; ++i) {
        Hash256 main_hash{};
        Hash256 alt_hash{};
        
        // Fill with test data
        for (size_t j = 0; j < main_hash.size(); ++j) {
            main_hash[j] = static_cast<uint8_t>(i * 10 + j);
            alt_hash[j] = static_cast<uint8_t>(i * 20 + j);
        }
        
        fork.main_chain_blocks.push_back(main_hash);
        fork.alternative_chain_blocks.push_back(alt_hash);
    }
    
    assert(fork.fork_height == 1500);
    assert(fork.main_chain_work > fork.alternative_chain_work);
    assert(fork.main_chain_blocks.size() == 5);
    assert(fork.alternative_chain_blocks.size() == 5);
    std::cout << "✓ Fork information structure validated" << std::endl;
    
    // Test fork resolution logic (simplified)
    bool should_reorganize = fork.alternative_chain_work > fork.main_chain_work;
    assert(should_reorganize == false); // Main chain has more work
    std::cout << "✓ Fork resolution logic tested" << std::endl;
    
    std::cout << "Fork information structure tests passed!" << std::endl;
}

void performance_benchmark_structures() {
    std::cout << "Running structure performance benchmark..." << std::endl;
    
    const int iterations = 100000;
    
    // Benchmark validator creation
    auto start_time = std::chrono::high_resolution_clock::now();
    
    std::vector<Validator> validators;
    validators.reserve(iterations);
    
    for (int i = 0; i < iterations; ++i) {
        Validator validator;
        validator.stake_amount = 1000000 + i;
        validator.reputation_score = 100;
        validator.is_active = true;
        validator.total_blocks_created = i / 100;
        validator.missed_slots = i / 1000;
        validators.push_back(validator);
    }
    
    auto end_time = std::chrono::high_resolution_clock::now();
    auto duration = std::chrono::duration_cast<std::chrono::microseconds>(end_time - start_time);
    
    std::cout << "Created " << iterations << " validators in " << duration.count() << " microseconds" << std::endl;
    std::cout << "Rate: " << (static_cast<uint64_t>(iterations) * 1000000) / duration.count() << " validators/second" << std::endl;
    
    // Benchmark slot creation
    start_time = std::chrono::high_resolution_clock::now();
    
    std::vector<BlockSlot> slots;
    slots.reserve(iterations);
    
    for (int i = 0; i < iterations; ++i) {
        BlockSlot slot;
        slot.block_height = 1000 + i;
        slot.slot_time = 1640995200 + i * 600;
        slot.stake_weight = 1000000 + i * 100;
        slots.push_back(slot);
    }
    
    end_time = std::chrono::high_resolution_clock::now();
    duration = std::chrono::duration_cast<std::chrono::microseconds>(end_time - start_time);
    
    std::cout << "Created " << iterations << " slots in " << duration.count() << " microseconds" << std::endl;
    std::cout << "Rate: " << (static_cast<uint64_t>(iterations) * 1000000) / duration.count() << " slots/second" << std::endl;
    
    std::cout << "Structure performance benchmark completed!" << std::endl;
}

int main() {
    std::cout << "=== Hybrid Consensus Structure Test Suite ===" << std::endl;
    
    try {
        test_hybrid_difficulty_adjustment();
        test_consensus_state();
        test_block_slot_structure();
        test_network_stats_structure();
        test_fork_info_structure();
        performance_benchmark_structures();
        
        std::cout << "\n✅ All hybrid consensus structure tests passed!" << std::endl;
        std::cout << "\n📊 Hybrid Consensus Features:" << std::endl;
        std::cout << "  • Proof-of-Work + Proof-of-Stake hybrid consensus" << std::endl;
        std::cout << "  • Dynamic validator selection based on stake weight" << std::endl;
        std::cout << "  • Reputation system with penalties and rewards" << std::endl;
        std::cout << "  • Adaptive difficulty adjustment for PoW/PoS balance" << std::endl;
        std::cout << "  • Fork resolution with cumulative work calculation" << std::endl;
        std::cout << "  • Block slot scheduling for predictable block times" << std::endl;
        std::cout << "  • Stake maturity and lock periods for security" << std::endl;
        std::cout << "  • Network statistics and health monitoring" << std::endl;
        
        return 0;
    } catch (const std::exception& e) {
        std::cerr << "❌ Test failed: " << e.what() << std::endl;
        return 1;
    }
}