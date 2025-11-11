#!/bin/bash

# EduNet Blockchain Launcher Script
# Usage: ./launch.sh [bootstrap|client] [server-address]

set -e

echo "🌐 EduNet Blockchain Launcher"
echo "=============================="

MODE=${1:-"bootstrap"}
SERVER_ADDRESS=${2:-""}

case $MODE in
    "bootstrap")
        echo "🌟 Launching as BOOTSTRAP NODE"
        echo "   - This will be the network seed point"
        echo "   - Blockchain network: port 8333"
        echo "   - Web interface: port 8080"
        echo "   - Others can connect to this server"
        echo ""
        
        # Build the project first
        echo "🔨 Building blockchain project..."
        cargo build --release
        
        echo "🚀 Starting bootstrap server..."
        cargo run --release --bin edunet-gui -- --bootstrap
        ;;
        
    "client")
        if [ -z "$SERVER_ADDRESS" ]; then
            echo "❌ Error: Server address required for client mode"
            echo "Usage: ./launch.sh client <bootstrap-server-ip:port>"
            echo "Example: ./launch.sh client 192.168.1.100:8333"
            exit 1
        fi
        
        echo "👥 Launching as CLIENT NODE"
        echo "   - Connecting to: $SERVER_ADDRESS"
        echo "   - Web interface: port 8080"
        echo "   - Will sync from bootstrap server"
        echo ""
        
        # Build the project first
        echo "🔨 Building blockchain project..."
        cargo build --release
        
        echo "🔗 Connecting to bootstrap server..."
        cargo run --release --bin edunet-gui -- --connect $SERVER_ADDRESS
        ;;
        
    "help"|"--help"|"-h")
        echo "EduNet Blockchain Network Launcher"
        echo ""
        echo "Usage:"
        echo "  ./launch.sh bootstrap                    # Run as bootstrap server"
        echo "  ./launch.sh client <server:port>        # Connect to existing network"
        echo ""
        echo "Examples:"
        echo "  ./launch.sh bootstrap                    # Start new network"
        echo "  ./launch.sh client 192.168.1.100:8333   # Join existing network"
        echo ""
        echo "Network Architecture:"
        echo "  - Bootstrap server: The initial network seed (runs on port 8333)"
        echo "  - Client nodes: Connect to bootstrap to join network"
        echo "  - Web interface: Available on port 8080 for all nodes"
        echo ""
        exit 0
        ;;
        
    *)
        echo "❌ Invalid mode: $MODE"
        echo "Run './launch.sh help' for usage information"
        exit 1
        ;;
esac