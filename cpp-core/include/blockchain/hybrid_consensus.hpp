#pragma once

#include "blockchain/consensus.hpp"
#include "blockchain/storage.hpp"
#include <map>
#include <set>
#include <random>

namespace blockchain {
    namespace consensus {

        /// Validator information for Proof-of-Stake
        struct Validator {
            crypto::Hash256 validator_id;       // Public key hash
            uint64_t stake_amount = 0;           // Amount of stake
            uint64_t last_block_time = 0;        // Last time this validator created a block
            uint32_t reputation_score = 100;     // Reputation (0-100)
            bool is_active = true;               // Whether validator is active
            crypto::PublicKey public_key;        // Validator's public key
            uint32_t total_blocks_created = 0;   // Total blocks created by this validator
            uint32_t missed_slots = 0;           // Number of missed validation slots
        };

        /// Stake information
        struct StakeEntry {
            crypto::Hash256 validator_id;
            uint64_t amount;
            uint32_t lock_height;      // Block height when stake can be unlocked
            bool is_locked = true;
        };

        /// Block production slot for PoS
        struct BlockSlot {
            crypto::Hash256 validator_id;
            uint64_t slot_time;        // Unix timestamp for this slot
            uint32_t block_height;     // Expected block height
            uint64_t stake_weight;     // Stake weight for this slot
        };

        /// Fork information for chain reorganization
        struct ForkInfo {
            crypto::Hash256 fork_point_hash;
            uint32_t fork_height;
            std::vector<crypto::Hash256> main_chain_blocks;
            std::vector<crypto::Hash256> alternative_chain_blocks;
            uint64_t main_chain_work = 0;
            uint64_t alternative_chain_work = 0;
        };

        /// Consensus state for hybrid PoW/PoS
        struct ConsensusState {
            uint32_t current_height = 0;
            crypto::Hash256 best_block_hash;
            uint64_t total_chain_work = 0;
            uint32_t current_difficulty = 0x1d00ffff;  // Initial difficulty
            
            // PoS state
            std::map<crypto::Hash256, Validator> validators;
            std::map<crypto::Hash256, StakeEntry> stakes;
            std::vector<BlockSlot> upcoming_slots;
            uint64_t total_stake = 0;
            
            // Consensus parameters
            uint64_t min_stake_amount = 1000000;    // Minimum stake to become validator
            uint32_t stake_maturity_blocks = 100;   // Blocks before stake is active
            uint32_t pos_activation_height = 1000;  // Height when PoS begins
            double pos_weight_ratio = 0.5;          // Weight of PoS vs PoW (0.5 = 50/50)
        };

        /// Hybrid PoW/PoS consensus engine
        class HybridConsensusEngine {
        private:
            ConsensusState state_;
            ProofOfWorkMiner pow_miner_;
            storage::IBlockchainStorage* storage_;
            std::mt19937_64 rng_;
            
            // PoS validator selection
            crypto::Hash256 select_validator_by_stake(uint64_t slot_time, const crypto::Hash256& previous_block_hash);
            double calculate_validator_selection_weight(const Validator& validator, uint64_t slot_time);
            
            // Chain validation and reorganization
            bool validate_chain_segment(const std::vector<crypto::Hash256>& block_hashes);
            uint64_t calculate_chain_work(const std::vector<crypto::Hash256>& block_hashes);
            bool should_reorganize_chain(const ForkInfo& fork_info);
            
            // Stake management
            void update_validator_stakes(uint32_t current_height);
            bool is_validator_eligible(const crypto::Hash256& validator_id, uint64_t slot_time);
            
            // Difficulty and slot time adjustment
            void adjust_consensus_parameters(uint32_t block_height, uint64_t block_time);
            uint64_t calculate_next_slot_time(uint64_t current_time, uint32_t block_height);

        public:
            explicit HybridConsensusEngine(storage::IBlockchainStorage* storage);
            ~HybridConsensusEngine() = default;

            /// Initialize consensus engine with genesis state
            bool initialize_genesis(const ConsensusState& genesis_state);
            
            /// Add a new validator to the network
            bool add_validator(const crypto::Hash256& validator_id, 
                             const crypto::PublicKey& public_key, 
                             uint64_t stake_amount);
            
            /// Remove validator (after stake unlock period)
            bool remove_validator(const crypto::Hash256& validator_id);
            
