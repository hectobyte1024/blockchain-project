#!/bin/bash
# Quick test of real ECDSA signatures

cd "/home/hectobyte1024/Documents/blockchain project"

cat > /tmp/test_ecdsa.rs << 'EOF'
use blockchain_core::crypto::{generate_private_key, derive_public_key, sign_hash, verify_signature, sha256};

fn main() {
    println!("🔐 Testing Real ECDSA Cryptography\n");
    
    // Generate keys
    let private_key = generate_private_key().expect("Failed to generate key");
    let public_key = derive_public_key(&private_key).expect("Failed to derive pubkey");
    
    println!("✅ Generated keypair");
    println!("  Private key: {} bytes", private_key.len());
    println!("  Public key: {} bytes\n", public_key.len());
    
    // Sign message
    let message = b"Hello, blockchain with real ECDSA!";
    let hash = sha256(message);
    let signature = sign_hash(&hash, &private_key).expect("Failed to sign");
    
    println!("✅ Signed message");
    println!("  Message: {}", String::from_utf8_lossy(message));
    println!("  Signature: {} bytes\n", signature.len());
    
    // Verify signature
    let valid = verify_signature(&signature, &public_key, &hash).expect("Verification failed");
    
    if valid {
        println!("✅ Signature VALID - Real ECDSA working!");
    } else {
        println!("❌ Signature INVALID");
        std::process::exit(1);
    }
    
    // Test invalid signature
    let wrong_message = b"Different message";
    let wrong_hash = sha256(wrong_message);
    let invalid = verify_signature(&signature, &public_key, &wrong_hash).expect("Verification failed");
    
    if !invalid {
        println!("✅ Correctly rejected invalid signature\n");
    } else {
        println!("❌ SECURITY ISSUE: Accepted invalid signature!");
        std::process::exit(1);
    }
    
    println!("🎉 All crypto tests passed - Real ECDSA is working!");
}
EOF

echo "Compiling test..."
rustc --edition 2021 -L target/debug/deps /tmp/test_ecdsa.rs --extern blockchain_core=target/debug/libblockchain_core.rlib -o /tmp/test_ecdsa 2>&1 | grep -v "warning:" | head -5

if [ -f /tmp/test_ecdsa ]; then
    echo "Running test..."
    /tmp/test_ecdsa
else
    echo "Compilation failed"
fi
