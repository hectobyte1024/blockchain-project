# Blockchain TODO List - Improvements & Missing Features

**Generated**: November 25, 2025  
**Status**: Production-ready base, improvements needed for full decentralization

---

## 📊 Current Blockchain Storage Architecture

### **Where is the blockchain stored?**

Your blockchain is currently stored in **SQLite database** at:
- **Location**: `./edunet-gui/edunet.db`
- **Schema**: `edunet-gui/migrations/002_production_schema.sql`
- **Storage Format**: Relational database (NOT disk files like Bitcoin)

#### **Current Storage Tables:**

```sql
blocks (
    height INTEGER PRIMARY KEY,           -- Block number (0, 1, 2, ...)
    block_hash TEXT UNIQUE NOT NULL,      -- SHA256 hash
    prev_hash TEXT NOT NULL,              -- Previous block hash
    merkle_root TEXT NOT NULL,            -- Merkle tree root
    timestamp INTEGER NOT NULL,           -- Unix timestamp
    nonce INTEGER NOT NULL,               -- Proof-of-work nonce
    difficulty INTEGER NOT NULL,          -- Difficulty target
    block_data BLOB NOT NULL              -- Serialized block bytes
)

transactions (
    tx_hash TEXT UNIQUE,                  -- Transaction hash
    from_address TEXT,                    -- Sender address
    to_address TEXT,                      -- Recipient address
    amount INTEGER,                       -- Amount in satoshis
    block_height INTEGER,                 -- Which block contains this
    status TEXT                           -- pending/confirmed/failed
)

utxos (
    tx_hash TEXT,                         -- Transaction containing UTXO
    output_index INTEGER,                 -- Output index
    address TEXT,                         -- Owner address
    amount INTEGER,                       -- UTXO value
    is_spent BOOLEAN                      -- Has it been spent?
)
```

#### **What's Missing vs Bitcoin:**

| Feature | Bitcoin | Your System | Status |
|---------|---------|-------------|--------|
| **Block Storage** | Disk files (`blk00000.dat`, `blk00001.dat`) | SQLite database | ⚠️ Works but not scalable |
| **Block Index** | LevelDB index | SQLite index | ⚠️ Works but slower |
| **UTXO Set** | LevelDB chainstate | SQLite table | ⚠️ Works but no pruning |
| **Size Limit** | Unlimited (terabytes) | Limited by SQLite (~140TB theoretical, ~1TB practical) | ⚠️ Fine for now |
| **Performance** | Optimized for sequential I/O | Optimized for queries | ⚠️ Slower for sync |

**Current Status**: ✅ **Functional for small networks** (< 1 million blocks)  
**Problem**: Not optimized for Bitcoin-scale blockchain download/sync

---

## 🚀 PRIORITY 1: Blockchain Synchronization (CRITICAL)

**Problem**: Nodes can't download the blockchain from peers. Each node starts empty.

### 1.1 Initial Block Download (IBD) Implementation

**Location**: Create `rust-system/blockchain-core/src/sync.rs`

**Tasks**:
- [ ] **Create sync.rs module** (~500 lines)
  - [ ] `initial_block_download()` - Main IBD algorithm
  - [ ] `get_network_height()` - Query peers for chain height
  - [ ] `request_blocks()` - Request block range from peer
  - [ ] `download_block()` - Download single block with retry
  - [ ] `validate_downloaded_block()` - Verify block before applying
  - [ ] `is_synced()` - Check if node is synced with network
  - [ ] `get_sync_progress()` - Calculate % complete
  - [ ] `handle_sync_failure()` - Retry logic for failed downloads

