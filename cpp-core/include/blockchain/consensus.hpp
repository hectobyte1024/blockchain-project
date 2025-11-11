#pragma once

#include <string>
#include <vector>
#include <cstdint>
#include <atomic>
#include <future>
#include "blockchain/crypto.hpp"

namespace blockchain {
    namespace consensus {

        using namespace crypto;

        /// Mining result structure
        struct MiningResult {
            bool success = false;
            uint32_t nonce = 0;
            Hash256 block_hash;
            uint64_t hash_operations = 0;
            double elapsed_seconds = 0.0;
        };

        /// Mining statistics
        struct MiningStats {
            uint64_t total_hashes = 0;
            double hash_rate = 0.0;  // hashes per second
            uint32_t difficulty_target = 0;
            uint64_t target_value = 0;
        };

        /// High-performance proof-of-work miner
        class ProofOfWorkMiner {
        private:
            std::atomic<bool> should_stop_{false};
            uint32_t thread_count_;
            
            // Optimized hash computation for mining
            Hash256 compute_block_hash(const std::string& block_data, uint32_t nonce) const;
            
            // Check if hash meets difficulty target
            bool meets_difficulty_target(const Hash256& hash, uint32_t difficulty_bits) const;
            
            // Convert difficulty bits to target value
            uint64_t bits_to_target(uint32_t difficulty_bits) const;
            
            // Single-threaded mining worker
            MiningResult mine_worker(
                const std::string& block_data,
                uint32_t difficulty_target,
                uint32_t start_nonce,
                uint32_t nonce_range
            ) const;

        public:
            explicit ProofOfWorkMiner(uint32_t thread_count = 1);
            ~ProofOfWorkMiner() = default;

            /// Mine a block with proof-of-work
            /// @param block_data Serialized block header data (without nonce)
            /// @param difficulty_target Target difficulty bits
            /// @param max_iterations Maximum nonce iterations (0 = unlimited)
            /// @return Mining result with success status and found nonce
            MiningResult mine_block(
                const std::string& block_data,
                uint32_t difficulty_target,
                uint64_t max_iterations = 0
            );

            /// Multi-threaded mining
            MiningResult mine_block_parallel(
                const std::string& block_data,
                uint32_t difficulty_target,
                uint64_t max_iterations = 0
            );

            /// Verify that a block hash meets the difficulty target
            bool verify_proof_of_work(
                const std::string& block_data,
                uint32_t nonce,
                uint32_t difficulty_target
            ) const;

            /// Get current mining statistics
            MiningStats get_statistics() const;

            /// Stop ongoing mining operation
            void stop_mining();

            /// Calculate expected mining time
            double estimate_mining_time(
                uint32_t difficulty_target,
                double hash_rate = 0.0  // 0 = use current hash rate
            ) const;
        };

        /// Difficulty adjustment algorithms
        class DifficultyAdjustment {
        private:
            static constexpr uint64_t TARGET_BLOCK_TIME = 600; // 10 minutes in seconds
            static constexpr uint32_t DIFFICULTY_ADJUSTMENT_INTERVAL = 2016; // blocks
            static constexpr uint32_t MAX_DIFFICULTY_BITS = 0x1d00ffff;
            static constexpr uint32_t MIN_DIFFICULTY_BITS = 0x207fffff;

        public:
            /// Calculate new difficulty target based on block times
            /// @param current_difficulty Current difficulty bits
            /// @param actual_time_span Actual time taken for last N blocks
            /// @param target_time_span Expected time for N blocks  
            /// @return New difficulty bits
            static uint32_t calculate_next_difficulty(
                uint32_t current_difficulty,
                uint64_t actual_time_span,
                uint64_t target_time_span = TARGET_BLOCK_TIME * DIFFICULTY_ADJUSTMENT_INTERVAL
            );

            /// Check if difficulty should be adjusted
            static bool should_adjust_difficulty(uint32_t block_height);

            /// Calculate difficulty from compact bits representation
            static double bits_to_difficulty(uint32_t difficulty_bits);

            /// Convert difficulty back to compact bits
            static uint32_t difficulty_to_bits(double difficulty);

            /// Validate difficulty target
            static bool is_valid_difficulty_target(uint32_t difficulty_bits);
        };

        /// Block validation for consensus
        class BlockValidator {
        public:
            /// Validate block structure and basic rules
            static bool validate_block_structure(
                const std::string& block_data,
                const std::vector<std::string>& transactions
            );

            /// Validate proof-of-work
            static bool validate_proof_of_work(
                const std::string& block_data,
                uint32_t nonce,
                uint32_t difficulty_target
            );

            /// Validate block timestamp
            static bool validate_timestamp(
                uint64_t block_timestamp,
                uint64_t previous_block_timestamp,
                uint64_t current_time
            );

            /// Validate merkle root
            static bool validate_merkle_root(
                const Hash256& claimed_merkle_root,
                const std::vector<std::string>& transactions
            );
        };

    } // namespace consensus
} // namespace blockchain