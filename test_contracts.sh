#!/bin/bash
# Test Smart Contract Deployment and Execution

set -e

echo "🧪 Testing Smart Contracts on EDU Blockchain"
echo "=============================================="

# Kill any running node
pkill -f blockchain-node || true
sleep 1

# Clean blockchain data
echo "🧹 Cleaning blockchain data..."
rm -rf blockchain-data/blocks/*

# Start node
echo "🚀 Starting blockchain node..."
cargo run --bin blockchain-node -- --mining > blockchain-node.log 2>&1 &
NODE_PID=$!

# Wait for node to start
echo "⏳ Waiting for node to initialize..."
sleep 8

# Test 1: Deploy Simple Contract
echo ""
echo "Test 1: Deploy Simple Storage Contract"
echo "----------------------------------------"

# Simple bytecode: PUSH1 0x00 PUSH1 0x00 RETURN
BYTECODE="600060006000f0"

DEPLOY_RESULT=$(curl -s -X POST http://127.0.0.1:8545 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": "1",
    "method": "contract_deploy",
    "params": {
      "deployer": "edu1qContractDeployer000000000",
      "bytecode": "'"$BYTECODE"'",
      "value": 0,
      "gas_limit": 1000000
    }
  }')

echo "Deploy result:"
echo "$DEPLOY_RESULT" | jq '.'

# Extract contract address
CONTRACT_ADDR=$(echo "$DEPLOY_RESULT" | jq -r '.result.contract_address')

if [ "$CONTRACT_ADDR" != "null" ] && [ -n "$CONTRACT_ADDR" ]; then
    echo "✅ Contract deployed at: $CONTRACT_ADDR"
else
    echo "❌ Deployment failed"
    pkill -f blockchain-node
    exit 1
fi

# Test 2: Get Contract Code
echo ""
echo "Test 2: Get Contract Code"
echo "-------------------------"

CODE_RESULT=$(curl -s -X POST http://127.0.0.1:8545 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": "2",
    "method": "contract_getCode",
    "params": {
      "contract": "'"$CONTRACT_ADDR"'"
    }
  }')

echo "$CODE_RESULT" | jq '.'
echo "✅ Tests complete!"

# Cleanup
pkill -f blockchain-node
