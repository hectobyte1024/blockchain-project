#!/bin/bash
# Run Blockchain Node Only - For Node Operators & Miners
# This script is for friends who want to run a node and earn mining rewards

set -e

echo "⛏️  EduNet Blockchain Node"
echo "=========================="
echo ""
echo "This will start a blockchain node that:"
echo "  ✅ Validates and mines blocks (earn rewards!)"
echo "  ✅ Participates in P2P network"
echo "  ✅ Exposes RPC API for clients"
echo ""

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Configuration
NODE_RPC_PORT=${NODE_RPC_PORT:-8545}
NODE_P2P_PORT=${NODE_P2P_PORT:-9000}
DATA_DIR=${DATA_DIR:-"./blockchain-data"}
BOOTSTRAP_PEERS=${BOOTSTRAP_PEERS:-""}

# Validator address (where mining rewards go)
if [ -z "$VALIDATOR_ADDRESS" ]; then
    echo -e "${YELLOW}⚠️  No VALIDATOR_ADDRESS set. Using default.${NC}"
    echo "   Set your address with: export VALIDATOR_ADDRESS=your_address"
    VALIDATOR_ADDRESS="default_validator_$(hostname)"
fi

echo -e "${BLUE}📋 Configuration:${NC}"
echo "  - RPC Port: $NODE_RPC_PORT"
echo "  - P2P Port: $NODE_P2P_PORT"
echo "  - Data Directory: $DATA_DIR"
echo "  - Validator Address: $VALIDATOR_ADDRESS"
if [ -n "$BOOTSTRAP_PEERS" ]; then
    echo "  - Bootstrap Peers: $BOOTSTRAP_PEERS"
fi
echo ""

# Check if already built
if [ ! -f "./target/release/blockchain-node" ]; then
    echo -e "${YELLOW}🔨 Building blockchain node...${NC}"
    
    # Build C++ core
    cd cpp-core
    if [ ! -d "build" ]; then
        mkdir build
    fi
    cd build
    cmake ..
    make -j$(nproc)
    cd ../..
    
    # Build Rust node
    cargo build --release --bin blockchain-node
    
    echo -e "${GREEN}✅ Build complete!${NC}"
else
    echo -e "${GREEN}✅ Using existing build${NC}"
fi

# Create data directory
mkdir -p "$DATA_DIR"

# Build command
CMD="./target/release/blockchain-node \\
    --rpc-port $NODE_RPC_PORT \\
    --p2p-port $NODE_P2P_PORT \\
    --data-dir $DATA_DIR \\
    --mining \\
    --validator-address $VALIDATOR_ADDRESS"

# Add bootstrap peers if provided
if [ -n "$BOOTSTRAP_PEERS" ]; then
    CMD="$CMD --bootstrap-peers $BOOTSTRAP_PEERS"
fi

echo -e "${BLUE}🚀 Starting blockchain node...${NC}"
echo ""

# Run in foreground (or background if requested)
if [ "$RUN_BACKGROUND" = "true" ]; then
    $CMD > blockchain-node.log 2>&1 &
    NODE_PID=$!
    echo "$NODE_PID" > blockchain-node.pid
    echo -e "${GREEN}✅ Node started in background (PID: $NODE_PID)${NC}"
    echo "   View logs: tail -f blockchain-node.log"
    echo "   Stop node: kill $NODE_PID"
else
    echo -e "${GREEN}🎉 Node is now running!${NC}"
    echo "   Mining rewards go to: $VALIDATOR_ADDRESS"
    echo "   Press Ctrl+C to stop"
    echo ""
    exec $CMD
fi
