use blockchain_core::wallet::WalletManager;
use sqlx::SqlitePool;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("\n🔐 Converting to REAL ECDSA Wallets\n{}", "=".repeat(80));
    
    let pool = SqlitePool::connect("sqlite:./edunet-gui/edunet.db").await?;
    let mut wm = WalletManager::new();
    
    println!("📝 Creating wallets...");
    
    // Create and clone wallet data (because create_wallet returns a reference)
    let treasury_addr = wm.create_wallet("Treasury".into())?.address.clone();
    let mining_addr = wm.create_wallet("Mining".into())?.address.clone();
    let loans_addr = wm.create_wallet("Loans".into())?.address.clone();
    let nft_addr = wm.create_wallet("NFT".into())?.address.clone();
    let invest_addr = wm.create_wallet("Investment".into())?.address.clone();
    let circ_addr = wm.create_wallet("Circulating".into())?.address.clone();
    
    let alice_wallet = wm.create_wallet("Alice".into())?;
    let alice_addr = alice_wallet.address.clone();
    let alice_key = alice_wallet.private_key.clone();
    
    let bob_wallet = wm.create_wallet("Bob".into())?;
    let bob_addr = bob_wallet.address.clone();
    let bob_key = bob_wallet.private_key.clone();
    
    let carol_wallet = wm.create_wallet("Carol".into())?;
    let carol_addr = carol_wallet.address.clone();
    let carol_key = carol_wallet.private_key.clone();
    
    println!("  ✓ Alice: {}", alice_addr);
    println!("  ✓ Bob: {}", bob_addr);
    println!("  ✓ Carol: {}", carol_addr);
    
    println!("\n💾 Saving keys...");
    sqlx::query("UPDATE users SET wallet_address=?, private_key=? WHERE username=?")
        .bind(&alice_addr).bind(hex::encode(&alice_key)).bind("alice")
        .execute(&pool).await?;
    sqlx::query("UPDATE users SET wallet_address=?, private_key=? WHERE username=?")
        .bind(&bob_addr).bind(hex::encode(&bob_key)).bind("bob")
        .execute(&pool).await?;
    sqlx::query("UPDATE users SET wallet_address=?, private_key=? WHERE username=?")
        .bind(&carol_addr).bind(hex::encode(&carol_key)).bind("carol")
        .execute(&pool).await?;
    println!("  ✓ Keys saved");
    
    println!("\n🔄 Converting addresses...");
    let map = vec![
        ("EDU_Treasury_Pool_2025", treasury_addr),
        ("EDU_Mining_Rewards_Pool", mining_addr),
        ("EDU_Student_Loan_Pool", loans_addr),
        ("EDU_NFT_Marketplace_Pool", nft_addr),
        ("EDU_Investment_Pool", invest_addr),
        ("EDU_Circulating_Supply", circ_addr),
        ("alice_wallet_0x1a2b3c", alice_addr),
        ("bob_wallet_0x4d5e6f", bob_addr),
        ("carol_wallet_0x7g8h9i", carol_addr),
    ];
    
    for (old, new) in map {
        sqlx::query("UPDATE transactions SET from_address=? WHERE from_address=?")
            .bind(&new).bind(old).execute(&pool).await?;
        sqlx::query("UPDATE transactions SET to_address=? WHERE to_address=?")
            .bind(&new).bind(old).execute(&pool).await?;
    }
    
    println!("  ✓ All addresses converted");
    println!("\n{}\n✅ NOW USING REAL ECDSA WALLETS\n{}", "=".repeat(80), "=".repeat(80));
    Ok(())
}
