#pragma once

#include <stdint.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

/// C-compatible mining result
typedef struct {
    bool success;
    uint32_t nonce;
    uint8_t block_hash[32];
    uint64_t hash_operations;
    double elapsed_seconds;
} CMiningResult;

/// Mine a block with proof-of-work
/// @param block_data Block data as null-terminated string
/// @param difficulty_target Difficulty target bits
/// @param max_iterations Maximum iterations (0 = default limit)
/// @return Mining result
CMiningResult c_mine_block(
    const char* block_data,
    uint32_t difficulty_target,
    uint64_t max_iterations
);

/// Verify proof-of-work
/// @param block_data Block data as null-terminated string
/// @param nonce Nonce to verify
/// @param difficulty_target Difficulty target bits
/// @return true if proof-of-work is valid
bool c_verify_proof_of_work(
    const char* block_data,
    uint32_t nonce,
    uint32_t difficulty_target
);

/// Calculate next difficulty target
/// @param current_difficulty Current difficulty bits
/// @param actual_time_span Actual time taken for recent blocks
/// @param target_time_span Expected time for recent blocks
/// @return New difficulty bits
uint32_t c_calculate_next_difficulty(
    uint32_t current_difficulty,
    uint64_t actual_time_span,
    uint64_t target_time_span
);

/// Check if difficulty should be adjusted at this block height
/// @param block_height Current block height
/// @return true if difficulty should be adjusted
bool c_should_adjust_difficulty(uint32_t block_height);

#ifdef __cplusplus
}
#endif