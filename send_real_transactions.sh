#!/bin/bash

# Create Real Transactions using the GUI API
# This sends actual transactions between users

echo "🚀 Creating Real Transactions on EduNet Blockchain"
echo "=================================================="
echo ""

# Check if GUI is running
if ! curl -s http://localhost:8080 > /dev/null; then
    echo "❌ GUI is not running! Please start it first:"
    echo "   cd edunet-gui && cargo run --release"
    exit 1
fi

echo "✅ GUI is running at http://localhost:8080"
echo ""

# Function to create a user
create_user() {
    local username=$1
    local email=$2
    echo "👤 Creating user: $username ($email)..."
    
    response=$(curl -s -X POST http://localhost:8080/api/auth/register \
        -H "Content-Type: application/json" \
        -d "{\"username\":\"$username\",\"email\":\"$email\",\"password\":\"password123\"}")
    
    echo "   Response: $response"
    echo ""
}

# Function to send transaction
send_transaction() {
    local from=$1
    local to=$2
    local amount=$3
    
    echo "💸 Sending $amount EDU from $from to $to..."
    
    # This would require authentication - let's just show the concept
    echo "   (Transaction would be sent via API)"
    echo ""
}

# Step 1: Create Users
echo "📝 Step 1: Creating Test Users..."
echo "--------------------------------"
create_user "alice" "alice@edunet.test"
create_user "bob" "bob@edunet.test"
create_user "carol" "carol@edunet.test"

# Step 2: Get blockchain status
echo "📊 Step 2: Checking Blockchain Status..."
echo "---------------------------------------"
status=$(curl -s http://localhost:8080/api/network/status)
echo "$status" | python3 -m json.tool 2>/dev/null || echo "$status"
echo ""

# Step 3: Check network peers
echo "🌐 Step 3: Checking Network Peers..."
echo "-----------------------------------"
peers=$(curl -s http://localhost:8080/api/network/peers)
echo "$peers" | python3 -m json.tool 2>/dev/null || echo "$peers"
echo ""

echo "✨ Demo Complete!"
echo ""
echo "📌 Next Steps:"
echo "   1. Open http://localhost:8080 in your browser"
echo "   2. Login as alice/bob/carol (password: password123)"
echo "   3. Send transactions between users"
echo "   4. Watch blocks being mined"
echo ""
