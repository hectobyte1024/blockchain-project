#!/bin/bash
# Safe Project Cleanup Script
# Removes obsolete files while preserving C++ core and active components

set -e

echo "🧹 EduNet Blockchain Project Cleanup"
echo "====================================="
echo ""
echo "This script will remove:"
echo "  - Deprecated edunet-gui (replaced by blockchain-node + edunet-web)"
echo "  - Obsolete documentation (25+ markdown files)"
echo "  - Old shell scripts (20+ scripts for old architecture)"
echo "  - Demo/test directories"
echo "  - Temporary files (logs, PIDs)"
echo ""
echo "This script will PRESERVE:"
echo "  ✅ cpp-core/ (C++ hybrid implementation)"
echo "  ✅ rust-system/ (Rust FFI layer)"
echo "  ✅ blockchain-node/ (current mining node)"
echo "  ✅ edunet-web/ (current web client)"
echo "  ✅ voucher-pdf-gen/ (voucher QR generation)"
echo "  ✅ blockchain-rpc/ (RPC layer)"
echo "  ✅ Active voucher files"
echo "  ✅ README.md and BLOCKCHAIN-CORE-ARCHITECTURE.md"
echo ""
read -p "Continue? (y/N) " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Cleanup cancelled."
    exit 0
fi

# Create backup directory
BACKUP_DIR="cleanup_backup_$(date +%Y%m%d_%H%M%S)"
mkdir -p "$BACKUP_DIR"
echo "📦 Creating backup in $BACKUP_DIR/"

# Function to safely remove with backup
safe_remove() {
    local item="$1"
    if [ -e "$item" ]; then
        echo "  🗑️  Removing: $item"
        mv "$item" "$BACKUP_DIR/"
    fi
}

echo ""
echo "Step 1: Removing deprecated edunet-gui..."
safe_remove "edunet-gui"

echo ""
echo "Step 2: Removing demo/test directories..."
safe_remove "rpc_demo"
safe_remove "real-tx-demo"
safe_remove "tx-qr-gen"
safe_remove "blockchain-data"
safe_remove "test-blockchain-data"
safe_remove "shared"
safe_remove "tests"  # If empty or test-only
safe_remove "docs"   # If contains old docs

echo ""
echo "Step 3: Removing obsolete documentation..."
safe_remove "ARCHITECTURE-REFACTOR.md"
safe_remove "DEPLOYMENT-CHECKLIST.md"
safe_remove "DEPLOYMENT-GUIDE-ISP.md"
safe_remove "DEPLOYMENT-GUIDE.md"
safe_remove "DEPLOYMENT-SUMMARY.md"
safe_remove "DNS-SETUP.md"
safe_remove "FINAL-DEPLOYMENT-READY.md"
safe_remove "FRONTEND-INTEGRATION-COMPLETE.md"
safe_remove "GENESIS-UPDATE.md"
safe_remove "HOME-SERVER-SETUP.md"
safe_remove "IMPLEMENTATION-SUMMARY.md"
safe_remove "LAUNCH-PLAN.md"
safe_remove "MULTI-USER-SYSTEM.md"
safe_remove "PRODUCTION-READY.md"
safe_remove "PRODUCTION-STATUS.md"
safe_remove "PROJECT-COMPLETE.md"
safe_remove "REMOTE-DEPLOYMENT-GUIDE.md"
safe_remove "ROUTER-SETUP.md"
safe_remove "SEPARATED-ARCHITECTURE-COMPLETE.md"
safe_remove "SEPARATED-ARCHITECTURE-README.md"
safe_remove "SYSTEM-FLOW-DOCUMENTATION.md"
safe_remove "TODO-IMPROVEMENTS.md"
safe_remove "TRANSACTION-PROCESSING-COMPLETE.md"

echo ""
echo "Step 4: Removing obsolete shell scripts..."
safe_remove "launch.sh"
safe_remove "multi_user_demo.sh"
safe_remove "deploy-to-server.sh"
safe_remove "home-server-deploy.sh"
safe_remove "deploy-home-server.sh"
safe_remove "quick-deploy.sh"
safe_remove "send_real_transactions.sh"
safe_remove "build.sh"
safe_remove "consensus_demo.sh"
safe_remove "create_demo_transactions.sh"
safe_remove "create_transactions.sh"
safe_remove "demo.sh"
safe_remove "deploy-separated.sh"
safe_remove "deploy-to-remote-server.sh"
safe_remove "live_deployment_demo.sh"
safe_remove "multi_node_demo.sh"
safe_remove "networking_demo.sh"
safe_remove "run-node.sh"
safe_remove "setup-local-server.sh"
safe_remove "test-separated-locally.sh"
safe_remove "test_complete_flow.sh"
safe_remove "test_send_transaction.sh"
safe_remove "test_transaction_processing.sh"

