#include "blockchain/simple_consensus.hpp"
#include <openssl/sha.h>
#include <chrono>
#include <iomanip>
#include <sstream>

namespace blockchain {
    namespace consensus {

        Hash256 SimpleMiner::compute_block_hash(const std::string& block_data, uint32_t nonce) {
            // Append nonce to block data
            std::string data_with_nonce = block_data;
            data_with_nonce.append(reinterpret_cast<const char*>(&nonce), sizeof(nonce));
            
            // Single SHA-256 for simplicity
            Hash256 result;
            SHA256_CTX ctx;
            SHA256_Init(&ctx);
            SHA256_Update(&ctx, data_with_nonce.data(), data_with_nonce.size());
            SHA256_Final(result.data(), &ctx);
            
            return result;
        }

        bool SimpleMiner::meets_difficulty_target(const Hash256& hash, uint32_t difficulty_bits) {
            // Simple difficulty check: count leading zeros in hex representation
            std::ostringstream hex_stream;
            for (const auto& byte : hash) {
                hex_stream << std::hex << std::setfill('0') << std::setw(2) << static_cast<int>(byte);
            }
            
            std::string hex_hash = hex_stream.str();
            uint32_t leading_zeros = 0;
            for (char c : hex_hash) {
                if (c == '0') {
                    leading_zeros++;
                } else {
                    break;
                }
            }
            
            // Difficulty target specifies required leading zeros
            uint32_t required_zeros = difficulty_bits >> 24;  // Simple encoding
            return leading_zeros >= required_zeros;
        }

        MiningResult SimpleMiner::mine_block(
            const std::string& block_data,
            uint32_t difficulty_target,
            uint64_t max_iterations
        ) {
            MiningResult result;
            auto start_time = std::chrono::high_resolution_clock::now();
            
            for (uint64_t i = 0; i < max_iterations; ++i) {
                uint32_t nonce = static_cast<uint32_t>(i);
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

        bool SimpleMiner::verify_proof_of_work(
            const std::string& block_data,
            uint32_t nonce,
            uint32_t difficulty_target
        ) {
            Hash256 hash = compute_block_hash(block_data, nonce);
            return meets_difficulty_target(hash, difficulty_target);
        }

        uint32_t DifficultyAdjustment::calculate_next_difficulty(
            uint32_t current_difficulty,
            uint64_t actual_time_span,
            uint64_t target_time_span
        ) {
            // Simple adjustment: increase/decrease required leading zeros
            uint32_t current_zeros = current_difficulty >> 24;
            
            if (actual_time_span < target_time_span / 2) {
                // Too fast, increase difficulty
                return ((current_zeros + 1) << 24) | (current_difficulty & 0xFFFFFF);
            } else if (actual_time_span > target_time_span * 2) {
                // Too slow, decrease difficulty
                if (current_zeros > 0) {
                    return ((current_zeros - 1) << 24) | (current_difficulty & 0xFFFFFF);
                }
            }
            
            return current_difficulty;
        }

        bool DifficultyAdjustment::should_adjust_difficulty(uint32_t block_height) {
            return (block_height % 10) == 0 && block_height > 0;  // Adjust every 10 blocks for testing
        }

    } // namespace consensus
} // namespace blockchain