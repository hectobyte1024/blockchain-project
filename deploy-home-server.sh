#!/bin/bash
#
# EduNet Blockchain - Home Server Deployment Script
# This script sets up everything needed to host EduNet from your home ISP
#

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}"
echo "╔════════════════════════════════════════════════════════════╗"
echo "║      EduNet Blockchain - Home Deployment Setup            ║"
echo "║      Production-Ready Blockchain Hosting                  ║"
echo "╚════════════════════════════════════════════════════════════╝"
echo -e "${NC}"

# Check if running as root
if [[ $EUID -ne 0 ]]; then
   echo -e "${RED}This script must be run as root (use sudo)${NC}" 
   exit 1
fi

# Get the actual user (not root)
ACTUAL_USER="${SUDO_USER:-$USER}"
PROJECT_DIR="/home/$ACTUAL_USER/Documents/blockchain project"

echo -e "${YELLOW}Deployment Configuration:${NC}"
echo "User: $ACTUAL_USER"
echo "Project Directory: $PROJECT_DIR"
echo ""

# Step 1: System Updates
echo -e "${GREEN}[1/10] Updating system packages...${NC}"
apt-get update -qq
apt-get upgrade -y -qq

# Step 2: Install dependencies
echo -e "${GREEN}[2/10] Installing required packages...${NC}"
apt-get install -y \
    curl \
    wget \
    git \
    build-essential \
    pkg-config \
    libssl-dev \
    sqlite3 \
    ufw \
    net-tools \
    debian-keyring \
    debian-archive-keyring \
    apt-transport-https

# Step 3: Install Caddy (web server with automatic HTTPS)
echo -e "${GREEN}[3/10] Installing Caddy web server...${NC}"
if ! command -v caddy &> /dev/null; then
    curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/gpg.key' | gpg --dearmor -o /usr/share/keyrings/caddy-stable-archive-keyring.gpg
    curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/debian.deb.txt' | tee /etc/apt/sources.list.d/caddy-stable.list
    apt-get update -qq
    apt-get install -y caddy
    echo -e "${GREEN}✓ Caddy installed${NC}"
else
    echo -e "${GREEN}✓ Caddy already installed${NC}"
fi

# Step 4: Configure Caddy
echo -e "${GREEN}[4/10] Configuring Caddy reverse proxy...${NC}"
read -p "Enter your domain name (e.g., edunet.example.com or leave blank for IP-only access): " DOMAIN_NAME

if [ -z "$DOMAIN_NAME" ]; then
    # IP-only configuration
    cat > /etc/caddy/Caddyfile <<EOF
# EduNet Blockchain - IP-only configuration
:80 {
    reverse_proxy localhost:8080
    log {
        output file /var/log/caddy/edunet-access.log
    }
}
EOF
    echo -e "${YELLOW}⚠ Configured for IP-only access (HTTP). No HTTPS without domain.${NC}"
else
    # Domain configuration with auto-HTTPS
    cat > /etc/caddy/Caddyfile <<EOF
# EduNet Blockchain - Production configuration
$DOMAIN_NAME {
    reverse_proxy localhost:8080
    
    log {
        output file /var/log/caddy/edunet-access.log
    }
    
    # Security headers
    header {
        Strict-Transport-Security "max-age=31536000;"
        X-Content-Type-Options "nosniff"
        X-Frame-Options "DENY"
        Referrer-Policy "no-referrer-when-downgrade"
    }
}
EOF
    echo -e "${GREEN}✓ Configured for $DOMAIN_NAME with automatic HTTPS${NC}"
fi

mkdir -p /var/log/caddy
chown caddy:caddy /var/log/caddy

# Step 5: Create systemd service for EduNet
echo -e "${GREEN}[5/10] Creating systemd service...${NC}"
cat > /etc/systemd/system/edunet-blockchain.service <<EOF
[Unit]
Description=EduNet Blockchain Server
After=network.target
Wants=network-online.target

[Service]
Type=simple
User=$ACTUAL_USER
WorkingDirectory=$PROJECT_DIR
ExecStart=$PROJECT_DIR/target/release/edunet-gui
Restart=always
RestartSec=10
StandardOutput=append:/var/log/edunet/blockchain.log
StandardError=append:/var/log/edunet/blockchain-error.log

# Security settings
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=read-only
ReadWritePaths=/var/log/edunet $PROJECT_DIR/edunet-gui
Environment="RUST_LOG=info"
Environment="DATABASE_URL=sqlite://$PROJECT_DIR/edunet-gui/edunet.db"

[Install]
WantedBy=multi-user.target
EOF

# Create log directory
mkdir -p /var/log/edunet
chown $ACTUAL_USER:$ACTUAL_USER /var/log/edunet

echo -e "${GREEN}✓ Systemd service created${NC}"

