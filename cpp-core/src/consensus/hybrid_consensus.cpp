#include "blockchain/hybrid_consensus.hpp"
#include "blockchain/crypto.hpp"
#include <algorithm>
#include <chrono>
#include <random>
#include <cmath>

namespace blockchain {
    namespace consensus {

        using namespace crypto;

        HybridConsensusEngine::HybridConsensusEngine(storage::IBlockchainStorage* storage)
            : storage_(storage), pow_miner_(std::thread::hardware_concurrency()), 
              rng_(std::chrono::steady_clock::now().time_since_epoch().count()) {
        }

        bool HybridConsensusEngine::initialize_genesis(const ConsensusState& genesis_state) {
            state_ = genesis_state;
            
            // Initialize genesis validator if provided
            if (!state_.validators.empty()) {
                for (auto& [validator_id, validator] : state_.validators) {
                    validator.is_active = true;
                    validator.reputation_score = 100;
                }
            }
            
            return true;
        }

        bool HybridConsensusEngine::add_validator(const Hash256& validator_id, 
                                                const PublicKey& public_key, 
                                                uint64_t stake_amount) {
            if (stake_amount < state_.min_stake_amount) {
                return false; // Insufficient stake
            }
            
            Validator new_validator;
            new_validator.validator_id = validator_id;
            new_validator.public_key = public_key;
            new_validator.stake_amount = stake_amount;
            new_validator.last_block_time = 0;
            new_validator.reputation_score = 100;
            new_validator.is_active = true;
            
            state_.validators[validator_id] = new_validator;
            
            // Create stake entry
            StakeEntry stake;
            stake.validator_id = validator_id;
            stake.amount = stake_amount;
            stake.lock_height = state_.current_height + state_.stake_maturity_blocks;
            stake.is_locked = true;
            
            state_.stakes[validator_id] = stake;
            state_.total_stake += stake_amount;
            
            return true;
        }

        bool HybridConsensusEngine::remove_validator(const Hash256& validator_id) {
            auto validator_it = state_.validators.find(validator_id);
            if (validator_it == state_.validators.end()) {
                return false;
            }
            
            auto stake_it = state_.stakes.find(validator_id);
            if (stake_it != state_.stakes.end()) {
                // Check if stake unlock period has passed
                if (state_.current_height >= stake_it->second.lock_height) {
                    state_.total_stake -= stake_it->second.amount;
                    state_.stakes.erase(stake_it);
                    state_.validators.erase(validator_it);
                    return true;
                }
            }
            
            return false; // Stake still locked
        }

        bool HybridConsensusEngine::update_stake(const Hash256& validator_id, uint64_t new_stake_amount) {
            auto validator_it = state_.validators.find(validator_id);
            if (validator_it == state_.validators.end()) {
                return false;
            }
            
            auto stake_it = state_.stakes.find(validator_id);
            if (stake_it != state_.stakes.end()) {
                uint64_t old_stake = stake_it->second.amount;
                state_.total_stake = state_.total_stake - old_stake + new_stake_amount;
                
                stake_it->second.amount = new_stake_amount;
                validator_it->second.stake_amount = new_stake_amount;
                
                return true;
            }
            
            return false;
        }

        Hash256 HybridConsensusEngine::select_validator_by_stake(uint64_t slot_time, const Hash256& previous_block_hash) {
            if (state_.validators.empty() || state_.total_stake == 0) {
                return Hash256{}; // No validators available
            }
            
            // Create deterministic but unpredictable selection based on previous block hash and slot time
            std::string seed_data = std::to_string(slot_time);
            // Append block hash bytes to seed
            seed_data.append(reinterpret_cast<const char*>(previous_block_hash.data()), previous_block_hash.size());
            std::vector<uint8_t> seed_bytes(seed_data.begin(), seed_data.end());
            Hash256 seed_hash = SHA256::hash(seed_bytes.data(), seed_bytes.size());
            
            // Convert first 8 bytes of hash to uint64_t for random selection
            uint64_t seed_value = 0;
            for (int i = 0; i < 8; ++i) {
                seed_value = (seed_value << 8) | seed_hash[i];
            }
            
            std::mt19937_64 slot_rng(seed_value);
            std::uniform_real_distribution<double> dist(0.0, 1.0);
            
            // Calculate weighted selection
            std::vector<std::pair<Hash256, double>> validator_weights;
            double total_weight = 0.0;
            
            for (const auto& [validator_id, validator] : state_.validators) {
                if (validator.is_active && is_validator_eligible(validator_id, slot_time)) {
                    double weight = calculate_validator_selection_weight(validator, slot_time);
                    validator_weights.emplace_back(validator_id, weight);
                    total_weight += weight;
                }
            }
            
            if (validator_weights.empty() || total_weight <= 0.0) {
                return Hash256{}; // No eligible validators
            }
            
            // Weighted random selection
            double random_value = dist(slot_rng) * total_weight;
            double accumulated_weight = 0.0;
            
            for (const auto& [validator_id, weight] : validator_weights) {
                accumulated_weight += weight;
                if (random_value <= accumulated_weight) {
                    return validator_id;
                }
            }
            
            // Fallback to last validator (should not happen)
            return validator_weights.back().first;
        }

