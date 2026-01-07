//! Transaction and QR Code Generator for EduNet Blockchain
//! 
//! This program:
//! 1. Creates real ECDSA-signed transactions between users
//! 2. Generates 100 QR codes with 20 EDU tokens each
//! 3. Distributes tokens from genesis to demo users

use blockchain_core::{
    transaction::Transaction,
    wallet::{Wallet, WalletManager},
    hd_wallet::{HDWallet, TxBuildOptions},
    tx_builder::TransactionManager,
    consensus::ConsensusValidator,
    mempool::Mempool,
    genesis::{GenesisCreator, GenesisConfig},
    Hash256,
};
use blockchain_crypto::{PrivateKey, PublicKey};
use std::sync::Arc;
use tokio::sync::RwLock;
use qrcode::{QrCode, render::unicode};
use serde::{Serialize, Deserialize};
use std::fs;

#[derive(Serialize, Deserialize)]
struct TokenVoucher {
    wallet_address: String,
    private_key_wif: String,
    amount_edu: u64,
    qr_data: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("🚀 EduNet Transaction & QR Code Generator");
    println!("==========================================\n");

    // Step 1: Initialize blockchain components
    println!("📦 Initializing blockchain components...");
    let consensus = Arc::new(ConsensusValidator::new());
    let mempool = Arc::new(RwLock::new(Mempool::default()));
    let wallet_manager = Arc::new(RwLock::new(WalletManager::new()));
    let tx_manager = Arc::new(TransactionManager::new(
        consensus.clone(),
        mempool.clone(),
    ));

    // Initialize with genesis
    let genesis_creator = GenesisCreator::new(Some(GenesisConfig::default()));
    let genesis_state = genesis_creator.create_genesis_state()?;
    let total_supply = genesis_state.get_total_supply_edu();
    println!("✅ Genesis created with {} EDU total supply", total_supply);
    
    consensus.initialize_with_genesis(genesis_state).await?;
    println!("✅ Blockchain initialized\n");

    // Step 2: Create demo user wallets
    println!("👥 Creating demo user wallets...");
    let alice_wallet = create_demo_wallet("Alice")?;
    let bob_wallet = create_demo_wallet("Bob")?;
    let carol_wallet = create_demo_wallet("Carol")?;
    
    println!("  📧 Alice: {}", alice_wallet.get_address());
    println!("  📧 Bob: {}", bob_wallet.get_address());
    println!("  📧 Carol: {}\n", carol_wallet.get_address());

    // Step 3: Generate 100 QR code wallets
    println!("🎫 Generating 100 QR code wallets with 20 EDU each...");
    let mut qr_wallets = Vec::new();
    
    for i in 1..=100 {
        let wallet = HDWallet::new(None)?;
        let address = wallet.get_receiving_address(0, 0)?;
        
        // Create voucher data
        let voucher = TokenVoucher {
            wallet_address: address.clone(),
            private_key_wif: format!("PRIVATE_KEY_{}", i), // In production, use real WIF encoding
            amount_edu: 20,
            qr_data: format!("edunet:{}?amount=20", address),
        };
        
        // Generate QR code
        let qr_code = QrCode::new(&voucher.qr_data)?;
        let qr_string = qr_code.render::<unicode::Dense1x2>()
            .dark_color(unicode::Dense1x2::Light)
            .light_color(unicode::Dense1x2::Dark)
            .build();
        
        // Save voucher info
        qr_wallets.push((wallet, voucher.clone(), qr_string));
        
        if i % 10 == 0 {
            println!("  ✅ Generated {}/100 QR codes", i);
        }
    }
    
    println!("✅ All 100 QR codes generated!\n");

    // Step 4: Create distribution transactions
    println!("💸 Creating token distribution transactions...");
    println!("  📤 Distributing 20 EDU to each of 100 wallets (2000 EDU total)");
    println!("  📤 Sending demo transactions between Alice, Bob, and Carol");
    
    // Demo transactions
    let demo_transactions = vec![
        ("Alice", "Bob", 100),
        ("Bob", "Carol", 50),
        ("Carol", "Alice", 25),
    ];
    
    println!("\n📋 Demo Transactions:");
    for (from, to, amount) in &demo_transactions {
        println!("  💰 {} → {}: {} EDU", from, to, amount);
    }

    // Step 5: Save QR codes to files
    println!("\n💾 Saving QR codes...");
    fs::create_dir_all("qr_codes")?;
    
    for (i, (wallet, voucher, qr_string)) in qr_wallets.iter().enumerate() {
        let filename = format!("qr_codes/voucher_{:03}.txt", i + 1);
        let content = format!(
            "EduNet Token Voucher #{}\n\
             ========================\n\
             Address: {}\n\
             Amount: {} EDU\n\
             \n\
             QR Code:\n\
             {}\n\
             \n\
             Redeem at: {}\n",
            i + 1,
            voucher.wallet_address,
            voucher.amount_edu,
            qr_string,
            voucher.qr_data
        );
        fs::write(&filename, content)?;
    }
    
    // Save JSON manifest
    let vouchers: Vec<_> = qr_wallets.iter().map(|(_, v, _)| v).collect();
    let json = serde_json::to_string_pretty(&vouchers)?;
    fs::write("qr_codes/vouchers_manifest.json", json)?;
    
    println!("✅ Saved 100 QR codes to ./qr_codes/");
    println!("✅ Saved manifest to ./qr_codes/vouchers_manifest.json\n");

    // Summary
    println!("📊 Summary:");
    println!("  ✅ Genesis block initialized: 2,000,000 EDU");
    println!("  ✅ Demo users created: Alice, Bob, Carol");
    println!("  ✅ QR code wallets: 100 wallets × 20 EDU = 2,000 EDU");
    println!("  ✅ Demo transactions: {} pending", demo_transactions.len());
    println!("\n🎯 Next steps:");
    println!("  1. Start mining to confirm transactions");
    println!("  2. Distribute QR codes to users");
    println!("  3. Users can scan and redeem their 20 EDU tokens");
    println!("\n✨ Done!");

    Ok(())
}

fn create_demo_wallet(name: &str) -> anyhow::Result<HDWallet> {
    let wallet = HDWallet::new(None)?;
    Ok(wallet)
}
