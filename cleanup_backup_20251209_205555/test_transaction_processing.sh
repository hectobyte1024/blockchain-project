#!/bin/bash

echo "=== Testing Transaction Processing with Balance Updates ==="
echo ""

MINER="EDU_validator_miner"
ALICE="EDU23327d04d02229e70c2bbab1a0d1e0d98b806f09"
BOB="EDU0e9ab78774eedd68cad522346b9928565ba5a04b"
ALICE_SEED="alice@edu.net:Alice123!"

echo "Step 1: Check miner balance (should have mining rewards)"
MINER_BALANCE=$(curl -s -X POST http://localhost:8545 -H "Content-Type: application/json" \
  -d "{\"jsonrpc\":\"2.0\",\"method\":\"blockchain_getBalance\",\"params\":[\"$MINER\"],\"id\":1}" | jq -r '.result')
echo "Miner balance: $MINER_BALANCE satoshis ($(echo "scale=2; $MINER_BALANCE / 100000000" | bc) EDU)"
echo ""

echo "Step 2: Check Alice balance (should be 0)"
ALICE_BALANCE=$(curl -s -X POST http://localhost:8545 -H "Content-Type: application/json" \
  -d "{\"jsonrpc\":\"2.0\",\"method\":\"blockchain_getBalance\",\"params\":[\"$ALICE\"],\"id\":1}" | jq -r '.result')
echo "Alice balance: $ALICE_BALANCE satoshis"
echo ""

echo "Step 3: Send 100 EDU from Alice to Bob (should fail - insufficient balance)"
TX_RESULT=$(curl -s -X POST http://localhost:8080/api/wallet/send \
  -H "Content-Type: application/json" \
  -d "{
    \"from_address\": \"$ALICE\",
    \"to_address\": \"$BOB\",
    \"amount\": 100.0,
    \"seed_phrase\": \"$ALICE_SEED\"
  }")
echo "$TX_RESULT" | jq .
echo ""

echo "Step 4: Wait for a block to be mined (transaction should fail due to insufficient balance)"
echo "Waiting 12 seconds for mining..."
sleep 12
echo ""

echo "Step 5: Check balances again"
ALICE_BALANCE=$(curl -s -X POST http://localhost:8545 -H "Content-Type: application/json" \
  -d "{\"jsonrpc\":\"2.0\",\"method\":\"blockchain_getBalance\",\"params\":[\"$ALICE\"],\"id\":1}" | jq -r '.result')
BOB_BALANCE=$(curl -s -X POST http://localhost:8545 -H "Content-Type: application/json" \
  -d "{\"jsonrpc\":\"2.0\",\"method\":\"blockchain_getBalance\",\"params\":[\"$BOB\"],\"id\":1}" | jq -r '.result')
echo "Alice balance: $ALICE_BALANCE satoshis"
echo "Bob balance: $BOB_BALANCE satoshis"
echo ""

echo "=== Test Complete ==="
echo ""
echo "Note: To test successful transactions, the miner would need to send funds to Alice first."
echo "This requires implementing a special coinbase transaction or manual balance adjustment."