**Algorithm**:
```rust
async fn initial_block_download() -> Result<()> {
    // 1. Query 10+ peers for their chain height
    let network_height = get_network_height_consensus().await?;
    
    // 2. Get our current height from database
    let local_height = database.get_blockchain_height().await?;
    
    // 3. Download missing blocks in batches
    for batch_start in (local_height..network_height).step_by(500) {
        let batch_end = (batch_start + 500).min(network_height);
        
        // 4. Download 500 blocks in parallel from multiple peers
        let blocks = download_block_batch(batch_start..batch_end).await?;
        
        // 5. Validate each block
        for block in blocks {
            consensus.validate_block(&block).await?;
            consensus.apply_block(block).await?;
            database.save_block(&block).await?;
        }
        
        // 6. Update progress
        log_sync_progress(batch_end, network_height);
    }
    
    Ok(())
}
```

**Files to Modify**:
- [ ] Create `rust-system/blockchain-core/src/sync.rs`
- [ ] Update `rust-system/blockchain-core/src/lib.rs` to export sync module
- [ ] Modify `edunet-gui/src/blockchain_integration.rs`:
  ```rust
  pub async fn sync_blockchain(&self) -> Result<()> {
      self.sync_engine.initial_block_download().await
  }
  ```
- [ ] Modify `edunet-gui/src/main.rs`:
  ```rust
  // After network initialization
  info!("🔄 Starting blockchain synchronization...");
  backend.sync_blockchain().await?;
  info!("✅ Blockchain synced to height {}", backend.get_height().await?);
  ```

**Estimated Time**: 8-12 hours  
**Difficulty**: High  
**Impact**: Critical - enables full node functionality

---

### 1.2 Protocol Messages for Sync

**Location**: `rust-system/blockchain-network/src/protocol.rs`

**Tasks**:
- [ ] **Add sync-specific message types** to `MessageType` enum:
  ```rust
  pub enum MessageType {
      // Existing messages
      Version, VerAck, Ping, Pong, GetBlocks, Block, Tx,
      
      // NEW sync messages
      GetBlockchainHeight,    // Request peer's chain height
      BlockchainHeight,       // Response with height
      GetBlockByHeight,       // Request specific block
      BlockData,              // Block response
      GetHeaders,             // Request block headers only
      Headers,                // Headers response (for header-first sync)
      NotFound,               // Block not found response
  }
  ```

- [ ] **Implement message handlers** in `rust-system/blockchain-network/src/swarm.rs`:
  ```rust
  async fn handle_get_blockchain_height(&mut self, peer_id: &PeerId) {
      let height = self.consensus.get_chain_state().await.height;
      self.send_blockchain_height(peer_id, height).await;
  }
  
  async fn handle_get_block_by_height(&mut self, peer_id: &PeerId, height: u64) {
      if let Some(block) = self.storage.get_block_at_height(height).await {
          self.send_block_data(peer_id, block).await;
      } else {
          self.send_not_found(peer_id, height).await;
      }
  }
  ```

**Files to Modify**:
- [ ] `rust-system/blockchain-network/src/protocol.rs` - Add message types
- [ ] `rust-system/blockchain-network/src/swarm.rs` - Add handlers
- [ ] `rust-system/blockchain-network/src/lib.rs` - Export new functions

**Estimated Time**: 4-6 hours  
**Difficulty**: Medium  
**Impact**: Critical - required for IBD

---

### 1.3 Disk-Based Block Storage (Bitcoin-style)

**Problem**: SQLite is slow for sequential block I/O during sync.

**Location**: Create `rust-system/blockchain-core/src/storage.rs`

**Tasks**:
- [ ] **Implement blk*.dat file storage**:
  ```rust
  pub struct DiskBlockStorage {
      data_dir: PathBuf,
      current_file: File,
      current_file_num: u32,
      block_index: HashMap<Hash256, BlockLocation>,
  }
  
  struct BlockLocation {
      file_num: u32,      // Which blk*.dat file
      offset: u64,        // Byte offset in file
      size: u32,          // Block size in bytes
  }
  ```

