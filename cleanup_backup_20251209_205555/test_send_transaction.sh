#!/bin/bash

# Transaction from Alice to Bob
# Alice needs to provide her seed phrase (email:password)

echo "Testing transaction signing and broadcast..."

# Deterministic seed from email:password
ALICE_SEED="alice@edu.net:Alice123!"
ALICE_ADDRESS="EDU23327d04d02229e70c2bbab1a0d1e0d98b806f09"
BOB_ADDRESS="EDU0e9ab78774eedd68cad522346b9928565ba5a04b"

echo "From: $ALICE_ADDRESS"
echo "To: $BOB_ADDRESS"
echo "Amount: 10.5 EDU"

curl -X POST http://localhost:8080/api/wallet/send \
  -H "Content-Type: application/json" \
  -d "{
    \"from_address\": \"$ALICE_ADDRESS\",
    \"to_address\": \"$BOB_ADDRESS\",
    \"amount\": 10.5,
    \"seed_phrase\": \"$ALICE_SEED\"
  }" | jq .

echo ""
echo "Checking Alice balance..."
curl -X POST http://localhost:8545 -H "Content-Type: application/json" \
  -d "{\"jsonrpc\":\"2.0\",\"method\":\"blockchain_getBalance\",\"params\":[\"$ALICE_ADDRESS\"],\"id\":1}" 2>/dev/null | jq .

echo ""
echo "Checking Bob balance..."
curl -X POST http://localhost:8545 -H "Content-Type: application/json" \
  -d "{\"jsonrpc\":\"2.0\",\"method\":\"blockchain_getBalance\",\"params\":[\"$BOB_ADDRESS\"],\"id\":1}" 2>/dev/null | jq .
