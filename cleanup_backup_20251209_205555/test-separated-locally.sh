#!/bin/bash
# Test separated architecture locally

echo "🧪 Testing Separated Architecture Locally"
echo "=========================================="

# Kill any existing processes
pkill -f blockchain-node 2>/dev/null
pkill -f edunet-web 2>/dev/null
sleep 2

# Start blockchain node
echo "Starting blockchain-node..."
./target/release/blockchain-node \
    --rpc-port 8545 \
    --p2p-port 9000 \
    --data-dir ./test-blockchain-data \
    --validator-address test_validator \
    --mining \
    > blockchain-node.log 2>&1 &

NODE_PID=$!
echo "Node PID: $NODE_PID"
sleep 3

# Check if node started
if kill -0 $NODE_PID 2>/dev/null; then
    echo "✅ Blockchain node started"
else
    echo "❌ Node failed to start. Check blockchain-node.log"
    exit 1
fi

# Start web client
echo "Starting edunet-web..."
./target/release/edunet-web \
    --port 8080 \
    --node-rpc http://localhost:8545 \
    --database ./test-edunet-web.db \
    > edunet-web.log 2>&1 &

WEB_PID=$!
echo "Web PID: $WEB_PID"
sleep 2

# Check if web started
if kill -0 $WEB_PID 2>/dev/null; then
    echo "✅ Web client started"
else
    echo "❌ Web failed to start. Check edunet-web.log"
    kill $NODE_PID
    exit 1
fi

echo ""
echo "✅ Both services running!"
echo ""
echo "📡 Blockchain Node:"
echo "   RPC: http://localhost:8545"
echo "   P2P: Port 9000"
echo "   Logs: tail -f blockchain-node.log"
echo ""
echo "🌐 Web Interface:"
echo "   URL: http://localhost:8080"
echo "   Logs: tail -f edunet-web.log"
echo ""
echo "🧪 Test RPC connection:"
echo '   curl -X POST http://localhost:8545 -H "Content-Type: application/json" -d '"'"'{"jsonrpc":"2.0","method":"blockchain_getBlockHeight","params":[],"id":1}'"'"
echo ""
echo "🛑 To stop: kill $NODE_PID $WEB_PID"
echo ""
echo "PIDs saved to:"
echo "$NODE_PID" > blockchain-node.pid
echo "$WEB_PID" > edunet-web.pid
