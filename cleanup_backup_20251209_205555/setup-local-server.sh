#!/bin/bash
# Local Server Setup Script for Second Computer
# Run this on your second computer that will be the server

echo "🏠 Setting up EduNet Blockchain on Local Server"
echo "=============================================="

# Update system
echo "📦 Updating system packages..."
sudo apt update && sudo apt upgrade -y

# Install required dependencies
echo "🔧 Installing dependencies..."
sudo apt install -y curl git build-essential cmake pkg-config libssl-dev

# Install Rust if not already installed
if ! command -v cargo &> /dev/null; then
    echo "🦀 Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source $HOME/.cargo/env
else
    echo "✅ Rust already installed"
fi

# Get your blockchain code
echo "📥 Setting up blockchain project..."
if [ ! -d "edunet-blockchain" ]; then
    # If you don't have it on GitHub yet, copy from your main computer
    echo "📋 Copy your blockchain project to this computer:"
    echo "   scp -r /path/to/blockchain-project user@this-computer:~/edunet-blockchain"
    echo "   OR clone from GitHub when you upload it"
    # git clone https://github.com/yourusername/edunet-blockchain.git edunet-blockchain
else
    echo "✅ Project directory already exists"
fi

cd edunet-blockchain 2>/dev/null || { echo "❌ Please copy your blockchain project first"; exit 1; }

# Build the project
echo "🔨 Building blockchain..."
cargo build --release

# Get network information
LOCAL_IP=$(ip route get 1 | awk '{print $7; exit}')
PUBLIC_IP=$(curl -s ifconfig.me || echo "Unable to get public IP")

echo ""
echo "✅ Local server setup complete!"
echo ""
echo "🌐 Network Configuration:"
echo "   Local IP: $LOCAL_IP"
echo "   Public IP: $PUBLIC_IP"
echo ""
echo "🚀 To start your blockchain server:"
echo "   ./launch.sh bootstrap"
echo ""
echo "🔗 Others can connect using:"
echo "   Local network: ./launch.sh client $LOCAL_IP:8333"
echo "   Internet: ./launch.sh client $PUBLIC_IP:8333"
echo ""