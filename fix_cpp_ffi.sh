#!/bin/bash
# Fix C++ FFI Linker Issues
# This script resolves duplicate symbol errors in the hybrid architecture

set -e

echo "🔧 Fixing C++ FFI Linker Issues"
echo "================================"
echo ""

echo "Step 1: Rebuild C++ core with separated FFI..."
cd cpp-core/build
rm -rf *
cmake .. \
    -DCMAKE_BUILD_TYPE=Release \
    -DBUILD_SHARED_LIBS=OFF \
    -DBUILD_TESTS=OFF \
    -DBUILD_FFI_SEPARATE=ON  # Build FFI as separate library without core objects
make clean
make -j$(nproc)

echo ""
echo "Step 2: Verify libraries built..."
ls -lh libblockchain_core.a libblockchain_ffi.a

echo ""
echo "Step 3: Test FFI compilation..."
cd ../../rust-system/blockchain-ffi
cargo clean
cargo build --release

if [ $? -eq 0 ]; then
    echo "✅ FFI compilation successful!"
else
    echo "❌ FFI compilation failed"
    echo ""
    echo "Alternative: Merge libraries into single libblockchain.a"
    cd ../../cpp-core/build
    ar -M <<MRI
CREATE libblockchain.a
ADDLIB libblockchain_core.a
ADDLIB libblockchain_ffi.a
SAVE
END
MRI
    ranlib libblockchain.a
    echo "✅ Created merged libblockchain.a"
fi

echo ""
echo "Step 4: Build blockchain-node with C++ hybrid..."
cd ../../
cargo build --release --bin blockchain-node --features cpp-hybrid

echo ""
echo "✅ Hybrid architecture ready!"
echo ""
echo "To enable C++ crypto in miner:"
echo "1. Edit blockchain-node/src/miner.rs"
echo "2. Uncomment lines marked with TODO"
echo "3. Rebuild: cargo build --release --bin blockchain-node"
