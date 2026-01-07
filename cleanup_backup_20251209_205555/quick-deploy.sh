#!/bin/bash
# EduNet Blockchain - Quick Deploy Script
# Automates the entire deployment process

set -e  # Exit on error

echo "🚀 EduNet Blockchain - Quick Deployment"
echo "========================================"
echo ""

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check if running as root
if [ "$EUID" -ne 0 ]; then 
    echo -e "${RED}❌ Please run as root (sudo)${NC}"
    exit 1
fi

# Get the actual user (not root)
REAL_USER=${SUDO_USER:-$USER}
PROJECT_DIR="/home/$REAL_USER/Documents/blockchain project"

echo "📂 Project directory: $PROJECT_DIR"
echo "👤 User: $REAL_USER"
echo ""

# Step 1: Build the project
echo "🔨 Step 1/7: Building project..."
cd "$PROJECT_DIR"
sudo -u $REAL_USER cargo build --release 2>&1 | tee build.log
if [ ${PIPESTATUS[0]} -ne 0 ]; then
    echo -e "${RED}❌ Build failed! Check build.log${NC}"
    exit 1
fi
echo -e "${GREEN}✅ Build complete${NC}"
echo ""

# Step 2: Install dependencies
echo "📦 Step 2/7: Installing dependencies..."
apt-get update
apt-get install -y debian-keyring debian-archive-keyring apt-transport-https curl ufw sqlite3
if ! command -v caddy &> /dev/null; then
    curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/gpg.key' | gpg --dearmor -o /usr/share/keyrings/caddy-stable-archive-keyring.gpg
    curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/debian.deb.txt' | tee /etc/apt/sources.list.d/caddy-stable.list
    apt-get update
    apt-get install -y caddy
fi
echo -e "${GREEN}✅ Dependencies installed${NC}"
echo ""

# Step 3: Configure firewall
echo "🔥 Step 3/7: Configuring firewall..."
ufw --force enable
ufw default deny incoming
ufw default allow outgoing
ufw allow 22/tcp    # SSH
ufw allow 80/tcp    # HTTP
ufw allow 443/tcp   # HTTPS
ufw allow 9000/tcp  # P2P Blockchain
echo -e "${GREEN}✅ Firewall configured${NC}"
echo ""

# Step 4: Get domain name
echo "🌐 Step 4/7: Domain configuration..."
echo -n "Enter your domain name (or press Enter for IP-only): "
read DOMAIN

if [ -z "$DOMAIN" ]; then
    LOCAL_IP=$(ip route get 1 | awk '{print $(NF-2);exit}')
    DOMAIN="$LOCAL_IP"
    echo "Using local IP: $LOCAL_IP"
fi
echo ""

# Step 5: Configure Caddy
echo "⚙️  Step 5/7: Configuring web server..."
cat > /etc/caddy/Caddyfile << EOF
# EduNet Blockchain Reverse Proxy

$DOMAIN {
    reverse_proxy localhost:8080
    
    # Security headers
    header {
        X-Frame-Options "SAMEORIGIN"
        X-Content-Type-Options "nosniff"
        X-XSS-Protection "1; mode=block"
        Referrer-Policy "strict-origin-when-cross-origin"
    }
    
    # Access logging
    log {
        output file /var/log/caddy/edunet-access.log
    }
}

# Redirect HTTP to HTTPS
http://$DOMAIN {
    redir https://$DOMAIN{uri} permanent
}
EOF

systemctl enable caddy
systemctl restart caddy
echo -e "${GREEN}✅ Web server configured${NC}"
echo ""

# Step 6: Create systemd service
echo "🔧 Step 6/7: Creating system service..."
cat > /etc/systemd/system/edunet-blockchain.service << EOF
[Unit]
Description=EduNet Blockchain Node
After=network.target

[Service]
Type=simple
User=$REAL_USER
WorkingDirectory=$PROJECT_DIR
ExecStart=$PROJECT_DIR/target/release/edunet-gui
Restart=always
RestartSec=10

# Logging
StandardOutput=journal
StandardError=journal
SyslogIdentifier=edunet-blockchain

# Security
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=$PROJECT_DIR

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload
systemctl enable edunet-blockchain
echo -e "${GREEN}✅ Service created${NC}"
echo ""

# Step 7: Start the service
echo "🚀 Step 7/7: Starting blockchain..."
systemctl start edunet-blockchain
sleep 5

if systemctl is-active --quiet edunet-blockchain; then
    echo -e "${GREEN}✅ Blockchain started successfully!${NC}"
else
    echo -e "${RED}❌ Failed to start blockchain${NC}"
    echo "Check logs with: sudo journalctl -xeu edunet-blockchain"
    exit 1
fi
echo ""

# Display status
echo "================================"
echo "🎉 DEPLOYMENT COMPLETE!"
echo "================================"
echo ""
echo "📊 Service Status:"
systemctl status edunet-blockchain --no-pager | head -n 5
echo ""
echo "🌐 Access URLs:"
echo "   Local:    http://localhost:8080"
if [ "$DOMAIN" != "$LOCAL_IP" ]; then
    echo "   External: https://$DOMAIN"
else
    echo "   External: http://$DOMAIN"
fi
echo ""
echo "🔐 Demo Login:"
echo "   Username: alice"
echo "   Password: EduNet2025!Alice#Secure"
echo "   ⚠️  CHANGE THIS PASSWORD IMMEDIATELY!"
echo ""
echo "📋 Next Steps:"
echo "   1. Test locally: curl http://localhost:8080/api/blockchain/network-status"
echo "   2. Configure router port forwarding (80, 443, 9000)"
echo "   3. Set up Dynamic DNS (optional): https://www.duckdns.org"
echo "   4. Change demo passwords in production"
echo "   5. Invite friends to join your network!"
echo ""
echo "📚 View logs: sudo journalctl -fu edunet-blockchain"
echo "🔄 Restart:   sudo systemctl restart edunet-blockchain"
echo "🛑 Stop:      sudo systemctl stop edunet-blockchain"
echo ""
echo "✅ Your blockchain is now running 24/7!"
echo ""
