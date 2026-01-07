#!/bin/bash

echo "🚀 Creating Real Blockchain Transactions"
echo "========================================"
echo ""

# Login as Alice and get her session token
echo "1️⃣  Logging in as Alice..."
ALICE_LOGIN=$(curl -s -X POST http://localhost:8080/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"alice","password":"password123"}')

ALICE_TOKEN=$(echo $ALICE_LOGIN | python3 -c "import sys, json; print(json.load(sys.stdin).get('session_token', ''))" 2>/dev/null)

if [ -z "$ALICE_TOKEN" ]; then
    echo "❌ Failed to login as Alice"
    echo "Response: $ALICE_LOGIN"
    exit 1
fi

echo "✅ Alice logged in (token: ${ALICE_TOKEN:0:20}...)"

# Get Alice's wallet info
ALICE_INFO=$(curl -s http://localhost:8080/api/auth/me \
  -H "Authorization: Bearer $ALICE_TOKEN")
  
ALICE_WALLET=$(echo $ALICE_INFO | python3 -c "import sys, json; d=json.load(sys.stdin); print(d.get('data', {}).get('wallet_address', ''))" 2>/dev/null)
echo "   Wallet: $ALICE_WALLET"
echo ""

# Login as Bob
echo "2️⃣  Logging in as Bob..."
BOB_LOGIN=$(curl -s -X POST http://localhost:8080/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"bob","password":"password123"}')

BOB_TOKEN=$(echo $BOB_LOGIN | python3 -c "import sys, json; print(json.load(sys.stdin).get('session_token', ''))" 2>/dev/null)

if [ -z "$BOB_TOKEN" ]; then
    echo "❌ Failed to login as Bob"
    exit 1
fi

echo "✅ Bob logged in"

BOB_INFO=$(curl -s http://localhost:8080/api/auth/me \
  -H "Authorization: Bearer $BOB_TOKEN")
  
BOB_WALLET=$(echo $BOB_INFO | python3 -c "import sys, json; d=json.load(sys.stdin); print(d.get('data', {}).get('wallet_address', ''))" 2>/dev/null)
echo "   Wallet: $BOB_WALLET"
echo ""

# Login as Carol
echo "3️⃣  Logging in as Carol..."
CAROL_LOGIN=$(curl -s -X POST http://localhost:8080/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"carol","password":"password123"}')

CAROL_TOKEN=$(echo $CAROL_LOGIN | python3 -c "import sys, json; print(json.load(sys.stdin).get('session_token', ''))" 2>/dev/null)

if [ -z "$CAROL_TOKEN" ]; then
    echo "❌ Failed to login as Carol"
    exit 1
fi

echo "✅ Carol logged in"

CAROL_INFO=$(curl -s http://localhost:8080/api/auth/me \
  -H "Authorization: Bearer $CAROL_TOKEN")
  
CAROL_WALLET=$(echo $CAROL_INFO | python3 -c "import sys, json; d=json.load(sys.stdin); print(d.get('data', {}).get('wallet_address', ''))" 2>/dev/null)
echo "   Wallet: $CAROL_WALLET"
echo ""

# Transaction 1: Alice sends 25 EDU to Bob
echo "4️⃣  Transaction 1: Alice → Bob (25 EDU)"
TX1=$(curl -s -X POST http://localhost:8080/api/blockchain/send-transaction \
  -H "Authorization: Bearer $ALICE_TOKEN" \
  -H "Content-Type: application/json" \
  -d "{\"recipient\":\"$BOB_WALLET\",\"amount\":25}")

echo "   Response: $TX1"
echo ""

# Mine a block to confirm transaction
echo "5️⃣  Mining block to confirm transaction..."
MINE1=$(curl -s -X POST http://localhost:8080/api/blockchain/mine \
  -H "Authorization: Bearer $ALICE_TOKEN")

echo "   $MINE1"
echo ""

sleep 2

# Transaction 2: Bob sends 50 EDU to Carol
echo "6️⃣  Transaction 2: Bob → Carol (50 EDU)"
TX2=$(curl -s -X POST http://localhost:8080/api/blockchain/send-transaction \
  -H "Authorization: Bearer $BOB_TOKEN" \
  -H "Content-Type: application/json" \
  -d "{\"recipient\":\"$CAROL_WALLET\",\"amount\":50}")

echo "   Response: $TX2"
echo ""

# Mine another block
echo "7️⃣  Mining block to confirm transaction..."
MINE2=$(curl -s -X POST http://localhost:8080/api/blockchain/mine \
  -H "Authorization: Bearer $BOB_TOKEN")

echo "   $MINE2"
echo ""

sleep 2

# Transaction 3: Carol sends 10 EDU to Alice
echo "8️⃣  Transaction 3: Carol → Alice (10 EDU)"
TX3=$(curl -s -X POST http://localhost:8080/api/blockchain/send-transaction \
  -H "Authorization: Bearer $CAROL_TOKEN" \
  -H "Content-Type: application/json" \
  -d "{\"recipient\":\"$ALICE_WALLET\",\"amount\":10}")

echo "   Response: $TX3"
echo ""

# Mine final block
echo "9️⃣  Mining final block..."
MINE3=$(curl -s -X POST http://localhost:8080/api/blockchain/mine \
  -H "Authorization: Bearer $CAROL_TOKEN")

echo "   $MINE3"
echo ""

# Get final network status
echo "🔟 Final Blockchain Status:"
echo "================================"
curl -s http://localhost:8080/api/blockchain/network-status \
  -H "Authorization: Bearer $ALICE_TOKEN" | python3 -m json.tool 2>/dev/null || echo "Status check failed"

echo ""
echo "✅ Transaction creation complete!"
echo ""
echo "📊 Summary:"
echo "  • 3 transactions sent"
echo "  • 3 blocks mined"
echo "  • Alice → Bob: 25 EDU"
echo "  • Bob → Carol: 50 EDU"
echo "  • Carol → Alice: 10 EDU"
echo ""
echo "🌐 View in GUI: http://localhost:8080"