        double HybridConsensusEngine::calculate_validator_selection_weight(const Validator& validator, uint64_t slot_time) {
            // Base weight from stake amount
            double stake_weight = static_cast<double>(validator.stake_amount) / static_cast<double>(state_.total_stake);
            
            // Reputation factor (0.5x to 1.5x multiplier)
            double reputation_factor = 0.5 + (static_cast<double>(validator.reputation_score) / 100.0);
            
            // Time since last block factor (encourages rotation)
            double time_factor = 1.0;
            if (validator.last_block_time > 0) {
                uint64_t time_since_last = slot_time - validator.last_block_time;
                time_factor = std::min(2.0, 1.0 + (static_cast<double>(time_since_last) / 3600.0)); // Max 2x after 1 hour
            }
            
            // Activity factor (penalize for missed slots)
            double activity_factor = std::max(0.1, 1.0 - (static_cast<double>(validator.missed_slots) * 0.1));
            
            return stake_weight * reputation_factor * time_factor * activity_factor;
        }

        std::vector<BlockSlot> HybridConsensusEngine::generate_upcoming_slots(uint64_t from_time, uint32_t slot_count) {
            std::vector<BlockSlot> slots;
            slots.reserve(slot_count);
            
            uint64_t current_time = from_time;
            uint32_t current_height = state_.current_height;
            
            for (uint32_t i = 0; i < slot_count; ++i) {
                BlockSlot slot;
                slot.slot_time = calculate_next_slot_time(current_time, current_height + i);
                slot.block_height = current_height + i + 1;
                
                // Determine if this should be PoW or PoS slot
                if (current_height + i + 1 >= state_.pos_activation_height) {
                    // Hybrid mode: alternate or use ratio
                    bool is_pos_slot = (i % 2 == 0); // Simple alternating for now
                    
                    if (is_pos_slot) {
                        slot.validator_id = select_validator_by_stake(slot.slot_time, state_.best_block_hash);
                        if (auto validator = get_validator(slot.validator_id)) {
                            slot.stake_weight = validator->stake_amount;
                        }
                    } else {
                        // PoW slot - no specific validator
                        slot.validator_id = Hash256{}; // Empty hash indicates PoW
                        slot.stake_weight = 0;
                    }
                } else {
                    // Pure PoW mode before PoS activation
                    slot.validator_id = Hash256{};
                    slot.stake_weight = 0;
                }
                
                slots.push_back(slot);
                current_time = slot.slot_time;
            }
            
            return slots;
        }

        bool HybridConsensusEngine::is_validator_eligible(const Hash256& validator_id, uint64_t slot_time) {
            auto validator_it = state_.validators.find(validator_id);
            if (validator_it == state_.validators.end() || !validator_it->second.is_active) {
                return false;
            }
            
            auto stake_it = state_.stakes.find(validator_id);
            if (stake_it == state_.stakes.end() || stake_it->second.is_locked) {
                return false;
            }
            
            // Check if enough time has passed since last block (prevent spam)
            const uint64_t min_block_interval = 30; // 30 seconds minimum between blocks from same validator
            if (validator_it->second.last_block_time > 0 && 
                slot_time < validator_it->second.last_block_time + min_block_interval) {
                return false;
            }
            
            return true;
        }

        uint64_t HybridConsensusEngine::calculate_next_slot_time(uint64_t current_time, uint32_t block_height) {
            // Base block time (10 minutes)
            const uint64_t base_interval = 600;
            
            if (block_height < state_.pos_activation_height) {
                // Pure PoW: fixed interval
                return current_time + base_interval;
            }
            
            // Hybrid mode: adjust based on validator participation
            uint32_t active_validators = 0;
            for (const auto& [_, validator] : state_.validators) {
                if (validator.is_active) active_validators++;
            }
            
            // More validators = shorter intervals (but not too short)
            double adjustment_factor = std::max(0.5, 1.0 - (static_cast<double>(active_validators) * 0.02));
            uint64_t adjusted_interval = static_cast<uint64_t>(base_interval * adjustment_factor);
            
            return current_time + std::max(60UL, adjusted_interval); // Minimum 1 minute
        }