- [ ] **Write blocks to disk sequentially**:
  ```rust
  async fn write_block(&mut self, block: &Block) -> Result<BlockLocation> {
      // Serialize block
      let block_bytes = serialize_block(block)?;
      
      // Check if need new file (max 2GB per file)
      if self.current_file.metadata()?.len() > 2_000_000_000 {
          self.rotate_file()?;
      }
      
      // Append to current file
      let offset = self.current_file.seek(SeekFrom::End(0))?;
      self.current_file.write_all(&block_bytes)?;
      
      Ok(BlockLocation {
          file_num: self.current_file_num,
          offset,
          size: block_bytes.len() as u32,
      })
  }
  ```

- [ ] **Read blocks from disk**:
  ```rust
  async fn read_block(&self, location: &BlockLocation) -> Result<Block> {
      let file_path = self.data_dir.join(format!("blk{:05}.dat", location.file_num));
      let mut file = File::open(file_path)?;
      file.seek(SeekFrom::Start(location.offset))?;
      
      let mut buffer = vec![0u8; location.size as usize];
      file.read_exact(&mut buffer)?;
      
      deserialize_block(&buffer)
  }
  ```

**Files to Create**:
- [ ] `rust-system/blockchain-core/src/storage.rs`
- [ ] `rust-system/blockchain-core/src/storage/disk.rs` - Disk I/O
- [ ] `rust-system/blockchain-core/src/storage/index.rs` - Block index (LevelDB or RocksDB)

**Files to Modify**:
- [ ] `edunet-gui/src/blockchain_integration.rs` - Use disk storage instead of SQLite for blocks
- [ ] Keep SQLite for: users, transactions metadata, NFTs, loans (application data)
- [ ] Use disk files for: raw block data (high volume, sequential access)

**Estimated Time**: 12-16 hours  
**Difficulty**: High  
**Impact**: High - improves sync performance 10-100x

---

### 1.4 Node Modes (FullNode / LightClient / PruneNode)

**Location**: `rust-system/blockchain-core/src/node_mode.rs`

**Tasks**:
- [ ] **Define node modes**:
  ```rust
  pub enum NodeMode {
      FullNode,      // Download and store entire blockchain
      LightClient,   // Only download block headers + relevant transactions
      PruneNode,     // Store recent blocks, prune old ones (keep UTXO set)
  }
  ```

- [ ] **Add CLI flags**:
  ```bash
  ./edunet-gui --full-node        # Default: full blockchain
  ./edunet-gui --light            # Light client mode
  ./edunet-gui --prune            # Pruned node (save disk space)
  ```

- [ ] **Implement mode-specific behavior**:
  ```rust
  impl NodeMode {
      fn should_download_block(&self, height: u64, current_height: u64) -> bool {
          match self {
              FullNode => true,  // Download everything
              LightClient => false,  // Only headers
              PruneNode => (current_height - height) < 1000,  // Keep last 1000 blocks
          }
      }
  }
  ```

**Files to Create**:
- [ ] `rust-system/blockchain-core/src/node_mode.rs`

**Files to Modify**:
- [ ] `edunet-gui/src/main.rs` - Add `--light` and `--prune` arguments
- [ ] `edunet-gui/src/blockchain_integration.rs` - Pass mode to sync engine
- [ ] `rust-system/blockchain-core/src/sync.rs` - Respect node mode during sync

**Estimated Time**: 4-6 hours  
**Difficulty**: Medium  
**Impact**: Medium - provides flexibility for users

---

### 1.5 Sync Progress Tracking & UI

**Location**: `edunet-gui/src/blockchain_integration.rs`

**Tasks**:
- [ ] **Add sync status endpoint**:
  ```rust
  pub async fn get_sync_status(&self) -> SyncStatus {
      SyncStatus {
          is_syncing: self.is_syncing.load(Ordering::Relaxed),
          local_height: self.consensus.get_chain_state().await.height,
          network_height: self.network_height.load(Ordering::Relaxed),
          progress_percent: calculate_progress(),
          blocks_per_second: self.sync_speed.load(Ordering::Relaxed),
          eta_seconds: calculate_eta(),
      }
  }
  ```

