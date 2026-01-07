# EduNet Blockchain - Separated Architecture

## 🎯 Overview

EduNet uses a **separated architecture** with two independent components:

1. **`blockchain-node`** - Full blockchain node (for operators/miners)
2. **`edunet-web`** - Web interface (for hosting the website)

This separation allows:
- Regular users to access the website without running blockchain software
- Node operators to earn mining rewards by running dedicated nodes
- Independent scaling of web servers and blockchain nodes

## 🏗️ Architecture Diagram

```
┌─────────────┐         ┌──────────────────┐         ┌─────────────────┐
│   Browser   │ HTTP    │   edunet-web     │  RPC    │ blockchain-node │
│   (Users)   ├────────>│  (Web Server)    ├────────>│  (Node/Miner)   │
└─────────────┘         └──────────────────┘         └─────────────────┘
                                                               │
                        ┌──────────────────┐                  │ P2P
                        │ blockchain-node  │<─────────────────┤
                        │  (Friend's PC)   │                  │
                        └──────────────────┘                  │
                                                               │
                        ┌──────────────────┐                  │
                        │ blockchain-node  │<─────────────────┘
                        │ (Another Friend) │
                        └──────────────────┘
```

## 📦 Components

### blockchain-node (Blockchain Daemon)

**What it does:**
- Validates and mines blocks
- Maintains blockchain state
- Participates in P2P network
- Exposes JSON-RPC API (port 8545)
- Earns mining rewards

**Who runs it:**
- You (on your server)
- Friends who want to support the network
- Anyone wanting to earn mining rewards

**Ports:**
- `8545` - RPC API (for web clients)
- `9000` - P2P network (for other nodes)

### edunet-web (Web Client)

**What it does:**
- Serves website to users
- User registration and authentication
- Wallet management interface
- Marketplace, loans, NFTs UI
- Connects to blockchain-node via RPC

**Who runs it:**
- You (on your server, alongside blockchain-node)
- Anyone wanting to host a public web interface

**Ports:**
- `8080` - HTTP web server

## 🚀 Deployment Scenarios

### Scenario 1: Full Server (You)

Your server runs BOTH components:

```bash
./deploy-separated.sh
```

This starts:
- `blockchain-node` - Mining and earning rewards
- `edunet-web` - Website for users to access

**Benefits:**
- Users visit your website
- Your server mines blocks and earns rewards
- All-in-one solution

### Scenario 2: Node Operator (Friends)

Friends who want to earn rewards run just the node:

```bash
export VALIDATOR_ADDRESS=their_wallet_address
export BOOTSTRAP_PEERS=your_server_ip:9000
./run-node.sh
```

This starts:
- `blockchain-node` - Connects to your network, mines blocks
- Mining rewards go to their wallet

**Benefits:**
- Decentralized network
- Friends earn mining rewards
- Increased network security

### Scenario 3: Web-Only Server

Host just the website (connecting to existing node):

```bash
./target/release/edunet-web \
  --port 8080 \
  --node-rpc http://your_node_server:8545
```

**Benefits:**
- Multiple web servers for load balancing
- Geographic distribution
- Separate web hosting from mining

## 🛠️ Setup Instructions

### Prerequisites

- Rust (latest stable)
- C++ compiler (g++)
- CMake
- SQLite

### Step 1: Build Everything

```bash
# Build C++ blockchain core
cd cpp-core/build
cmake ..
make -j$(nproc)
cd ../..

# Build Rust components
cargo build --release --bin blockchain-node
cargo build --release --bin edunet-web
```

### Step 2: Deploy

**Option A: Full deployment (node + web)**
```bash
./deploy-separated.sh
```

**Option B: Node only (for friends/miners)**
```bash
export VALIDATOR_ADDRESS=your_wallet_address
./run-node.sh
```

**Option C: Web only (separate web server)**
```bash
./target/release/edunet-web \
  --port 8080 \
  --node-rpc http://blockchain-node-host:8545 \
  --database ./edunet-web.db
```

## 🔧 Configuration

### blockchain-node Options

```bash
--rpc-host <HOST>              RPC server host (default: 0.0.0.0)
--rpc-port <PORT>              RPC server port (default: 8545)
--p2p-port <PORT>              P2P network port (default: 9000)
--data-dir <DIR>               Blockchain data directory
--bootstrap-peers <PEERS>      Comma-separated bootstrap peers
--mining <BOOL>                Enable mining (default: true)
--validator-address <ADDR>     Address for mining rewards
```

