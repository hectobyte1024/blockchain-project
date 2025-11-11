#!/bin/bash
# Complete Home Server Deployment Guide

echo "🏠 EduNet Blockchain - Home Server Deployment"
echo "============================================="
echo ""
echo "This will set up your second computer as a blockchain server."
echo "People will be able to connect from the internet to use your blockchain!"
echo ""

# Step 1: Check if we're on the right computer
read -p "Is this your second computer that will be the server? (y/n): " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Please run this on your second computer (the one that will be the server)"
    exit 1
fi

# Step 2: Get network info
echo "🔍 Getting network information..."
LOCAL_IP=$(ip route get 1 | awk '{print $7; exit}' 2>/dev/null)
PUBLIC_IP=$(curl -s --max-time 10 ifconfig.me 2>/dev/null || echo "Could not determine")

echo "📍 Network Information:"
echo "   Local IP: $LOCAL_IP"
echo "   Public IP: $PUBLIC_IP"
echo ""

# Step 3: Copy blockchain project
echo "📁 Setting up blockchain project..."
if [ ! -d "edunet-blockchain" ]; then
    echo "You need to copy your blockchain project to this computer."
    echo ""
    echo "Option 1 - Copy from your main computer:"
    echo "   scp -r user@main-computer:/path/to/blockchain-project ~/edunet-blockchain"
    echo ""
    echo "Option 2 - Use Git (if you've uploaded to GitHub):"
    echo "   git clone https://github.com/yourusername/blockchain-project.git edunet-blockchain"
    echo ""
    echo "Option 3 - Use USB drive:"
    echo "   Copy the entire blockchain-project folder to ~/edunet-blockchain"
    echo ""
    read -p "Press Enter when you have the blockchain project in ~/edunet-blockchain..."
fi

cd ~/edunet-blockchain 2>/dev/null || {
    echo "❌ Could not find blockchain project. Please copy it to ~/edunet-blockchain first."
    exit 1
}

# Step 4: Install dependencies and build
echo "🔧 Installing dependencies..."
sudo apt update
sudo apt install -y curl git build-essential cmake pkg-config libssl-dev ufw

# Install Rust if needed
if ! command -v cargo &> /dev/null; then
    echo "🦀 Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source $HOME/.cargo/env
fi

# Build the blockchain
echo "🔨 Building blockchain..."
cargo build --release

# Step 5: Configure firewall
echo "🔥 Configuring firewall..."
sudo ufw --force reset
sudo ufw allow ssh
sudo ufw allow 8333/tcp comment "EduNet P2P"
sudo ufw allow 8080/tcp comment "EduNet Web Interface"
sudo ufw --force enable

# Step 6: Create systemd service for auto-start
echo "⚙️ Setting up auto-start service..."
sudo tee /etc/systemd/system/edunet-blockchain.service > /dev/null << EOF
[Unit]
Description=EduNet Blockchain Bootstrap Server
After=network.target

[Service]
Type=simple
User=$USER
WorkingDirectory=$HOME/edunet-blockchain
ExecStart=$HOME/edunet-blockchain/target/release/edunet-gui --bootstrap
Restart=always
RestartSec=10
Environment=RUST_LOG=info

[Install]
WantedBy=multi-user.target
EOF

sudo systemctl daemon-reload
sudo systemctl enable edunet-blockchain

echo ""
echo "✅ Home server setup complete!"
echo ""
echo "🌐 Your blockchain server is ready!"
echo "   Local IP: $LOCAL_IP"
echo "   Public IP: $PUBLIC_IP"
echo ""
echo "📋 IMPORTANT - Next Steps:"
echo "1. Configure port forwarding on your router:"
echo "   Forward port 8333 (blockchain P2P) -> $LOCAL_IP:8333"
echo "   Forward port 8080 (web interface) -> $LOCAL_IP:8080"
echo ""
echo "2. Start your blockchain server:"
echo "   sudo systemctl start edunet-blockchain"
echo "   # OR manually: ./launch.sh bootstrap"
echo ""
echo "3. Test locally:"
echo "   curl http://$LOCAL_IP:8080"
echo ""
echo "4. Share with others:"
echo "   Local network: http://$LOCAL_IP:8080"
echo "   Internet: http://$PUBLIC_IP:8080 (after port forwarding)"
echo ""
echo "5. Monitor server:"
echo "   sudo systemctl status edunet-blockchain"
echo "   sudo journalctl -u edunet-blockchain -f"
echo ""
echo "🎉 Ready to launch your blockchain network!"