- [ ] **Add API endpoint**: `GET /api/blockchain/sync-status`
- [ ] **Update dashboard** to show sync progress bar
- [ ] **Log sync milestones**:
  ```
  🔄 Syncing blockchain: 12.5% (25,000 / 200,000 blocks)
  🔄 Syncing blockchain: 25.0% (50,000 / 200,000 blocks)
  🔄 Syncing blockchain: 50.0% (100,000 / 200,000 blocks)
  ✅ Blockchain sync complete! Height: 200,000
  ```

**Files to Modify**:
- [ ] `edunet-gui/src/blockchain_integration.rs` - Add sync status methods
- [ ] `edunet-gui/src/main.rs` - Add sync status endpoint
- [ ] `edunet-gui/templates/dashboard.html` - Add progress bar UI

**Estimated Time**: 3-4 hours  
**Difficulty**: Low  
**Impact**: Medium - improves user experience

---

## 🔐 PRIORITY 2: Consensus Improvements

### 2.1 Add Slashing Mechanism (Security Critical)

**Problem**: Validators can behave maliciously without penalty.

**Location**: `cpp-core/src/consensus/hybrid_consensus.cpp`

**Tasks**:
- [ ] **Detect double-signing** (validator creates two conflicting blocks):
  ```cpp
  bool detect_double_signing(const Hash256& validator_id, 
                            const Hash256& block_hash_1, 
                            const Hash256& block_hash_2) {
      // If same validator signed two blocks at same height
      if (block_height_1 == block_height_2 && block_hash_1 != block_hash_2) {
          return true;  // Double signing detected!
      }
      return false;
  }
  ```

- [ ] **Slash stake** when detected:
  ```cpp
  void slash_validator(const Hash256& validator_id, double slash_percent) {
      auto validator = get_validator(validator_id);
      uint64_t slashed_amount = validator->stake_amount * slash_percent;
      
      // Remove slashed stake
      validator->stake_amount -= slashed_amount;
      state_.total_stake -= slashed_amount;
      
      // Burn or redistribute slashed coins
      burn_coins(slashed_amount);  // or redistribute_to_honest_validators()
      
      // Deactivate validator
      validator->is_active = false;
      
      tracing::error!("⚠️ Slashed validator {} for misbehavior: -{} coins", 
                     hex::encode(validator_id), slashed_amount);
  }
  ```

- [ ] **Slash amounts**:
  - Double signing: **100%** of stake (severe)
  - Offline for extended period: **5%** of stake (mild)
  - Invalid block creation: **50%** of stake (moderate)

**Files to Modify**:
- [ ] `cpp-core/include/blockchain/hybrid_consensus.hpp` - Add slashing methods
- [ ] `cpp-core/src/consensus/hybrid_consensus.cpp` - Implement detection & slashing
- [ ] `rust-system/blockchain-core/src/consensus.rs` - Expose slashing to Rust layer

**Estimated Time**: 6-8 hours  
**Difficulty**: High  
**Impact**: Critical - prevents 51% attacks and validator abuse

---

### 2.2 Verifiable Random Function (VRF) for Validator Selection

**Problem**: Current selection uses simple hash-based randomness (predictable).

**Location**: `cpp-core/src/consensus/hybrid_consensus.cpp`

