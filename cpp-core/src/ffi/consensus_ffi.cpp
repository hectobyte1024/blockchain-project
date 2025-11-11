#include "ffi/consensus_ffi.h"
#include "blockchain/simple_consensus.hpp"
#include <cstring>

extern "C" {

CMiningResult c_mine_block(
    const char* block_data,
    uint32_t difficulty_target,
    uint64_t max_iterations
) {
    CMiningResult c_result = {0};
    
    if (!block_data) {
        return c_result;
    }
    
    try {
        std::string block_str(block_data);
        uint64_t iterations = (max_iterations == 0) ? 100000 : max_iterations;
        
        auto result = blockchain::consensus::SimpleMiner::mine_block(
            block_str, difficulty_target, iterations
        );
        
        c_result.success = result.success;
        c_result.nonce = result.nonce;
        c_result.hash_operations = result.hash_operations;
        c_result.elapsed_seconds = result.elapsed_seconds;
        
        // Copy hash data
        std::memcpy(c_result.block_hash, result.block_hash.data(), 32);
        
    } catch (...) {
        // Return failed result on any exception
        c_result.success = false;
    }
    
    return c_result;
}

bool c_verify_proof_of_work(
    const char* block_data,
    uint32_t nonce,
    uint32_t difficulty_target
) {
    if (!block_data) {
        return false;
    }
    
    try {
        std::string block_str(block_data);
        return blockchain::consensus::SimpleMiner::verify_proof_of_work(
            block_str, nonce, difficulty_target
        );
    } catch (...) {
        return false;
    }
}

uint32_t c_calculate_next_difficulty(
    uint32_t current_difficulty,
    uint64_t actual_time_span,
    uint64_t target_time_span
) {
    try {
        return blockchain::consensus::DifficultyAdjustment::calculate_next_difficulty(
            current_difficulty, actual_time_span, target_time_span
        );
    } catch (...) {
        return current_difficulty; // Return unchanged on error
    }
}

bool c_should_adjust_difficulty(uint32_t block_height) {
    try {
        return blockchain::consensus::DifficultyAdjustment::should_adjust_difficulty(block_height);
    } catch (...) {
        return false;
    }
}

} // extern "C"