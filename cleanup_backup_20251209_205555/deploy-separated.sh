#!/bin/bash
# Deploy Separated Architecture - Blockchain Node + Web Client

set -e

echo "🚀 EduNet Deployment - Separated Architecture"
echo "=============================================="
echo ""
echo "This script will deploy:"
echo "  1. blockchain-node (mining, P2P, RPC)"
echo "  2. edunet-web (website for users)"
echo ""

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
NODE_RPC_PORT=8545
NODE_P2P_PORT=9000
WEB_PORT=8080
DATA_DIR="./blockchain-data"
DB_FILE="./edunet-web.db"

echo -e "${BLUE}📋 Configuration:${NC}"
echo "  - Blockchain Node RPC: http://localhost:$NODE_RPC_PORT"
echo "  - Blockchain Node P2P: Port $NODE_P2P_PORT"
echo "  - Web Interface: http://localhost:$WEB_PORT"
echo "  - Data Directory: $DATA_DIR"
echo ""

# Build C++ core
echo -e "${YELLOW}🔨 Building C++ blockchain core...${NC}"
cd cpp-core
if [ ! -d "build" ]; then
    mkdir build
fi
cd build
cmake ..
make -j$(nproc)
cd ../..

# Build Rust components
echo -e "${YELLOW}🔨 Building Rust components...${NC}"
cargo build --release --bin blockchain-node
cargo build --release --bin edunet-web

echo -e "${GREEN}✅ Build complete!${NC}"
echo ""

# Create data directory
mkdir -p "$DATA_DIR"

# Start blockchain node in background
echo -e "${BLUE}🚀 Starting blockchain node...${NC}"
./target/release/blockchain-node \
    --rpc-port $NODE_RPC_PORT \
    --p2p-port $NODE_P2P_PORT \
    --data-dir "$DATA_DIR" \
    --mining \
    > blockchain-node.log 2>&1 &

NODE_PID=$!
echo "  Node PID: $NODE_PID"
echo "$NODE_PID" > blockchain-node.pid

# Wait for node to start
echo "  Waiting for node to initialize..."
sleep 3

# Check if node is running
if kill -0 $NODE_PID 2>/dev/null; then
    echo -e "${GREEN}  ✅ Blockchain node started successfully${NC}"
else
    echo -e "${YELLOW}  ⚠️  Node may have failed to start. Check blockchain-node.log${NC}"
fi

# Start web server
echo -e "${BLUE}🌐 Starting web client...${NC}"
./target/release/edunet-web \
    --port $WEB_PORT \
    --node-rpc "http://localhost:$NODE_RPC_PORT" \
    --database "$DB_FILE" \
    > edunet-web.log 2>&1 &

WEB_PID=$!
echo "  Web PID: $WEB_PID"
echo "$WEB_PID" > edunet-web.pid

# Wait for web server to start
sleep 2

# Check if web server is running
if kill -0 $WEB_PID 2>/dev/null; then
    echo -e "${GREEN}  ✅ Web client started successfully${NC}"
else
    echo -e "${YELLOW}  ⚠️  Web client may have failed to start. Check edunet-web.log${NC}"
fi

echo ""
echo -e "${GREEN}=============================================="
echo "🎉 EduNet is now running!"
echo "=============================================="
echo ""
echo "📡 Blockchain Node:"
echo "   - RPC API: http://localhost:$NODE_RPC_PORT"
echo "   - P2P Network: Port $NODE_P2P_PORT"
echo "   - Logs: blockchain-node.log"
echo "   - PID: $NODE_PID"
echo ""
echo "🌐 Web Interface:"
echo "   - Website: http://localhost:$WEB_PORT"
echo "   - Logs: edunet-web.log"
echo "   - PID: $WEB_PID"
echo ""
echo "💡 What this means:"
echo "   - YOUR server runs both components (mining + website)"
echo "   - FRIENDS can run 'blockchain-node' only (earn rewards)"
echo "   - USERS just visit the website in browser (no install)"
echo ""
echo "🛑 To stop:"
echo "   kill $NODE_PID $WEB_PID"
echo "   or run: ./stop-services.sh"
echo -e "${NC}"

# Create stop script
cat > stop-services.sh << 'EOF'
#!/bin/bash
echo "🛑 Stopping EduNet services..."

if [ -f blockchain-node.pid ]; then
    NODE_PID=$(cat blockchain-node.pid)
    if kill -0 $NODE_PID 2>/dev/null; then
        kill $NODE_PID
        echo "✅ Stopped blockchain node (PID: $NODE_PID)"
    fi
    rm blockchain-node.pid
fi

if [ -f edunet-web.pid ]; then
    WEB_PID=$(cat edunet-web.pid)
    if kill -0 $WEB_PID 2>/dev/null; then
        kill $WEB_PID
        echo "✅ Stopped web client (PID: $WEB_PID)"
    fi
    rm edunet-web.pid
fi

echo "✅ All services stopped"
EOF

chmod +x stop-services.sh

# Create status check script
cat > check-status.sh << 'EOF'
#!/bin/bash
echo "📊 EduNet Status Check"
echo "======================"

if [ -f blockchain-node.pid ]; then
    NODE_PID=$(cat blockchain-node.pid)
    if kill -0 $NODE_PID 2>/dev/null; then
        echo "✅ Blockchain Node: Running (PID: $NODE_PID)"
    else
        echo "❌ Blockchain Node: Not running"
    fi
else
    echo "❌ Blockchain Node: Not started"
fi

if [ -f edunet-web.pid ]; then
    WEB_PID=$(cat edunet-web.pid)
    if kill -0 $WEB_PID 2>/dev/null; then
        echo "✅ Web Client: Running (PID: $WEB_PID)"
    else
        echo "❌ Web Client: Not running"
    fi
else
    echo "❌ Web Client: Not started"
fi

echo ""
echo "📊 Latest Logs:"
echo "Blockchain Node (last 5 lines):"
if [ -f blockchain-node.log ]; then
    tail -5 blockchain-node.log
fi
echo ""
echo "Web Client (last 5 lines):"
if [ -f edunet-web.log ]; then
    tail -5 edunet-web.log
fi
EOF

chmod +x check-status.sh

echo "📝 Additional scripts created:"
echo "   - stop-services.sh (stop everything)"
echo "   - check-status.sh (check if running)"