**Tasks**:
- [ ] **Integrate VRF library** (e.g., libsodium's VRF or BLS signatures)
- [ ] **Replace hash-based selection** with VRF:
  ```cpp
  Hash256 select_validator_by_stake_vrf(uint64_t slot_time) {
      std::vector<std::pair<Hash256, VRFProof>> validator_vrf_outputs;
      
      // Each validator computes VRF
      for (const auto& [validator_id, validator] : state_.validators) {
          // VRF input: slot_time + previous_block_hash + validator_secret_key
          VRFProof proof = validator.compute_vrf(slot_time, state_.best_block_hash);
          validator_vrf_outputs.push_back({validator_id, proof});
      }
      
      // Verify all proofs and select lowest output (provably random)
      Hash256 winner = select_lowest_vrf_output(validator_vrf_outputs);
      return winner;
  }
  ```

- [ ] **Benefits**:
  - Unpredictable: No one knows who will be selected until the slot
  - Verifiable: Everyone can verify the selection was fair
  - Non-interactive: No communication needed between validators

**Files to Modify**:
- [ ] `cpp-core/CMakeLists.txt` - Add VRF library dependency
- [ ] `cpp-core/include/blockchain/crypto.hpp` - Add VRF functions
- [ ] `cpp-core/src/crypto/crypto.cpp` - Implement VRF
- [ ] `cpp-core/src/consensus/hybrid_consensus.cpp` - Use VRF for selection

**Estimated Time**: 8-12 hours  
**Difficulty**: High  
**Impact**: High - improves security and unpredictability

---

### 2.3 Randomize PoW/PoS Slot Assignment

**Problem**: Alternating slots (`i % 2`) are predictable.

**Location**: `cpp-core/src/consensus/hybrid_consensus.cpp` (line 169)

**Current Code**:
```cpp
bool is_pos_slot = (i % 2 == 0);  // Predictable: PoS, PoW, PoS, PoW...
```

**Tasks**:
- [ ] **Replace with weighted random**:
  ```cpp
  bool is_pos_slot = (deterministic_random(slot_time, prev_hash) < 0.4);  // 40% PoS, 60% PoW
  ```

- [ ] **Implement deterministic random**:
  ```cpp
  double deterministic_random(uint64_t slot_time, const Hash256& prev_hash) {
      std::string seed = std::to_string(slot_time) + std::string(prev_hash.begin(), prev_hash.end());
      Hash256 hash = SHA256::hash(seed);
      
      // Convert first 8 bytes to 0.0-1.0 range
      uint64_t value = 0;
      for (int i = 0; i < 8; ++i) {
          value = (value << 8) | hash[i];
      }
      return static_cast<double>(value) / static_cast<double>(UINT64_MAX);
  }
  ```

**Files to Modify**:
- [ ] `cpp-core/src/consensus/hybrid_consensus.cpp` (line 169)

**Estimated Time**: 1-2 hours  
**Difficulty**: Low  
**Impact**: Medium - reduces predictability

---

### 2.4 Minimum Online Time for Validators

**Problem**: Validators can join and immediately start validating.

**Location**: `cpp-core/src/consensus/hybrid_consensus.cpp`

**Tasks**:
- [ ] **Track validator registration time**:
  ```cpp
  struct Validator {
      uint64_t registration_time;
      uint64_t total_online_time;
      // ... existing fields
  };
  ```

- [ ] **Require minimum online time**:
  ```cpp
  bool is_validator_eligible(const Hash256& validator_id, uint64_t slot_time) {
      auto validator = get_validator(validator_id);
      
      // Must be online for at least 24 hours
      const uint64_t MIN_ONLINE_TIME = 86400;  // 24 hours in seconds
      uint64_t time_since_registration = slot_time - validator->registration_time;
      
      if (time_since_registration < MIN_ONLINE_TIME) {
          return false;  // Too new
      }
      
      return true;
  }
  ```

**Files to Modify**:
- [ ] `cpp-core/include/blockchain/hybrid_consensus.hpp` - Add time fields
- [ ] `cpp-core/src/consensus/hybrid_consensus.cpp` - Enforce minimum time

**Estimated Time**: 2-3 hours  
**Difficulty**: Low  
**Impact**: Low - prevents spam validators

---

## 📈 PRIORITY 3: Performance & Scalability

### 3.1 Replace SQLite with LevelDB/RocksDB for Block Index

**Tasks**:
- [ ] Integrate RocksDB for key-value storage
- [ ] Migrate block index from SQLite to RocksDB
- [ ] Keep SQLite for application data (users, loans, NFTs)
- [ ] Benchmark performance improvement

**Estimated Time**: 8-10 hours  
**Difficulty**: Medium  
**Impact**: High - 10-50x faster lookups

---

### 3.2 UTXO Set Pruning

**Tasks**:
- [ ] Track spent UTXOs
- [ ] Prune spent UTXOs older than 1000 blocks
- [ ] Keep unspent UTXOs forever
- [ ] Reduce database size by 60-80%

**Estimated Time**: 4-6 hours  
**Difficulty**: Medium  
**Impact**: Medium - saves disk space

---

### 3.3 Block Caching Layer

**Tasks**:
- [ ] Add LRU cache for recent blocks (last 100 blocks)
- [ ] Cache in memory for faster API responses
- [ ] Reduce database queries by 80%

**Estimated Time**: 3-4 hours  
**Difficulty**: Low  
**Impact**: Medium - faster API responses

---

## 🌐 PRIORITY 4: Networking Enhancements

### 4.1 Multiple Bootstrap Nodes (DNS Seeds)

**Current**: Single bootstrap node (hardcoded)  
**Needed**: Multiple DNS seed nodes for redundancy

**Tasks**:
- [ ] Add 5-10 bootstrap node addresses
- [ ] Implement fallback logic if one fails
- [ ] Add to `rust-system/blockchain-network/src/discovery.rs`

**Estimated Time**: 2-3 hours  
**Difficulty**: Low  
**Impact**: High - prevents single point of failure

---

### 4.2 Peer Scoring and Ban System

**Tasks**:
- [ ] Track peer behavior (response time, invalid blocks sent)
- [ ] Score peers (0-100)
- [ ] Ban peers who send invalid data
- [ ] Prefer high-scoring peers for block requests

**Estimated Time**: 6-8 hours  
**Difficulty**: Medium  
**Impact**: Medium - improves sync reliability

---

## 🧪 PRIORITY 5: Testing & Validation

### 5.1 Integration Tests

**Tasks**:
- [ ] Test 3-node network sync
- [ ] Test validator slashing
- [ ] Test fork resolution
- [ ] Test block download failure recovery

**Estimated Time**: 8-12 hours  
**Difficulty**: Medium  
**Impact**: High - ensures reliability

---

### 5.2 Stress Testing

**Tasks**:
- [ ] Test with 1 million blocks
- [ ] Test with 10,000 transactions/block
- [ ] Test with 100 concurrent peers
- [ ] Measure sync time, memory usage, CPU usage

**Estimated Time**: 4-6 hours  
**Difficulty**: Low  
**Impact**: Medium - identifies bottlenecks

---

## 📋 Summary

### Critical Path (Must Have for Full Node):
1. **Blockchain Sync (IBD)** - 8-12 hours
2. **Sync Protocol Messages** - 4-6 hours
3. **Disk Block Storage** - 12-16 hours
4. **Slashing Mechanism** - 6-8 hours

**Total Critical**: ~40-50 hours of development

### Nice to Have (Future Enhancements):
- VRF for validators - 8-12 hours
- RocksDB migration - 8-10 hours
- UTXO pruning - 4-6 hours
- Comprehensive testing - 12-18 hours

**Total Nice to Have**: ~32-46 hours

### Grand Total: 72-96 hours (9-12 days of full-time work)

---

## ✅ What You Already Have (Working)

- ✅ **Hybrid PoW/PoS consensus** (production-grade)
- ✅ **Multi-threaded mining** (efficient)
- ✅ **P2P network layer** (fully implemented)
- ✅ **Block broadcasting** (works)
- ✅ **Transaction validation** (UTXO model)
- ✅ **Difficulty adjustment** (Bitcoin-style)
- ✅ **SQLite storage** (functional for small scale)
- ✅ **Web UI** (complete)
- ✅ **User authentication** (working)

**Your system is 70% complete for production use.**  
**The 30% gap is blockchain synchronization and consensus hardening.**
