#!/bin/bash

# EduNet Multi-User Blockchain Demo
# This script demonstrates the multi-user wallet system

echo "🎓 EduNet Blockchain - Multi-User Demo"
echo "======================================"
echo ""

# Build the project
echo "🔨 Building EduNet blockchain..."
cd "/home/hectobyte1024/Documents/blockchain project"
cargo build --release --bin edunet-gui

if [ $? -ne 0 ]; then
    echo "❌ Build failed!"
    exit 1
fi

echo "✅ Build completed!"
echo ""

# Start the server in background
echo "🚀 Starting EduNet blockchain server..."
cargo run --release --bin edunet-gui -- --bootstrap &
SERVER_PID=$!

# Wait for server to start
sleep 5

echo ""
echo "🌐 EduNet Multi-User System Ready!"
echo ""
echo "📱 Open your web browser and visit:"
echo "   http://localhost:8080"
echo ""
echo "👥 Demo Users Available:"
echo "   Username: alice    | Password: password123 | University: Stanford"
echo "   Username: bob      | Password: password123 | University: MIT"  
echo "   Username: carol    | Password: password123 | University: UC Berkeley"
echo ""
echo "🎯 What to Test:"
echo "   1. Login as different users in different browser tabs/windows"
echo "   2. Each user has their own blockchain wallet automatically!"
echo "   3. Send EDU tokens between users"
echo "   4. Start mining from different accounts"
echo "   5. View real-time blockchain network stats"
echo ""
echo "🔧 Technical Features:"
echo "   ✅ Individual wallets per user"
echo "   ✅ Session-based authentication"
echo "   ✅ Real-time WebSocket updates"  
echo "   ✅ Blockchain transaction system"
echo "   ✅ P2P network simulation"
echo ""
echo "Press Ctrl+C to stop the server..."

# Wait for interrupt
trap "echo ''; echo '🛑 Stopping EduNet server...'; kill $SERVER_PID; exit 0" INT

# Keep script running
wait $SERVER_PID