### edunet-web Options

```bash
--host <HOST>                  Web server host (default: 0.0.0.0)
--port <PORT>                  Web server port (default: 8080)
--node-rpc <URL>               Blockchain node RPC endpoint
--database <PATH>              SQLite database path
```

## 📡 RPC API

The blockchain node exposes a JSON-RPC API:

### Endpoint
```
http://localhost:8545
```

### Methods

| Method | Description |
|--------|-------------|
| `blockchain_getBlockHeight` | Get current block height |
| `blockchain_getBalance` | Get address balance |
| `blockchain_getTransaction` | Get transaction by hash |
| `blockchain_sendRawTransaction` | Submit signed transaction |
| `blockchain_getBlock` | Get block by height |
| `blockchain_getNetworkInfo` | Network statistics |
| `blockchain_getMempoolInfo` | Mempool status |
| `blockchain_getMiningInfo` | Mining/node info |
| `blockchain_getSyncStatus` | Sync progress |

### Example Request

```bash
curl -X POST http://localhost:8545 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "blockchain_getBlockHeight",
    "params": [],
    "id": 1
  }'
```

## 💰 Mining Rewards

When running a blockchain node with mining enabled:

1. Node validates transactions
2. Creates new blocks
3. Finds valid proof-of-work
4. Receives block reward to validator address
5. Earns transaction fees

**To earn rewards:**
```bash
export VALIDATOR_ADDRESS=your_edunet_wallet_address
./run-node.sh
```

## 🌐 Network Setup

### Your Server (Bootstrap Node)

```bash
# Start with default settings
./deploy-separated.sh
```

Your server becomes the bootstrap node at:
- RPC: `http://your_ip:8545`
- P2P: `your_ip:9000`
- Web: `http://your_ip:8080`

### Friends (Additional Nodes)

```bash
# Connect to your bootstrap node
export BOOTSTRAP_PEERS=your_server_ip:9000
export VALIDATOR_ADDRESS=their_wallet
./run-node.sh
```

Their nodes will:
- Connect to your P2P network
- Sync blockchain data
- Mine blocks independently
- Earn rewards to their wallet

## 📊 Monitoring

### Check Status
```bash
./check-status.sh
```

### View Logs
```bash
# Node logs
tail -f blockchain-node.log

# Web logs
tail -f edunet-web.log
```

### Stop Services
```bash
./stop-services.sh
```

## 🔒 Security Notes

1. **RPC API**: By default open to all IPs (0.0.0.0). In production:
   - Use firewall rules to restrict access
   - Enable authentication
   - Use HTTPS proxy (nginx/caddy)

2. **P2P Network**: Open to allow peer connections
   - Configure firewall to allow port 9000
   - Monitor peer connections

3. **Web Server**: Public-facing
   - Use HTTPS (Let's Encrypt)
   - Set up rate limiting
   - Regular security updates

## 📚 User Access Models

### Regular Users
- Visit website in browser
- No software installation
- Create account, use wallet
- Access marketplace, loans, NFTs

### Node Operators (You + Friends)
- Install and run `blockchain-node`
- Earn mining rewards
- Support network decentralization
- Can be anywhere in the world

### Web Hosts
- Run `edunet-web` only
- Connect to any blockchain node
- Provide user interface
- Geographic distribution

## 🎓 Comparison to Old Architecture

| Aspect | Old (Monolithic) | New (Separated) |
|--------|------------------|-----------------|
| User access | Must run full node | Just visit website |
| Node operators | Same as users | Dedicated miners |
| Mining rewards | Unclear | Clear validator address |
| Scalability | Limited | Independent scaling |
| Deployment | One binary | Two specialized binaries |

## 🛠️ Development

### Adding RPC Methods

1. Define method in `rust-system/blockchain-rpc/src/lib.rs`
2. Implement server handler in `server.rs`
3. Implement client method in `client.rs`
4. Use in web or node code

### Testing RPC Connection

```bash
# Start node
./run-node.sh &

# Test connection
curl http://localhost:8545 -X POST \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"blockchain_getBlockHeight","params":[],"id":1}'
```

## 📝 License

MIT License - See LICENSE file

## 🤝 Contributing

This is a production blockchain system. Contributions welcome for:
- Additional RPC methods
- Performance optimizations
- Security enhancements
- Documentation improvements
