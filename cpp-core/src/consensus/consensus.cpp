#include "blockchain/consensus.hpp"
#include <algorithm>
#include <chrono>
#include <thread>
#include <sstream>
#include <iomanip>
#include <cmath>

namespace blockchain {
    namespace consensus {

        ProofOfWorkMiner::ProofOfWorkMiner(uint32_t thread_count) 
            : thread_count_(std::max(1u, thread_count)) {
        }

        Hash256 ProofOfWorkMiner::compute_block_hash(const std::string& block_data, uint32_t nonce) const {
            // Append nonce to block data
            std::string data_with_nonce = block_data;
            
            // Append nonce as 4-byte little-endian
            data_with_nonce.append(reinterpret_cast<const char*>(&nonce), sizeof(nonce));
            
            // Double SHA-256 (Bitcoin-style)
            std::vector<uint8_t> data_vector(data_with_nonce.begin(), data_with_nonce.end());
            return SHA256::double_hash(data_vector);
        }

        bool ProofOfWorkMiner::meets_difficulty_target(const Hash256& hash, uint32_t difficulty_bits) const {
            uint64_t target = bits_to_target(difficulty_bits);
            
            // Convert first 8 bytes of hash to uint64_t (little-endian)
            uint64_t hash_value = 0;
            for (int i = 7; i >= 0; --i) {
                hash_value = (hash_value << 8) | hash[i];
            }
            
            return hash_value <= target;
        }

        uint64_t ProofOfWorkMiner::bits_to_target(uint32_t difficulty_bits) const {
            // Extract exponent and mantissa from compact representation
            uint32_t exponent = difficulty_bits >> 24;
            uint64_t mantissa = difficulty_bits & 0xffffff;
            
            if (exponent <= 3) {
                mantissa >>= (8 * (3 - exponent));
            } else {
                mantissa <<= (8 * (exponent - 3));
            }
            
            return mantissa;
        }

        MiningResult ProofOfWorkMiner::mine_worker(
            const std::string& block_data,
            uint32_t difficulty_target,
            uint32_t start_nonce,
            uint32_t nonce_range
        ) const {
            MiningResult result;
            auto start_time = std::chrono::high_resolution_clock::now();
            
            for (uint32_t i = 0; i < nonce_range && !should_stop_.load(); ++i) {
                uint32_t nonce = start_nonce + i;
                Hash256 hash = compute_block_hash(block_data, nonce);
                result.hash_operations++;
                
                if (meets_difficulty_target(hash, difficulty_target)) {
                    result.success = true;
                    result.nonce = nonce;
                    result.block_hash = hash;
                    break;
                }
            }
            
            auto end_time = std::chrono::high_resolution_clock::now();
            auto duration = std::chrono::duration_cast<std::chrono::milliseconds>(end_time - start_time);
            result.elapsed_seconds = duration.count() / 1000.0;
            
            return result;
        }

        MiningResult ProofOfWorkMiner::mine_block(
            const std::string& block_data,
            uint32_t difficulty_target,
            uint64_t max_iterations
        ) {
            should_stop_.store(false);
            
            uint64_t iterations = (max_iterations == 0) ? UINT32_MAX : max_iterations;
            return mine_worker(block_data, difficulty_target, 0, static_cast<uint32_t>(iterations));
        }

        MiningResult ProofOfWorkMiner::mine_block_parallel(
            const std::string& block_data,
            uint32_t difficulty_target,
            uint64_t max_iterations
        ) {
            should_stop_.store(false);
            
            uint64_t iterations_per_thread = (max_iterations == 0) ? 
                (UINT32_MAX / thread_count_) : (max_iterations / thread_count_);
            
            std::vector<std::future<MiningResult>> futures;
            
            // Launch worker threads
            for (uint32_t i = 0; i < thread_count_; ++i) {
                uint32_t start_nonce = i * static_cast<uint32_t>(iterations_per_thread);
                uint32_t nonce_range = static_cast<uint32_t>(iterations_per_thread);
                
                futures.push_back(std::async(std::launch::async, [=] {
                    return mine_worker(block_data, difficulty_target, start_nonce, nonce_range);
                }));
            }
            
            // Wait for first successful result
            MiningResult final_result;
            auto start_time = std::chrono::high_resolution_clock::now();
            
            while (true) {
                for (auto& future : futures) {
                    if (future.wait_for(std::chrono::milliseconds(10)) == std::future_status::ready) {
                        MiningResult result = future.get();
                        if (result.success) {
                            should_stop_.store(true);
                            
                            // Collect statistics from all threads
                            final_result = result;
                            auto end_time = std::chrono::high_resolution_clock::now();
                            auto duration = std::chrono::duration_cast<std::chrono::milliseconds>(end_time - start_time);
                            final_result.elapsed_seconds = duration.count() / 1000.0;
                            
                            return final_result;
                        }
                        final_result.hash_operations += result.hash_operations;
                    }
                }
                
                if (should_stop_.load()) {
                    break;
                }
                
                std::this_thread::sleep_for(std::chrono::milliseconds(1));
            }
            
            auto end_time = std::chrono::high_resolution_clock::now();
            auto duration = std::chrono::duration_cast<std::chrono::milliseconds>(end_time - start_time);
            final_result.elapsed_seconds = duration.count() / 1000.0;
            
            return final_result;
        }

