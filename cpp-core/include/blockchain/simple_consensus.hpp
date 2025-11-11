#pragma once

#include <string>
#include <cstdint>
#include <array>

// Simple types for consensus module
using Hash256 = std::array<uint8_t, 32>;

namespace blockchain {
    namespace consensus {

        /// Mining result structure
        struct MiningResult {
            bool success = false;
            uint32_t nonce = 0;
            Hash256 block_hash;
            uint64_t hash_operations = 0;
            double elapsed_seconds = 0.0;
        };

        /// Simple proof-of-work miner
        class SimpleMiner {
        public:
            /// Mine a block with proof-of-work
            static MiningResult mine_block(
                const std::string& block_data,
                uint32_t difficulty_target,
                uint64_t max_iterations = 100000
            );

            /// Verify that a block hash meets the difficulty target
            static bool verify_proof_of_work(
                const std::string& block_data,
                uint32_t nonce,
                uint32_t difficulty_target
            );

        private:
            /// Compute block hash with nonce
            static Hash256 compute_block_hash(const std::string& block_data, uint32_t nonce);
            
            /// Check if hash meets difficulty target
            static bool meets_difficulty_target(const Hash256& hash, uint32_t difficulty_bits);
        };

        /// Difficulty adjustment
        class DifficultyAdjustment {
        public:
            /// Calculate new difficulty target based on block times
            static uint32_t calculate_next_difficulty(
                uint32_t current_difficulty,
                uint64_t actual_time_span,
                uint64_t target_time_span
            );

            /// Check if difficulty should be adjusted
            static bool should_adjust_difficulty(uint32_t block_height);
        };

    } // namespace consensus
} // namespace blockchain