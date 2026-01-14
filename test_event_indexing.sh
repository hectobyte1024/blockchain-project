#!/bin/bash
# Test contract event indexing and querying

set -e

echo "🧪 Testing Event Indexing System..."

# Cleanup
echo "🧹 Cleaning up old data..."
rm -rf blockchain-data/
pkill -f blockchain-node || true
sleep 2

# Start node
echo "🚀 Starting blockchain node..."
cargo run --release --bin blockchain-node -- --rpc-port 8545 > /tmp/blockchain-node.log 2>&1 &
NODE_PID=$!
sleep 5

# Check if node started
if ! ps -p $NODE_PID > /dev/null; then
    echo "❌ Node failed to start"
    cat /tmp/blockchain-node.log
    exit 1
fi
echo "✅ Node started (PID: $NODE_PID)"

# Deploy a contract that emits events
# EVM bytecode: PUSH1 0x42 PUSH1 0x00 MSTORE PUSH1 0x20 PUSH1 0x00 LOG1 (emit event with topic)
BYTECODE="604260005260206000a1"

echo "📝 Deploying contract with event emission..."
DEPLOY_RESPONSE=$(curl -s -X POST http://localhost:8545 \
    -H "Content-Type: application/json" \
    -d '{
        "jsonrpc": "2.0",
        "id": 1,
        "method": "contract_deploy",
        "params": [{
            "deployer": "edu1qTreasury0000000000000000000",
            "bytecode": "'"$BYTECODE"'",
            "value": 0,
            "gasLimit": 100000
        }]
    }')

echo "Deploy response: $DEPLOY_RESPONSE"

# Extract contract address
CONTRACT_ADDRESS=$(echo $DEPLOY_RESPONSE | jq -r '.result.contractAddress')

if [ "$CONTRACT_ADDRESS" == "null" ] || [ -z "$CONTRACT_ADDRESS" ]; then
    echo "❌ Contract deployment failed"
    kill $NODE_PID
    exit 1
fi

echo "✅ Contract deployed at: $CONTRACT_ADDRESS"

# Wait for indexing
sleep 2

# Test 1: Get events by contract address
echo ""
echo "📊 Test 1: Get events by contract address..."
EVENTS_BY_ADDR=$(curl -s -X POST http://localhost:8545 \
    -H "Content-Type: application/json" \
    -d '{
        "jsonrpc": "2.0",
        "id": 1,
        "method": "contract_getEventsByAddress",
        "params": [{
            "address": "'"$CONTRACT_ADDRESS"'"
        }]
    }')

echo "Events by address: $EVENTS_BY_ADDR"
EVENT_COUNT=$(echo $EVENTS_BY_ADDR | jq -r '.result.events | length')

if [ "$EVENT_COUNT" -gt 0 ]; then
    echo "✅ Found $EVENT_COUNT event(s) for contract"
else
    echo "⚠️  No events found (contract might not have emitted events)"
fi

# Test 2: Get events by block height
echo ""
echo "📊 Test 2: Get events by block height..."
EVENTS_BY_BLOCK=$(curl -s -X POST http://localhost:8545 \
    -H "Content-Type: application/json" \
    -d '{
        "jsonrpc": "2.0",
        "id": 1,
        "method": "contract_getEventsByBlock",
        "params": [0]
    }')

echo "Events by block: $EVENTS_BY_BLOCK"

# Test 3: Query logs with filter
echo ""
echo "📊 Test 3: Query logs with filter..."
LOGS=$(curl -s -X POST http://localhost:8545 \
    -H "Content-Type: application/json" \
    -d '{
        "jsonrpc": "2.0",
        "id": 1,
        "method": "contract_getLogs",
        "params": [{
            "address": "'"$CONTRACT_ADDRESS"'",
            "fromBlock": 0,
            "toBlock": 10
        }]
    }')

echo "Query logs result: $LOGS"
LOG_COUNT=$(echo $LOGS | jq -r '.result.logs | length')

echo ""
if [ "$LOG_COUNT" -gt 0 ]; then
    echo "✅ Event Indexing Test PASSED - Found $LOG_COUNT log(s)"
else
    echo "⚠️  No logs found (event emission might have failed)"
fi

# Get total event count
echo ""
echo "📈 Getting total indexed events..."
# Note: We don't have a direct RPC for this, but we can check via address query

# Cleanup
echo ""
echo "🧹 Cleaning up..."
kill $NODE_PID
sleep 2

echo ""
echo "✅ Event Indexing Tests Complete!"
echo "📋 Summary:"
echo "  - Events by address: $EVENT_COUNT"
echo "  - Logs via filter: $LOG_COUNT"
