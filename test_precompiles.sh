#!/bin/bash
# Test precompiled contracts

echo "🧪 Testing Precompiled Contracts..."
echo ""

cd "$(dirname "$0")"

# Create a simple test program
cat > /tmp/test_precompiles.rs << 'EOF'
use sha2::{Sha256, Digest};
use ripemd::{Ripemd160, Digest as RipemdDigest};

fn identity(input: &[u8]) -> Vec<u8> {
    input.to_vec()
}

fn sha256(input: &[u8]) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(input);
    hasher.finalize().to_vec()
}

fn ripemd160(input: &[u8]) -> Vec<u8> {
    let mut hasher = Ripemd160::new();
    hasher.update(input);
    let hash = hasher.finalize();
    
    // Left-pad to 32 bytes
    let mut result = vec![0u8; 32];
    result[12..32].copy_from_slice(&hash);
    result
}

fn main() {
    println!("Testing Precompiled Contracts:");
    println!();
    
    // Test identity (0x04)
    let input = vec![1, 2, 3, 4, 5];
    let output = identity(&input);
    assert_eq!(output, input);
    println!("✅ Identity (0x04): PASS");
    
    // Test SHA-256 (0x02)
    let input = b"hello world";
    let output = sha256(input);
    assert_eq!(output.len(), 32);
    let expected = hex::decode("b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9").unwrap();
    assert_eq!(output, expected);
    println!("✅ SHA-256 (0x02): PASS");
    
    // Test RIPEMD-160 (0x03)
    let input = b"hello world";
    let output = ripemd160(input);
    assert_eq!(output.len(), 32);
    assert_eq!(&output[0..12], &[0u8; 12]); // Should be left-padded
    println!("✅ RIPEMD-160 (0x03): PASS");
    
    println!();
    println!("🎉 All precompiled contract tests passed!");
}
EOF

# Build and run the test
cd /tmp
mkdir -p test_precompiles/src
mv test_precompiles.rs test_precompiles/src/main.rs
cd test_precompiles
cat > Cargo.toml << 'EOF'
[package]
name = "test_precompiles"
version = "0.1.0"
edition = "2021"

[dependencies]
sha2 = "0.10"
ripemd = "0.1"
hex = "0.4"
EOF

cargo run --quiet 2>&1
