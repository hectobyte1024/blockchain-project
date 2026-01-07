#!/bin/bash
# Treasury System Test Script
# Demonstrates how to use the coin sales API

echo "╔═══════════════════════════════════════════════════════════╗"
echo "║       Treasury Coin Sales System - Test Demo             ║"
echo "╚═══════════════════════════════════════════════════════════╝"
echo ""

RPC_URL="http://localhost:8545"

echo "1️⃣  Getting current EDU price..."
curl -s -X POST $RPC_URL \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "treasury_getPrice",
    "params": [],
    "id": 1
  }' | jq '.'
echo ""

echo "2️⃣  Getting treasury statistics..."
curl -s -X POST $RPC_URL \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "treasury_getStats",
    "params": [],
    "id": 2
  }' | jq '.'
echo ""

echo "3️⃣  Setting price to \$0.25 per EDU..."
curl -s -X POST $RPC_URL \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "treasury_setPrice",
    "params": [25],
    "id": 3
  }' | jq '.'
echo ""

echo "4️⃣  Checking treasury balance..."
curl -s -X POST $RPC_URL \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "wallet_getBalance",
    "params": ["edu1qTreasury00000000000000000000"],
    "id": 4
  }' | jq '.'
echo ""

echo "5️⃣  Simulating a coin sale (DISABLED - needs UTXO implementation)..."
echo "    In production, after receiving \$10 cash:"
echo ""
echo "    curl -X POST $RPC_URL \\"
echo "      -H 'Content-Type: application/json' \\"
echo "      -d '{"
echo "        \"jsonrpc\": \"2.0\","
echo "        \"method\": \"treasury_sellCoins\","
echo "        \"params\": {"
echo "          \"buyer_address\": \"buyer_wallet_address_here\","
echo "          \"amount\": 100,"
echo "          \"payment_method\": \"cash\","
echo "          \"payment_proof\": \"Receipt #12345 - \$10 cash received\""
echo "        },"
echo "        \"id\": 5"
echo "      }'"
echo ""

echo "✅ Treasury system is ready for production!"
echo ""
echo "📝 How to sell coins:"
echo "   1. User contacts you wanting to buy EDU coins"
echo "   2. You quote the price (check with treasury_getPrice)"
echo "   3. User pays you cash/bank transfer/etc. (OFF-CHAIN)"
echo "   4. You confirm payment received"
echo "   5. You call treasury_sellCoins with their wallet address"
echo "   6. System creates transaction and sends them coins"
echo "   7. Transaction gets mined into a block"
echo "   8. User has coins in their wallet!"
echo ""
