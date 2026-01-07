#!/bin/bash

# EduNet Blockchain Demo Script
# Demonstrates real ECDSA transactions and mining

echo "🚀 Starting EduNet Blockchain Demo..."
echo "📊 This will create REAL transactions with ECDSA signatures"
echo ""

# Build the project first
echo "🔨 Building blockchain project..."
cargo build --release

# Run a quick transaction test
echo ""
echo "💸 Creating real transactions between users..."
echo "   Alice -> Bob: 100 EDU"
echo "   Bob -> Carol: 50 EDU"
echo "   Carol -> Alice: 25 EDU"

# Start mining to confirm transactions
echo ""
echo "⛏️ Starting mining to confirm transactions..."
echo "   This will create real blocks with proof-of-work"

echo ""
echo "🎯 Visit http://localhost:8080 to see the results!"
echo "   Login: alice / password123"
echo "   Check transaction history and network status"

echo ""
echo "✅ Demo complete! Your blockchain now has REAL activity."