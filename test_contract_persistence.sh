#!/bin/bash
set -e

echo "=== Contract Persistence Test ==="
echo

# Clean up previous test data
echo "1. Cleaning up previous data..."
pkill -f blockchain-node || true
rm -rf blockchain-data/contracts
rm -rf blockchain-data/blocks/*
sleep 2

# Start node
echo "2. Starting blockchain node with mining..."
cargo run --bin blockchain-node -- --mining > blockchain-node.log 2>&1 &
NODE_PID=$!
echo "   Node PID: $NODE_PID"
sleep 10

# Deploy a contract
echo "3. Deploying test contract..."
DEPLOY_RESULT=$(curl -s -X POST http://127.0.0.1:8545 -H "Content-Type: application/json" -d '{
  "jsonrpc":"2.0",
  "id":"1",
  "method":"contract_deploy",
  "params":{
    "deployer":"edu1qTreasury00000000000000000000",
    "bytecode":"6000356000526001601ff3",
    "value":0,
    "gas_limit":100000
  }
}')

echo "$DEPLOY_RESULT" | jq '.'

# Extract contract address
CONTRACT_ADDR=$(echo "$DEPLOY_RESULT" | jq -r '.result.contract_address')
echo "   Contract deployed at: $CONTRACT_ADDR"

# Check if contract file was created
echo
echo "4. Checking if contract was persisted to disk..."
if [ -f "blockchain-data/contracts/contract_${CONTRACT_ADDR}.json" ]; then
    echo "   ✅ Contract file exists!"
    echo "   File contents:"
    cat "blockchain-data/contracts/contract_${CONTRACT_ADDR}.json" | jq '.'
else
    echo "   ❌ Contract file NOT found!"
    exit 1
fi

# Stop node
echo
echo "5. Stopping node..."
kill $NODE_PID
sleep 3

# Restart node
echo "6. Restarting node (should load contracts from disk)..."
cargo run --bin blockchain-node -- --mining > blockchain-node.log 2>&1 &
NODE_PID=$!
sleep 10

# Try to get contract code (should work if loaded from disk)
echo
echo "7. Retrieving contract code after restart..."
CODE_RESULT=$(curl -s -X POST http://127.0.0.1:8545 -H "Content-Type: application/json" -d "{
  \"jsonrpc\":\"2.0\",
  \"id\":\"2\",
  \"method\":\"contract_getCode\",
  \"params\":{
    \"contract\":\"$CONTRACT_ADDR\"
  }
}")

echo "$CODE_RESULT" | jq '.'

CODE=$(echo "$CODE_RESULT" | jq -r '.result.code')
if [ "$CODE" = "6000356000526001601ff3" ]; then
    echo "   ✅ Contract code loaded successfully after restart!"
else
    echo "   ❌ Contract code NOT found after restart!"
    kill $NODE_PID
    exit 1
fi

# Cleanup
echo
echo "8. Cleaning up..."
kill $NODE_PID

echo
echo "=== ✅ Contract Persistence Test PASSED ==="