# Step 6: Configure Firewall
echo -e "${GREEN}[6/10] Configuring firewall (UFW)...${NC}"
ufw --force enable
ufw default deny incoming
ufw default allow outgoing
ufw allow 22/tcp    # SSH
ufw allow 80/tcp    # HTTP
ufw allow 443/tcp   # HTTPS
ufw allow 8333/tcp  # Bitcoin P2P (if using P2P networking)
echo -e "${GREEN}✓ Firewall configured${NC}"

# Step 7: Build the blockchain server
echo -e "${GREEN}[7/10] Building EduNet blockchain server...${NC}"
cd "$PROJECT_DIR"
sudo -u $ACTUAL_USER cargo build --release --manifest-path edunet-gui/Cargo.toml
echo -e "${GREEN}✓ Server built successfully${NC}"

# Step 8: Initialize database
echo -e "${GREEN}[8/10] Initializing database...${NC}"
mkdir -p "$PROJECT_DIR/edunet-gui"
touch "$PROJECT_DIR/edunet-gui/edunet.db"
chown $ACTUAL_USER:$ACTUAL_USER "$PROJECT_DIR/edunet-gui/edunet.db"
echo -e "${GREEN}✓ Database initialized${NC}"

# Step 9: Enable and start services
echo -e "${GREEN}[9/10] Enabling services...${NC}"
systemctl daemon-reload
systemctl enable caddy
systemctl enable edunet-blockchain
systemctl restart caddy
systemctl start edunet-blockchain

# Wait a moment for services to start
sleep 3

# Step 10: Verify deployment
echo -e "${GREEN}[10/10] Verifying deployment...${NC}"
echo ""

if systemctl is-active --quiet caddy; then
    echo -e "${GREEN}✓ Caddy is running${NC}"
else
    echo -e "${RED}✗ Caddy failed to start${NC}"
fi

if systemctl is-active --quiet edunet-blockchain; then
    echo -e "${GREEN}✓ EduNet blockchain is running${NC}"
else
    echo -e "${RED}✗ EduNet blockchain failed to start${NC}"
    echo "Check logs: journalctl -u edunet-blockchain -n 50"
fi

# Get local IP
LOCAL_IP=$(hostname -I | awk '{print $1}')

echo ""
echo -e "${BLUE}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║              Deployment Complete!                          ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════════╝${NC}"
echo ""
echo -e "${GREEN}Local Network Access:${NC}"
echo "  http://$LOCAL_IP"
echo ""

if [ ! -z "$DOMAIN_NAME" ]; then
    echo -e "${GREEN}Internet Access (once DNS configured):${NC}"
    echo "  https://$DOMAIN_NAME"
    echo ""
fi

echo -e "${YELLOW}Next Steps:${NC}"
echo ""
echo "1. ${BLUE}Router Configuration:${NC}"
echo "   - Log into your router admin panel"
echo "   - Forward ports 80 and 443 to $LOCAL_IP"
echo "   - Save and restart router"
echo ""

if [ -z "$DOMAIN_NAME" ]; then
    echo "2. ${BLUE}Dynamic DNS Setup (recommended):${NC}"
    echo "   - Sign up at https://www.duckdns.org"
    echo "   - Create a subdomain (e.g., myedunet.duckdns.org)"
    echo "   - Install DuckDNS update client:"
    echo "     mkdir -p ~/duckdns"
    echo "     cd ~/duckdns"
    echo "     echo 'echo url=\"https://www.duckdns.org/update?domains=YOUR_DOMAIN&token=YOUR_TOKEN&ip=\" | curl -k -o ~/duckdns/duck.log -K -' > duck.sh"
    echo "     chmod 700 duck.sh"
    echo "     crontab -e  # Add: */5 * * * * ~/duckdns/duck.sh >/dev/null 2>&1"
    echo ""
fi

echo "3. ${BLUE}Security Checklist:${NC}"
echo "   ✓ Firewall enabled"
echo "   ✓ Services auto-restart on failure"
echo "   ⚠ Change default demo user passwords"
echo "   ⚠ Set up backups for /edunet-gui/edunet.db"
echo ""

echo "4. ${BLUE}Service Management:${NC}"
echo "   - View logs:    journalctl -u edunet-blockchain -f"
echo "   - Restart:      sudo systemctl restart edunet-blockchain"
echo "   - Stop:         sudo systemctl stop edunet-blockchain"
echo "   - Status:       sudo systemctl status edunet-blockchain"
echo ""

echo "5. ${BLUE}Database Backups:${NC}"
echo "   - Backup:  cp $PROJECT_DIR/edunet-gui/edunet.db ~/edunet-backup-\$(date +%Y%m%d).db"
echo "   - Automate: Add to crontab: 0 2 * * * (backup command)"
echo ""

echo -e "${GREEN}Your EduNet blockchain is now live! 🚀${NC}"
echo ""
echo "Access your blockchain explorer locally:"
echo "  http://$LOCAL_IP/dashboard"
echo ""
echo "Default demo accounts:"
echo "  alice / password123"
echo "  bob   / password123"
echo "  carol / password123"
echo ""
echo -e "${YELLOW}⚠ Remember to change these passwords in production!${NC}"