echo ""
echo "Step 5: Removing temporary/log files..."
safe_remove "blockchain-node.log"
safe_remove "blockchain-node.pid"
safe_remove "edunet-web.log"
safe_remove "edunet-web.pid"
safe_remove "server.log"
safe_remove "cookies.txt"
safe_remove "test-edunet-web.db"

echo ""
echo "Step 6: Removing standalone demo files..."
safe_remove "edunet_gui.ipynb"
safe_remove "create_real_genesis_transactions.rs"
safe_remove "live_demo.rs"
safe_remove "transaction_qr_generator.rs"
safe_remove "init_blockchain_db.py"
safe_remove "simple_test.html"
safe_remove "test.html"
safe_remove "test_transactions.html"

echo ""
echo "Step 7: Removing old voucher QR directories (keeping latest)..."
# Remove old voucher directories, keep only the latest one
find . -maxdepth 1 -type d -name "voucher-qr-codes-*" | sort | head -n -1 | while read dir; do
    safe_remove "$dir"
done
safe_remove "voucher-qr-codes"  # Remove generic one

echo ""
echo "Step 8: Updating Cargo.toml workspace..."
# Backup Cargo.toml
cp Cargo.toml "$BACKUP_DIR/Cargo.toml"

# Update workspace members
cat > Cargo.toml.new << 'CARGO_EOF'
[workspace]
resolver = "2"
members = [
    "rust-system/blockchain-ffi",       # C++ FFI bindings
    "rust-system/blockchain-core",      # Rust blockchain core
    "rust-system/blockchain-network",   # P2P networking
    "rust-system/blockchain-rpc",       # RPC interface
    "blockchain-node",                  # Mining node daemon
    "edunet-web",                       # Web client
    "voucher-pdf-gen",                  # Voucher QR generator
]

[workspace.package]
version = "0.1.0"
edition = "2021"
authors = ["EduNet Blockchain Team"]
license = "MIT OR Apache-2.0"
repository = "https://github.com/hectobyte1024/blockchain-project"

[workspace.dependencies]
# Async runtime and networking
tokio = { version = "1.35", features = ["full"] }
tokio-util = "0.7"
futures = "0.3"
async-trait = "0.1"

# Networking
libp2p = { version = "0.53", features = ["tcp", "noise", "mplex", "websocket", "ping", "mdns"] }

# HTTP server and client
axum = "0.7"
tower = "0.4"
tower-http = { version = "0.5", features = ["fs", "trace", "cors"] }
hyper = "1.0"
reqwest = { version = "0.11", features = ["json"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
bincode = "1.3"

# Cryptography
sha2 = "0.10"
hmac = "0.12"
secp256k1 = { version = "0.28", features = ["global-context", "rand-std"] }
rand = "0.8"
ed25519-dalek = "2.1"

# Database
sqlx = { version = "0.7", features = ["runtime-tokio-native-tls", "sqlite"] }

# Authentication
argon2 = "0.5"
jsonwebtoken = "9.2"

# Utilities
thiserror = "1.0"
anyhow = "1.0"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
uuid = { version = "1.6", features = ["v4", "serde"] }
hex = "0.4"
bs58 = "0.5"
chrono = "0.4"

# QR code generation
qrcode = "0.14"

# FFI
libc = "0.2"

# Build dependencies (for FFI)
bindgen = "0.69"
cmake = "0.1"
cbindgen = "0.26"
CARGO_EOF

mv Cargo.toml.new Cargo.toml

echo ""
echo "✅ Cleanup complete!"
echo ""
echo "📊 Summary:"
echo "  - Backup created: $BACKUP_DIR/"
echo "  - Removed: edunet-gui (deprecated monolithic system)"
echo "  - Removed: 23 obsolete markdown files"
echo "  - Removed: 24 obsolete shell scripts"
echo "  - Removed: 4 demo directories"
echo "  - Removed: 8 temporary files"
echo "  - Updated: Cargo.toml workspace"
echo ""
echo "✅ Preserved (Hybrid Architecture):"
echo "  - cpp-core/ (C++ performance layer)"
echo "  - rust-system/ (Rust FFI and core)"
echo "  - blockchain-node/ (mining daemon)"
echo "  - edunet-web/ (web client)"
echo "  - voucher-pdf-gen/ (QR generator)"
echo "  - README.md"
echo "  - BLOCKCHAIN-CORE-ARCHITECTURE.md"
echo ""
echo "📝 Next steps:"
echo "  1. Review backup: ls -la $BACKUP_DIR/"
echo "  2. Test build: cargo build --release"
echo "  3. If everything works, delete backup: rm -rf $BACKUP_DIR/"
echo "  4. Commit changes: git add -A && git commit -m 'Clean up obsolete files'"
echo ""
echo "⚠️  NOTE: The current implementation uses pure Rust."
echo "   To enable hybrid C++/Rust, you need to integrate blockchain-ffi"
echo "   into blockchain-node for performance-critical operations."
echo ""
