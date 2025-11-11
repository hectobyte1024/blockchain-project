# Blockchain Network Architecture & Deployment Guide

## 🌐 Complete Network Architecture Overview

Your blockchain network will consist of multiple interconnected components:

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Web Frontend  │    │  API Gateway    │    │  Full Nodes     │
│   (EduNet GUI)  │◄──►│   (Your Server) │◄──►│  (Validators)   │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │
         │                       │                       │
         ▼                       ▼                       ▼
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│  Light Clients  │    │   RPC Server    │    │  Mining Nodes   │
│   (Mobile App)  │    │ (Blockchain API)│    │  (Block Miners) │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

## 🚀 Deployment Strategy

### 1. **Your Central Server** (Bootstrap Node)
This is your main server that acts as the network coordinator:

```toml
# server-config.toml
[network]
listen_addr = "0.0.0.0:8333"
public_ip = "YOUR_SERVER_IP"
is_bootstrap_node = true
enable_mining = true
enable_rpc = true

[api]
bind_address = "0.0.0.0:8332"  # RPC API
web_port = 3000                # Web interface
enable_cors = true
max_connections = 1000

[mining]
miner_address = "your_address_here"
enable_auto_mining = true
difficulty_target = 0x1e0fffff

[database]
path = "/var/blockchain/data"
max_size = "100GB"
```

### 2. **Web Interface Integration**
Your EduNet GUI becomes the primary user interface:

```javascript
// Frontend configuration
const BLOCKCHAIN_CONFIG = {
  rpcEndpoint: 'https://your-server.com:8332/rpc',
  wsEndpoint: 'wss://your-server.com:8332/ws',
  networkId: 'edunet-main',
  chainId: 1337,
  
  // Connection options
  retryAttempts: 3,
  connectionTimeout: 10000,
  
  // API endpoints
  endpoints: {
    balance: '/api/v1/wallets/{address}/balance',
    send: '/api/v1/transactions/send',
    history: '/api/v1/transactions/{address}',
    status: '/api/v1/network/status'
  }
};
```

## 📱 Client Types & How People Join

### **Type 1: Web Users** (Easiest Entry)
Users access through your website without installing anything:

```html
<!-- Your website integration -->
<script src="https://your-server.com/js/blockchain-client.js"></script>
<script>
const wallet = new EduNetWallet({
  serverUrl: 'https://your-server.com',
  networkId: 'edunet-main'
});

// Users can immediately:
wallet.createAccount();           // Create wallet
wallet.sendTransaction(to, amount); // Send coins
wallet.viewBalance();            // Check balance
</script>
```

**How they join**: Visit your website → Create account → Start transacting

### **Type 2: Full Node Operators** (Network Validators)
People who want to help secure the network:

```bash
# They download your node software
curl -sSL https://your-server.com/install.sh | bash

# Configure their node
./edunet-node --connect your-server.com:8333 \
              --data-dir ~/.edunet \
              --enable-mining \
              --wallet-address their_address
```

**How they join**: Download node → Connect to your bootstrap → Start validating

### **Type 3: Mobile App Users**
Light clients that connect through your API:

```javascript
// Mobile app configuration
const mobileClient = new EduNetLightClient({
  bootstrapNodes: ['your-server.com:8333'],
  apiGateway: 'https://your-server.com:8332',
  enableNotifications: true
});
```

**How they join**: Download mobile app → Auto-connects to network

## 🔧 Technical Implementation

### **1. Bootstrap Server Setup**

<function_calls>
<invoke name="create_file">
<parameter name="content">#!/bin/bash
# bootstrap-server-setup.sh

echo "🚀 Setting up EduNet Bootstrap Server"

# Create directory structure
mkdir -p /opt/edunet/{data,logs,config}
cd /opt/edunet

# Build the blockchain node
git clone https://github.com/your-username/edunet-blockchain.git
cd edunet-blockchain
cargo build --release --bin edunet-node

# Create configuration
cat > config/bootstrap.toml << EOF
[network]
listen_addr = "0.0.0.0:8333"
max_peers = 500
is_bootstrap = true
network_magic = 0xEDUC8NET

[mining]
enable_mining = true
miner_address = "edunet1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh"
difficulty_target = 0x1e0fffff

[rpc]
bind_address = "0.0.0.0:8332"
enable_auth = false  # For now, add auth later
enable_cors = true

[storage]
data_dir = "/opt/edunet/data"
max_db_size = "100GB"

[logging]
level = "info"
file = "/opt/edunet/logs/node.log"
EOF

# Create systemd service
cat > /etc/systemd/system/edunet-node.service << EOF
[Unit]
Description=EduNet Blockchain Node
After=network.target

[Service]
Type=simple
User=edunet
WorkingDirectory=/opt/edunet
ExecStart=/opt/edunet/edunet-blockchain/target/release/edunet-node --config config/bootstrap.toml
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
EOF

# Start the service
systemctl daemon-reload
systemctl enable edunet-node
systemctl start edunet-node

echo "✅ Bootstrap server is running on port 8333"
echo "   RPC API available on port 8332"
echo "   View logs: journalctl -u edunet-node -f"