            /// Update validator stake
            bool update_stake(const crypto::Hash256& validator_id, uint64_t new_stake_amount);
            
            /// Generate upcoming block slots for PoS
            std::vector<BlockSlot> generate_upcoming_slots(uint64_t from_time, uint32_t slot_count);
            
            /// Validate a block using hybrid consensus rules
            bool validate_block(const crypto::Hash256& block_hash, 
                              const std::string& block_data, 
                              uint32_t nonce, 
                              const crypto::Hash256& validator_id);
            
            /// Mine a block using PoW (if PoW slot)
            MiningResult mine_pow_block(const std::string& block_data, uint32_t difficulty_target);
            
            /// Create a block using PoS (if validator is selected)
            bool create_pos_block(const crypto::Hash256& validator_id, 
                                const std::string& block_data,
                                crypto::Hash256& block_hash);
            
            /// Handle potential chain reorganization
            bool handle_chain_reorganization(const crypto::Hash256& new_block_hash);
            
            /// Get current consensus state
            const ConsensusState& get_consensus_state() const { return state_; }
            
            /// Update consensus state with new block
            bool update_consensus_state(const crypto::Hash256& block_hash, 
                                      uint32_t block_height, 
                                      uint64_t block_time,
                                      bool is_pow_block);
            
            /// Calculate expected reward for block
            uint64_t calculate_block_reward(uint32_t block_height, bool is_pow_block);
            
            /// Verify validator signature for PoS block
            bool verify_pos_signature(const crypto::Hash256& block_hash,
                                    const crypto::Signature& signature,
                                    const crypto::Hash256& validator_id);
            
            /// Get validator by ID
            std::optional<Validator> get_validator(const crypto::Hash256& validator_id) const;
            
            /// Get all active validators
            std::vector<Validator> get_active_validators() const;
            
            /// Calculate network statistics
            struct NetworkStats {
                uint32_t total_validators = 0;
                uint32_t active_validators = 0;
                uint64_t total_network_stake = 0;
                double average_block_time = 0.0;
                uint32_t pow_blocks_last_100 = 0;
                uint32_t pos_blocks_last_100 = 0;
                double network_hash_rate = 0.0;
                uint32_t current_difficulty = 0;
            };
            
            NetworkStats calculate_network_stats() const;
            
            /// Penalty system for misbehaving validators
            void penalize_validator(const crypto::Hash256& validator_id, uint32_t penalty_points);
            void reward_validator(const crypto::Hash256& validator_id, uint32_t reward_points);
        };

        /// Fork resolution and chain reorganization manager
        class ForkResolver {
        private:
            storage::IBlockchainStorage* storage_;
            
        public:
            explicit ForkResolver(storage::IBlockchainStorage* storage);
            
            /// Detect potential forks in the blockchain
            std::vector<ForkInfo> detect_forks(const crypto::Hash256& current_best_block);
            
            /// Resolve fork by choosing the heaviest chain
            bool resolve_fork(const ForkInfo& fork_info, crypto::Hash256& new_best_block);
            
            /// Calculate cumulative chain work (PoW + PoS weight)
            uint64_t calculate_cumulative_work(const std::vector<crypto::Hash256>& chain);
            
            /// Validate alternative chain
            bool validate_alternative_chain(const std::vector<crypto::Hash256>& chain);
        };

        /// Advanced difficulty adjustment with hybrid considerations
        class HybridDifficultyAdjustment : public DifficultyAdjustment {
        private:
            static constexpr double POW_TARGET_RATIO = 0.6;  // 60% PoW blocks target
            static constexpr double POS_TARGET_RATIO = 0.4;  // 40% PoS blocks target
            
        public:
            /// Calculate difficulty considering PoW/PoS ratio
            static uint32_t calculate_hybrid_difficulty(
                uint32_t current_difficulty,
                uint64_t actual_time_span,
                uint64_t target_time_span,
                double pow_ratio_last_period,
                double target_pow_ratio = POW_TARGET_RATIO
            );
            
            /// Adjust PoS slot timing based on validator participation
            static uint64_t calculate_pos_slot_interval(
                uint32_t active_validators,
                double participation_rate,
                uint64_t target_block_time = 600
            );
        };

    } // namespace consensus
} // namespace blockchain