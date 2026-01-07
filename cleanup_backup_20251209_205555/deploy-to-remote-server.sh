#!/bin/bash
# Deploy EduNet to Remote Debian Server
# This script helps you deploy the separated architecture to another computer

set -e

echo "🚀 EduNet Remote Deployment Guide"
echo "=================================="
echo ""

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

echo -e "${BLUE}This guide will help you deploy EduNet to your other Debian computer.${NC}"
echo ""
echo "Prerequisites on the remote server:"
echo "  ✓ Debian/Ubuntu Linux"
echo "  ✓ Rust (install with: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh)"
echo "  ✓ C++ compiler (install with: sudo apt install build-essential cmake)"
echo "  ✓ SQLite (install with: sudo apt install sqlite3 libsqlite3-dev)"
echo ""

# Ask for server details
read -p "Enter your server's IP address: " SERVER_IP
read -p "Enter SSH username (default: $USER): " SSH_USER
SSH_USER=${SSH_USER:-$USER}
read -p "Enter deployment directory on server (default: ~/edunet): " DEPLOY_DIR
DEPLOY_DIR=${DEPLOY_DIR:-~/edunet}

echo ""
echo -e "${YELLOW}Configuration:${NC}"
echo "  Server IP: $SERVER_IP"
echo "  SSH User: $SSH_USER"
echo "  Deploy Directory: $DEPLOY_DIR"
echo ""

read -p "Continue with deployment? (y/n): " CONFIRM
if [ "$CONFIRM" != "y" ]; then
    echo "Deployment cancelled."
    exit 0
fi

echo ""
echo -e "${BLUE}Step 1: Testing SSH connection...${NC}"
if ssh -o ConnectTimeout=5 $SSH_USER@$SERVER_IP "echo 'Connection successful'" 2>/dev/null; then
    echo -e "${GREEN}✅ SSH connection successful${NC}"
else
    echo -e "${RED}❌ Cannot connect to server${NC}"
    echo "Please check:"
    echo "  1. Server IP address is correct"
    echo "  2. SSH is running on server (sudo systemctl start ssh)"
    echo "  3. You can SSH manually: ssh $SSH_USER@$SERVER_IP"
    exit 1
fi

echo ""
echo -e "${BLUE}Step 2: Creating deployment directory...${NC}"
ssh $SSH_USER@$SERVER_IP "mkdir -p $DEPLOY_DIR"
echo -e "${GREEN}✅ Directory created${NC}"

echo ""
echo -e "${BLUE}Step 3: Copying project files to server...${NC}"
rsync -avz --progress \
    --exclude 'target' \
    --exclude 'cpp-core/build' \
    --exclude '*.log' \
    --exclude '*.pid' \
    --exclude 'blockchain-data' \
    --exclude 'edunet-web.db' \
    --exclude '.git' \
    ./ $SSH_USER@$SERVER_IP:$DEPLOY_DIR/

echo -e "${GREEN}✅ Files copied${NC}"

echo ""
echo -e "${BLUE}Step 4: Installing dependencies on server...${NC}"
ssh $SSH_USER@$SERVER_IP "bash -s" << 'ENDSSH'
    # Check for Rust
    if ! command -v cargo &> /dev/null; then
        echo "Installing Rust..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source $HOME/.cargo/env
    fi
    
    # Check for build tools
    if ! command -v cmake &> /dev/null; then
        echo "Installing build tools..."
        sudo apt update
        sudo apt install -y build-essential cmake sqlite3 libsqlite3-dev pkg-config libssl-dev
    fi
    
    echo "✅ All dependencies installed"
ENDSSH

echo -e "${GREEN}✅ Dependencies ready${NC}"

echo ""
echo -e "${BLUE}Step 5: Building on server...${NC}"
ssh $SSH_USER@$SERVER_IP "cd $DEPLOY_DIR && bash -s" << 'ENDSSH'
    source $HOME/.cargo/env
    
    # Build C++ core
    echo "Building C++ blockchain core..."
    cd cpp-core
    mkdir -p build
    cd build
    cmake ..
    make -j$(nproc)
    cd ../..
    
    # Build Rust components
    echo "Building Rust components..."
    cargo build --release --bin blockchain-node
    cargo build --release --bin edunet-web
    
    echo "✅ Build complete"
ENDSSH

echo -e "${GREEN}✅ Build successful${NC}"