        bool ProofOfWorkMiner::verify_proof_of_work(
            const std::string& block_data,
            uint32_t nonce,
            uint32_t difficulty_target
        ) const {
            Hash256 hash = compute_block_hash(block_data, nonce);
            return meets_difficulty_target(hash, difficulty_target);
        }

        MiningStats ProofOfWorkMiner::get_statistics() const {
            MiningStats stats;
            stats.difficulty_target = 0; // Would be set from last mining operation
            return stats;
        }

        void ProofOfWorkMiner::stop_mining() {
            should_stop_.store(true);
        }

        double ProofOfWorkMiner::estimate_mining_time(
            uint32_t difficulty_target,
            double hash_rate
        ) const {
            if (hash_rate <= 0.0) {
                hash_rate = 1000000.0; // Default 1 MH/s
            }
            
            uint64_t target = bits_to_target(difficulty_target);
            double expected_hashes = static_cast<double>(UINT64_MAX) / static_cast<double>(target);
            
            return expected_hashes / hash_rate;
        }

        // Difficulty Adjustment Implementation
        uint32_t DifficultyAdjustment::calculate_next_difficulty(
            uint32_t current_difficulty,
            uint64_t actual_time_span,
            uint64_t target_time_span
        ) {
            // Limit adjustment to 4x increase or 1/4 decrease
            if (actual_time_span < target_time_span / 4) {
                actual_time_span = target_time_span / 4;
            }
            if (actual_time_span > target_time_span * 4) {
                actual_time_span = target_time_span * 4;
            }
            
            // Calculate adjustment factor
            double adjustment_factor = static_cast<double>(target_time_span) / static_cast<double>(actual_time_span);
            
            // Apply adjustment to current difficulty
            double current_difficulty_value = bits_to_difficulty(current_difficulty);
            double new_difficulty = current_difficulty_value * adjustment_factor;
            
            uint32_t new_difficulty_bits = difficulty_to_bits(new_difficulty);
            
            // Enforce difficulty limits
            if (new_difficulty_bits > MAX_DIFFICULTY_BITS) {
                new_difficulty_bits = MAX_DIFFICULTY_BITS;
            }
            if (new_difficulty_bits < MIN_DIFFICULTY_BITS) {
                new_difficulty_bits = MIN_DIFFICULTY_BITS;
            }
            
            return new_difficulty_bits;
        }

        bool DifficultyAdjustment::should_adjust_difficulty(uint32_t block_height) {
            return (block_height % DIFFICULTY_ADJUSTMENT_INTERVAL) == 0 && block_height > 0;
        }

        double DifficultyAdjustment::bits_to_difficulty(uint32_t difficulty_bits) {
            uint32_t max_target = 0x1d00ffff; // Maximum difficulty target
            double max_target_value = static_cast<double>((max_target & 0xffffff) << (8 * ((max_target >> 24) - 3)));
            
            uint32_t exponent = difficulty_bits >> 24;
            double mantissa = difficulty_bits & 0xffffff;
            double target_value = mantissa * std::pow(256.0, exponent - 3);
            
            return max_target_value / target_value;
        }

        uint32_t DifficultyAdjustment::difficulty_to_bits(double difficulty) {
            if (difficulty <= 0.0) return MAX_DIFFICULTY_BITS;
            
            uint32_t max_target = 0x1d00ffff;
            double max_target_value = static_cast<double>((max_target & 0xffffff) << (8 * ((max_target >> 24) - 3)));
            double target_value = max_target_value / difficulty;
            
            // Convert back to compact representation
            uint32_t exponent = 1;
            uint64_t mantissa = static_cast<uint64_t>(target_value);
            
            while (mantissa > 0xffffff) {
                mantissa >>= 8;
                exponent++;
            }
            
            return (exponent << 24) | (mantissa & 0xffffff);
        }

        bool DifficultyAdjustment::is_valid_difficulty_target(uint32_t difficulty_bits) {
            return difficulty_bits >= MIN_DIFFICULTY_BITS && difficulty_bits <= MAX_DIFFICULTY_BITS;
        }

        // Block Validator Implementation
        bool BlockValidator::validate_block_structure(
            const std::string& block_data,
            const std::vector<std::string>& transactions
        ) {
            // Basic structure validation
            return !block_data.empty() && !transactions.empty();
        }

        bool BlockValidator::validate_proof_of_work(
            const std::string& block_data,
            uint32_t nonce,
            uint32_t difficulty_target
        ) {
            ProofOfWorkMiner miner;
            return miner.verify_proof_of_work(block_data, nonce, difficulty_target);
        }

        bool BlockValidator::validate_timestamp(
            uint64_t block_timestamp,
            uint64_t previous_block_timestamp,
            uint64_t current_time
        ) {
            // Block timestamp should be greater than previous block
            if (block_timestamp <= previous_block_timestamp) {
                return false;
            }
            
            // Block timestamp should not be too far in the future (2 hours)
            if (block_timestamp > current_time + 7200) {
                return false;
            }
            
            return true;
        }

        bool BlockValidator::validate_merkle_root(
            const Hash256& claimed_merkle_root,
            const std::vector<std::string>& transactions
        ) {
            // This would compute the actual merkle root and compare
            // For now, just basic validation
            return true;
        }

    } // namespace consensus
} // namespace blockchain