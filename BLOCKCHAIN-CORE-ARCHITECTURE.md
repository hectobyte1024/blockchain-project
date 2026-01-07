# EduNet Blockchain - Separated Architecture Documentation
## Production-Ready Blockchain Node + Web Client System

---

## Table of Contents
1. [Architecture Overview](#architecture-overview)
2. [System Components](#system-components)
3. [Complete Transaction Flow](#complete-transaction-flow)
4. [Voucher System](#voucher-system)
5. [RPC Communication Layer](#rpc-communication-layer)
6. [Mining & Consensus](#mining-consensus)
7. [Wallet & Authentication](#wallet-authentication)
8. [Deployment Guide](#deployment-guide)

---

## Architecture Overview

### Separated Design Philosophy
The system is split into two independent services for scalability:
- **blockchain-node**: Mining daemon (can run on dedicated servers)
- **edunet-web**: Web client (connects to node via RPC)

```
┌─────────────────────────────────────────────────────────────────┐
│                    BROWSER (User Interface)                      │
│  Future: React/Vue frontend                                     │
└─────────────────────────────────────────────────────────────────┘
                            ↓ HTTP API (REST)
┌─────────────────────────────────────────────────────────────────┐
│               EDUNET-WEB (Web Client Server)                     │
│  edunet-web/src/main.rs                                         │
│  • HTTP server (port 8080)                                      │
│  • User authentication (Argon2 + JWT)                           │
│  • Deterministic wallets (email:password derived)               │
│  • Transaction signing (HMAC-SHA256)                            │
│  • Voucher redemption system                                    │
│  • SQLite database (users, vouchers, sessions)                  │
└─────────────────────────────────────────────────────────────────┘
                            ↓ JSON-RPC (port 8545)
┌─────────────────────────────────────────────────────────────────┐
│           BLOCKCHAIN-RPC (Communication Layer)                   │
│  rust-system/blockchain-rpc/                                    │
│  • Client: RpcClient (edunet-web uses this)                     │
│  • Server: RpcServer (blockchain-node runs this)                │
│  • Methods: 10 total (balance, send, mining, credit, etc.)      │
└─────────────────────────────────────────────────────────────────┘
                            ↓ Method dispatch
┌─────────────────────────────────────────────────────────────────┐
│            BLOCKCHAIN-NODE (Mining Daemon)                       │
│  blockchain-node/src/main.rs                                    │
│  • RPC server (port 8545)                                       │
│  • P2P networking (port 9000)                                   │
│  • Mining loop (SHA256 PoW, difficulty 4)                       │
│  • Transaction processing (mempool → block → execute)           │
│  • Block rewards (50 EDU per block)                             │
│  • Shared state (Arc<Mutex<>> for balances, blocks, txs)        │
└─────────────────────────────────────────────────────────────────┘
                            ↓ State updates
┌─────────────────────────────────────────────────────────────────┐
│              IN-MEMORY STATE (Production: Add Persistence)       │
│  • Balances: HashMap<Address, u64>                              │
│  • Blocks: Vec<Block>                                           │
│  • Transactions: HashMap<TxHash, Transaction>                   │
│  • Mempool: Vec<Transaction>                                    │
└─────────────────────────────────────────────────────────────────┘
```

---

## System Components

### 1. blockchain-node (Mining Daemon)
**Location:** `blockchain-node/src/main.rs`
**Purpose:** Consensus layer - mining, validation, blockchain state
**Ports:** 
- 8545 (RPC server)
- 9000 (P2P networking)

**Key Features:**
- PoW mining with SHA256 (difficulty 4)
- 50 EDU block rewards every 10 seconds
- Transaction processing from mempool
- Balance state management
- RPC server for remote queries

**Startup Command:**
```bash
./target/release/blockchain-node --mining --validator-address "EDU_validator_miner"
```

**RPC Methods Provided:**
1. `blockchain_getBalance` - Query address balance
2. `blockchain_sendTransaction` - Submit signed transaction
3. `blockchain_getBlockHeight` - Get current block number
4. `blockchain_getBlock` - Get block by height
5. `blockchain_getTransaction` - Get transaction by hash
6. `blockchain_getTransactionStatus` - Check if tx is confirmed
7. `blockchain_getPeers` - List connected P2P peers
8. `blockchain_getMiningInfo` - Get mining statistics
9. `blockchain_getMempool` - View pending transactions
10. `blockchain_creditBalance` - Direct balance credit (for vouchers/airdrops)

---

### 2. edunet-web (Web Client)
**Location:** `edunet-web/src/main.rs`
**Purpose:** User-facing HTTP API, authentication, wallet management
**Port:** 8080 (HTTP)
**Database:** SQLite (`edunet-web.db`)

**Key Features:**
- User registration with deterministic wallets
- Argon2 password hashing
- JWT session tokens (24h expiry)
- Wallet generation from email:password (user-recoverable)
- Transaction signing with HMAC-SHA256
- Voucher redemption system
- RPC client for blockchain queries

**Startup Command:**
```bash
./target/release/edunet-web --node-rpc "http://127.0.0.1:8545" --port 8080
```

**HTTP API Endpoints:**
- `POST /api/auth/register` - Create account
- `POST /api/auth/login` - Get JWT token
- `POST /api/auth/logout` - Invalidate session
- `GET /api/wallet/balance` - Check balance
- `POST /api/wallet/send` - Send transaction
- `GET /api/wallet/transactions` - Transaction history
- `POST /api/voucher/redeem` - Redeem voucher code
- `POST /api/voucher/generate` - Generate vouchers (admin)

**Database Schema:**
```sql
-- Users table
CREATE TABLE users (
    id INTEGER PRIMARY KEY,
    email TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    wallet_address TEXT UNIQUE NOT NULL,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP
);

-- Sessions table
CREATE TABLE sessions (
    id INTEGER PRIMARY KEY,
    user_id INTEGER NOT NULL,
    token TEXT UNIQUE NOT NULL,
    expires_at TEXT NOT NULL,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id)
);

-- Vouchers table
CREATE TABLE vouchers (
    code TEXT PRIMARY KEY,
    amount INTEGER NOT NULL DEFAULT 2000000000, -- 20 EDU in satoshis
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    redeemed_at TEXT,
    redeemed_by INTEGER,
    FOREIGN KEY (redeemed_by) REFERENCES users(id)
);
```

---

### 3. blockchain-rpc (Communication Layer)
**Location:** `rust-system/blockchain-rpc/`
**Purpose:** JSON-RPC protocol implementation
**Components:**
- `client.rs` - RPC client (used by edunet-web)
- `server.rs` - RPC server (used by blockchain-node)
- `lib.rs` - Shared types and method constants

**RPC Request Format:**
```json
{
  "jsonrpc": "2.0",
  "method": "blockchain_getBalance",
  "params": ["EDU23327d04d02229e70c2bbab1a0d1e0d98b806f09"],
  "id": 1
}
```

**RPC Response Format:**
```json
{
  "jsonrpc": "2.0",
  "result": 2000000000,
  "id": 1
}
```

---

## Complete Transaction Flow

### Scenario: Alice sends 5 EDU to Bob

#### **Step 1: User Authentication**

Alice logs in via `POST /api/auth/login`:
```json
{
  "email": "alice@edu.net",
  "password": "Alice123!"
}
```

**Backend Process (edunet-web/src/main.rs):**
```rust
async fn login_handler(Json(req): Json<LoginRequest>) -> Result<Json<Value>> {
    // 1. Query user from database
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = ?")
        .bind(&req.email)
        .fetch_one(&db).await?;
    
    // 2. Verify password with Argon2
    let parsed_hash = PasswordHash::new(&user.password_hash)?;
    Argon2::default().verify_password(req.password.as_bytes(), &parsed_hash)?;
    
    // 3. Generate JWT token (24h expiry)
    let token = generate_jwt_token(user.id);
    
    // 4. Store session in database
    sqlx::query("INSERT INTO sessions (user_id, token, expires_at) VALUES (?, ?, ?)")
        .bind(user.id)
        .bind(&token)
        .bind(expires_at)
        .execute(&db).await?;
    
    Ok(Json(json!({
        "success": true,
        "token": token,
        "wallet_address": user.wallet_address
    })))
}
```

**Result:** Alice receives JWT token and her wallet address

---

#### **Step 2: Send Transaction Request**

Alice submits transaction via `POST /api/wallet/send`:
```json
{
  "from_address": "EDU23327d04d02229e70c2bbab1a0d1e0d98b806f09",
  "to_address": "EDU0e9ab78774eedd68cad522346b9928565ba5a04b",
  "amount": 5.0,
  "seed_phrase": "alice@edu.net:Alice123!"
}
```

**Backend Process (edunet-web/src/main.rs):**
```rust
async fn send_transaction_handler(
    State(state): State<AppState>,
    Json(req): Json<SendTransactionRequest>
) -> Result<Json<Value>> {
    // 1. Regenerate wallet from seed phrase
    let wallet = Wallet::from_seed_phrase(&req.seed_phrase)?;
    
    // 2. Verify wallet matches from_address
    if wallet.address != req.from_address {
        return Err("Invalid credentials");
    }
    
    // 3. Convert EDU to satoshis
    let amount_satoshis = (req.amount * 100_000_000.0) as u64;
    
    // 4. Create unsigned transaction
    let nonce = get_nonce(&req.from_address).await;
    let unsigned_tx = UnsignedTransaction {
        from: req.from_address.clone(),
        to: req.to_address.clone(),
        amount: amount_satoshis,
        nonce,
        timestamp: current_timestamp(),
    };
    
    // 5. Sign transaction with HMAC-SHA256
    let tx_data = serde_json::to_string(&unsigned_tx)?;
    let signature = wallet.sign_transaction(&tx_data);
    
    // 6. Create signed transaction
    let signed_tx = SignedTransaction {
        from: unsigned_tx.from,
        to: unsigned_tx.to,
        amount: unsigned_tx.amount,
        nonce: unsigned_tx.nonce,
        timestamp: unsigned_tx.timestamp,
        signature,
        hash: calculate_tx_hash(&unsigned_tx),
    };
    
    // 7. Submit to blockchain via RPC
    let result = state.rpc_client
        .send_transaction(&signed_tx)
        .await?;
    
    Ok(Json(json!({
        "success": true,
        "tx_hash": result.tx_hash,
        "message": "Transaction submitted to network"
    })))
}
```

**Result:** Transaction signed and submitted with tx_hash

---

#### **Step 3: RPC Communication**

**edunet-web → blockchain-node RPC call:**

```rust
// In edunet-web (client side)
let rpc_client = RpcClient::new("http://127.0.0.1:8545");
let result = rpc_client.send_transaction(&signed_tx).await?;
```

**RPC Request over HTTP:**
```json
POST http://127.0.0.1:8545
{
  "jsonrpc": "2.0",
  "method": "blockchain_sendTransaction",
  "params": [{
    "from": "EDU23327d04d02229e70c2bbab1a0d1e0d98b806f09",
    "to": "EDU0e9ab78774eedd68cad522346b9928565ba5a04b",
    "amount": 500000000,
    "nonce": 0,
    "timestamp": 1702159200,
    "signature": "a3f5d...",
    "hash": "0x4c7f64de..."
  }],
  "id": 1
}
```

---

#### **Step 4: Transaction Processing in Node**

**Backend Process (blockchain-node/src/main.rs):**

```rust
// RPC handler receives transaction
async fn handle_send_transaction(
    tx: SignedTransaction,
    state: Arc<SharedState>
) -> Result<Value> {
    // 1. Validate transaction signature
    if !verify_hmac_signature(&tx) {
        return Err("Invalid signature");
    }
    
    // 2. Check sender balance
    let balances = state.balances.lock().unwrap();
    let sender_balance = balances.get(&tx.from).unwrap_or(&0);
    if *sender_balance < tx.amount {
        return Err("Insufficient balance");
    }
    drop(balances);
    
    // 3. Add to mempool
    let mut mempool = state.mempool.lock().unwrap();
    mempool.push(tx.clone());
    drop(mempool);
    
    // 4. Store transaction with pending status
    let mut transactions = state.transactions.lock().unwrap();
    transactions.insert(tx.hash.clone(), tx.clone());
    drop(transactions);
    
    Ok(json!({
        "tx_hash": tx.hash,
        "status": "pending"
    }))
}
```

**Result:** Transaction added to mempool, waiting for mining

---

#### **Step 5: Mining Loop**

**Backend Process (blockchain-node/src/miner.rs):**

```rust
pub async fn mining_loop(state: Arc<SharedState>, validator_address: String) {

## Voucher System

### Overview
The voucher system enables promotional distribution of EDU tokens (airdrops) without requiring users to mine or purchase tokens. Perfect for onboarding new users with initial EDU balance.

### Key Features
- ✅ One-time use vouchers with unique codes
- ✅ Customizable EDU amounts (default: 20 EDU)
- ✅ QR code generation for easy distribution
- ✅ Instant balance credit (no mining delay)
- ✅ Double-redemption prevention
- ✅ Audit trail (who redeemed when)

### Usage

**Generate 30 vouchers with 10 EDU each:**
```bash
./generate_voucher_pdf.sh 30 10
```

**Output files:**
- `vouchers_30x10_TIMESTAMP.json` - Voucher codes and data
- `voucher-qr-codes-TIMESTAMP/` - SVG QR codes for each voucher
- `voucher-qr-codes-TIMESTAMP/vouchers.html` - Print-ready HTML (4x7 grid)

**Redeem voucher:**
```bash
curl -X POST http://localhost:8080/api/voucher/redeem \
  -H "Content-Type: application/json" \
  -d '{"voucher_code":"EDUF30621959DAD","wallet_address":"EDU..."}'
```

---

## Deployment Guide

### Local Development Setup

**1. Start blockchain node with mining:**
```bash
cargo build --release --bin blockchain-node
./target/release/blockchain-node --mining --validator-address "EDU_validator_miner"
```

**2. Start web client:**
```bash
cargo build --release --bin edunet-web
./target/release/edunet-web --node-rpc "http://127.0.0.1:8545" --port 8080
```

**3. Test the system:**
```bash
# Register user
curl -X POST http://localhost:8080/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{"email":"alice@edu.net","password":"Alice123!"}'

# Generate voucher
curl -X POST http://localhost:8080/api/voucher/generate \
  -H "Content-Type: application/json" \
  -d '{"count":1,"amount":20}'

# Redeem voucher
curl -X POST http://localhost:8080/api/voucher/redeem \
  -H "Content-Type: application/json" \
  -d '{"voucher_code":"EDU...","wallet_address":"EDU..."}'

# Send transaction
curl -X POST http://localhost:8080/api/wallet/send \
  -H "Content-Type: application/json" \
  -d '{"from_address":"EDU...","to_address":"EDU...","amount":5.0,"seed_phrase":"alice@edu.net:Alice123!"}'
```

---

### Production Deployment

**Server Requirements:**
- Linux server (Debian/Ubuntu recommended)
- 2+ CPU cores
- 4+ GB RAM
- 50+ GB disk space
- Open ports: 8080 (HTTP), 8545 (RPC), 9000 (P2P)

**Architecture:**
```
Internet → Port 8080 → edunet-web → RPC → blockchain-node
                                      ↓
                                   Port 9000 → P2P Network
```

**Steps:**

1. **Install dependencies:**
```bash
sudo apt update
sudo apt install -y build-essential pkg-config libssl-dev sqlite3
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

2. **Clone and build:**
```bash
git clone <repo-url>
cd blockchain-project
cargo build --release
```

3. **Set up systemd services:**

`/etc/systemd/system/blockchain-node.service`:
```ini
[Unit]
Description=EduNet Blockchain Node
After=network.target

[Service]
Type=simple
User=blockchain
WorkingDirectory=/opt/edunet
ExecStart=/opt/edunet/target/release/blockchain-node --mining --validator-address "EDU_production_miner"
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

`/etc/systemd/system/edunet-web.service`:
```ini
[Unit]
Description=EduNet Web Client
After=network.target blockchain-node.service

[Service]
Type=simple
User=blockchain
WorkingDirectory=/opt/edunet
ExecStart=/opt/edunet/target/release/edunet-web --node-rpc http://127.0.0.1:8545 --port 8080
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

4. **Start services:**
```bash
sudo systemctl daemon-reload
sudo systemctl enable blockchain-node edunet-web
sudo systemctl start blockchain-node edunet-web
```

5. **Configure firewall:**
```bash
sudo ufw allow 8080/tcp  # HTTP API
sudo ufw allow 9000/tcp  # P2P
sudo ufw enable
```

6. **Set up reverse proxy (Nginx):**
```nginx
server {
    listen 80;
    server_name edunet.example.com;
    
    location / {
        proxy_pass http://127.0.0.1:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
```

---

## System Status & Monitoring

**Check services:**
```bash
sudo systemctl status blockchain-node
sudo systemctl status edunet-web
```

**View logs:**
```bash
sudo journalctl -u blockchain-node -f
sudo journalctl -u edunet-web -f
```

**Query blockchain:**
```bash
# Block height
curl -X POST http://localhost:8545 \
  -d '{"jsonrpc":"2.0","method":"blockchain_getBlockHeight","params":[],"id":1}'

# Check balance
curl -X POST http://localhost:8545 \
  -d '{"jsonrpc":"2.0","method":"blockchain_getBalance","params":["EDU..."],"id":1}'

# Mining info
curl -X POST http://localhost:8545 \
  -d '{"jsonrpc":"2.0","method":"blockchain_getMiningInfo","params":[],"id":1}'
```

---

## Architecture Summary

**✅ Completed Features:**
- Separated architecture (node + web)
- User authentication (Argon2 + JWT)
- Deterministic wallets (recoverable from email:password)
- Transaction signing (HMAC-SHA256)
- Mining (SHA256 PoW, 50 EDU rewards, 10s blocks)
- Transaction processing (balance transfers)
- Voucher system (generation + redemption + QR codes)
- RPC communication (10 methods)
- SQLite persistence (users, sessions, vouchers)

**🚀 Future Enhancements:**
- Database persistence for blockchain state
- Proper ECDSA signatures (secp256k1)
- Transaction fees
- WebSocket for real-time updates
- Admin dashboard
- Rate limiting
- Prometheus metrics
- Docker containerization
- C++ core integration (performance critical operations)

---

**Last Updated:** December 9, 2025
**System Version:** 1.0.0 (Production Ready)
**Architecture:** Separated Node + Web Client + Voucher System