echo ""
echo -e "${BLUE}Step 6: Configuring firewall...${NC}"
ssh $SSH_USER@$SERVER_IP "sudo bash -s" << 'ENDSSH'
    # Install ufw if not present
    if ! command -v ufw &> /dev/null; then
        apt install -y ufw
    fi
    
    # Configure firewall
    ufw allow 22/tcp     # SSH
    ufw allow 8080/tcp   # Web interface
    ufw allow 8545/tcp   # RPC (optional, only if exposing RPC)
    ufw allow 9000/tcp   # P2P network
    
    # Don't enable yet, let user do it
    echo "Firewall rules configured (not enabled yet)"
ENDSSH

echo -e "${GREEN}✅ Firewall configured${NC}"
echo -e "${YELLOW}⚠️  To enable firewall: sudo ufw enable${NC}"

echo ""
echo -e "${BLUE}Step 7: Creating systemd services...${NC}"
ssh $SSH_USER@$SERVER_IP "sudo bash -s" << ENDSSH
    # Blockchain node service
    cat > /etc/systemd/system/edunet-node.service << 'EOF'
[Unit]
Description=EduNet Blockchain Node
After=network.target

[Service]
Type=simple
User=$SSH_USER
WorkingDirectory=$DEPLOY_DIR
Environment="VALIDATOR_ADDRESS=server_validator_address"
ExecStart=$DEPLOY_DIR/target/release/blockchain-node \
    --rpc-port 8545 \
    --p2p-port 9000 \
    --data-dir $DEPLOY_DIR/blockchain-data \
    --mining true \
    --validator-address server_validator_address
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
EOF

    # Web client service
    cat > /etc/systemd/system/edunet-web.service << 'EOF'
[Unit]
Description=EduNet Web Interface
After=network.target edunet-node.service
Requires=edunet-node.service

[Service]
Type=simple
User=$SSH_USER
WorkingDirectory=$DEPLOY_DIR
ExecStart=$DEPLOY_DIR/target/release/edunet-web \
    --port 8080 \
    --node-rpc http://localhost:8545 \
    --database $DEPLOY_DIR/edunet-web.db
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
EOF

    # Reload systemd
    systemctl daemon-reload
    
    echo "✅ Systemd services created"
ENDSSH

echo -e "${GREEN}✅ Services configured${NC}"

echo ""
echo -e "${GREEN}=================================="
echo "🎉 Deployment Complete!"
echo "==================================${NC}"
echo ""
echo -e "${BLUE}Server Details:${NC}"
echo "  IP Address: $SERVER_IP"
echo "  Deployment Directory: $DEPLOY_DIR"
echo ""
echo -e "${BLUE}Next Steps on Server:${NC}"
echo ""
echo "1. SSH into server:"
echo -e "   ${YELLOW}ssh $SSH_USER@$SERVER_IP${NC}"
echo ""
echo "2. Start services:"
echo -e "   ${YELLOW}sudo systemctl start edunet-node${NC}"
echo -e "   ${YELLOW}sudo systemctl start edunet-web${NC}"
echo ""
echo "3. Enable auto-start on boot:"
echo -e "   ${YELLOW}sudo systemctl enable edunet-node${NC}"
echo -e "   ${YELLOW}sudo systemctl enable edunet-web${NC}"
echo ""
echo "4. Check status:"
echo -e "   ${YELLOW}sudo systemctl status edunet-node${NC}"
echo -e "   ${YELLOW}sudo systemctl status edunet-web${NC}"
echo ""
echo "5. View logs:"
echo -e "   ${YELLOW}sudo journalctl -u edunet-node -f${NC}"
echo -e "   ${YELLOW}sudo journalctl -u edunet-web -f${NC}"
echo ""
echo -e "${BLUE}Access Your Site:${NC}"
echo "  🌐 Web Interface: http://$SERVER_IP:8080"
echo "  📡 RPC Endpoint: http://$SERVER_IP:8545"
echo "  🔗 P2P Network: $SERVER_IP:9000"
echo ""
echo -e "${BLUE}For Friends to Connect:${NC}"
echo "  Tell them to use bootstrap peer: ${YELLOW}$SERVER_IP:9000${NC}"
echo ""
echo -e "${BLUE}Optional: Setup Domain & HTTPS${NC}"
echo "  1. Point domain DNS to $SERVER_IP"
echo "  2. Install nginx: sudo apt install nginx"
echo "  3. Install certbot: sudo apt install certbot python3-certbot-nginx"
echo "  4. Get SSL certificate: sudo certbot --nginx -d yourdomain.com"
echo ""
echo -e "${GREEN}Happy mining! 🎉${NC}"