        bool HybridConsensusEngine::validate_block(const Hash256& block_hash, 
                                                 const std::string& block_data, 
                                                 uint32_t nonce, 
                                                 const Hash256& validator_id) {
            // Check if this is a PoW or PoS block
            bool is_pow_block = validator_id == Hash256{};
            
            if (is_pow_block) {
                // Validate PoW
                return pow_miner_.verify_proof_of_work(block_data, nonce, state_.current_difficulty);
            } else {
                // Validate PoS
                auto validator = get_validator(validator_id);
                if (!validator) {
                    return false; // Unknown validator
                }
                
                if (!validator->is_active) {
                    return false; // Inactive validator
                }
                
                // Check if validator was selected for this slot (simplified)
                uint64_t current_time = std::chrono::system_clock::to_time_t(std::chrono::system_clock::now());
                return is_validator_eligible(validator_id, current_time);
            }
        }

        MiningResult HybridConsensusEngine::mine_pow_block(const std::string& block_data, uint32_t difficulty_target) {
            return pow_miner_.mine_block_parallel(block_data, difficulty_target);
        }

        bool HybridConsensusEngine::create_pos_block(const Hash256& validator_id, 
                                                   const std::string& block_data,
                                                   Hash256& block_hash) {
            auto validator = get_validator(validator_id);
            if (!validator) {
                return false;
            }
            
            // For PoS, the \"mining\" is just signing the block
            std::vector<uint8_t> data_bytes(block_data.begin(), block_data.end());
            block_hash = SHA256::hash(data_bytes.data(), data_bytes.size());
            
            // Update validator stats
            auto& validator_ref = state_.validators[validator_id];
            validator_ref.last_block_time = std::chrono::system_clock::to_time_t(std::chrono::system_clock::now());
            validator_ref.total_blocks_created++;
            
            return true;
        }

        bool HybridConsensusEngine::update_consensus_state(const Hash256& block_hash, 
                                                         uint32_t block_height, 
                                                         uint64_t block_time,
                                                         bool is_pow_block) {
            state_.current_height = block_height;
            state_.best_block_hash = block_hash;
            
            // Update difficulty if needed
            if (DifficultyAdjustment::should_adjust_difficulty(block_height)) {
                adjust_consensus_parameters(block_height, block_time);
            }
            
            // Update validator stakes maturity
            update_validator_stakes(block_height);
            
            return true;
        }

        void HybridConsensusEngine::update_validator_stakes(uint32_t current_height) {
            for (auto& [validator_id, stake] : state_.stakes) {
                if (stake.is_locked && current_height >= stake.lock_height) {
                    stake.is_locked = false;
                }
            }
        }

        void HybridConsensusEngine::adjust_consensus_parameters(uint32_t block_height, uint64_t block_time) {
            // Calculate time span for difficulty adjustment
            const uint32_t adjustment_interval = 100; // Adjust every 100 blocks
            uint32_t start_height = block_height - adjustment_interval;
            
            // This would require block time history from storage
            uint64_t target_time_span = adjustment_interval * 600; // 10 minutes per block
            uint64_t actual_time_span = block_time; // Simplified - would need actual calculation
            
            // Update difficulty
            state_.current_difficulty = HybridDifficultyAdjustment::calculate_hybrid_difficulty(
                state_.current_difficulty, actual_time_span, target_time_span, 0.6
            );
        }

        uint64_t HybridConsensusEngine::calculate_block_reward(uint32_t block_height, bool is_pow_block) {
            // Base reward (decreases over time)
            uint64_t base_reward = 5000000000; // 50 coins initial
            
            // Halving every 210,000 blocks (like Bitcoin)
            uint32_t halvings = block_height / 210000;
            base_reward >>= halvings;
            
            // PoS blocks get slightly lower rewards to encourage PoW participation
            if (!is_pow_block) {
                base_reward = static_cast<uint64_t>(base_reward * 0.8);
            }
            
            return base_reward;
        }

        std::optional<Validator> HybridConsensusEngine::get_validator(const Hash256& validator_id) const {
            auto it = state_.validators.find(validator_id);
            if (it != state_.validators.end()) {
                return it->second;
            }
            return std::nullopt;
        }

        std::vector<Validator> HybridConsensusEngine::get_active_validators() const {
            std::vector<Validator> active_validators;
            for (const auto& [_, validator] : state_.validators) {
                if (validator.is_active) {
                    active_validators.push_back(validator);
                }
            }
            return active_validators;
        }

        HybridConsensusEngine::NetworkStats HybridConsensusEngine::calculate_network_stats() const {
            NetworkStats stats;
            
            stats.total_validators = static_cast<uint32_t>(state_.validators.size());
            stats.total_network_stake = state_.total_stake;
            stats.current_difficulty = state_.current_difficulty;
            
            for (const auto& [_, validator] : state_.validators) {
                if (validator.is_active) {
                    stats.active_validators++;
                }
            }
            
            // These would require historical data from storage
            stats.average_block_time = 600.0; // Default 10 minutes
            stats.pow_blocks_last_100 = 60;    // Estimated
            stats.pos_blocks_last_100 = 40;    // Estimated
            stats.network_hash_rate = 1000000.0; // Estimated
            
            return stats;
        }

        void HybridConsensusEngine::penalize_validator(const Hash256& validator_id, uint32_t penalty_points) {
            auto it = state_.validators.find(validator_id);
            if (it != state_.validators.end()) {
                auto& validator = it->second;
                validator.reputation_score = std::max(0u, validator.reputation_score - penalty_points);
                validator.missed_slots++;
                
                // Deactivate validator if reputation drops too low
                if (validator.reputation_score < 10) {
                    validator.is_active = false;
                }
            }
        }

        void HybridConsensusEngine::reward_validator(const Hash256& validator_id, uint32_t reward_points) {
            auto it = state_.validators.find(validator_id);
            if (it != state_.validators.end()) {
                auto& validator = it->second;
                validator.reputation_score = std::min(100u, validator.reputation_score + reward_points);
                
                // Reactivate validator if reputation improves
                if (validator.reputation_score >= 50 && !validator.is_active) {
                    validator.is_active = true;
                }
            }
        }

        // HybridDifficultyAdjustment implementation
        uint32_t HybridDifficultyAdjustment::calculate_hybrid_difficulty(
            uint32_t current_difficulty,
            uint64_t actual_time_span,
            uint64_t target_time_span,
            double pow_ratio_last_period,
            double target_pow_ratio
        ) {
            // First apply standard difficulty adjustment
            uint32_t adjusted_difficulty = DifficultyAdjustment::calculate_next_difficulty(
                current_difficulty, actual_time_span, target_time_span
            );
            
            // Then adjust based on PoW/PoS ratio
            if (pow_ratio_last_period < target_pow_ratio) {
                // Too few PoW blocks, decrease difficulty to encourage PoW
                double ratio_factor = target_pow_ratio / std::max(0.1, pow_ratio_last_period);
                ratio_factor = std::min(2.0, ratio_factor); // Limit adjustment
                
                // Decrease difficulty (increase target)
                adjusted_difficulty = static_cast<uint32_t>(adjusted_difficulty * ratio_factor);
            } else if (pow_ratio_last_period > target_pow_ratio) {
                // Too many PoW blocks, increase difficulty slightly
                double ratio_factor = pow_ratio_last_period / target_pow_ratio;
                ratio_factor = std::min(1.5, ratio_factor); // Limit adjustment
                
                // Increase difficulty (decrease target)
                adjusted_difficulty = static_cast<uint32_t>(adjusted_difficulty / ratio_factor);
            }
            
            // Ensure difficulty stays within valid bounds
            if (!DifficultyAdjustment::is_valid_difficulty_target(adjusted_difficulty)) {
                return current_difficulty; // Keep current if invalid
            }
            
            return adjusted_difficulty;
        }

        uint64_t HybridDifficultyAdjustment::calculate_pos_slot_interval(
            uint32_t active_validators,
            double participation_rate,
            uint64_t target_block_time
        ) {
            if (active_validators == 0 || participation_rate <= 0.0) {
                return target_block_time;
            }
            
            // Base interval adjusted for validator count and participation
            double base_interval = static_cast<double>(target_block_time) * POS_TARGET_RATIO;
            double validator_factor = 1.0 / static_cast<double>(active_validators);
            double participation_factor = 1.0 / std::max(0.1, participation_rate);
            
            uint64_t pos_interval = static_cast<uint64_t>(base_interval * validator_factor * participation_factor);
            
            // Ensure reasonable bounds (1 minute to 30 minutes)
            return std::max(60UL, std::min(1800UL, pos_interval));
        }

    } // namespace consensus
} // namespace blockchain