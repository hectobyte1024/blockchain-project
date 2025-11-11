// Real Transaction Creator - Sends actual transactions and mines blocks
// This populates the blockchain with real activity

use blockchain_core::{
    genesis::{GenesisCreator, GenesisState},
    transaction::{Transaction, TransactionInput, TransactionOutput},
    block::{Block, BlockHeader},
    utxo::UTXOSet,
    hd_wallet::HDWallet,
};
use sha2::{Sha256, Digest};
use std::time::{SystemTime, UNIX_EPOCH};

fn main() -> anyhow::Result<()> {
    println!("\n🚀 EduNet Real Transaction Demo");
    println!("=================================\n");

    // Step 1: Create Genesis
    println!("📦 Step 1: Creating Genesis Block...");
    let creator = GenesisCreator::new(None);
    let mut genesis_state = creator.create_genesis_state()?;
    genesis_state.verify()?;
    
    let total_supply = genesis_state.get_total_supply_edu();
    println!("   ✅ Genesis created with {:.2} EDU total supply", total_supply);
    println!("   ✅ Block height: {}", genesis_state.genesis_block.header.height);
    
    // Display genesis accounts
    println!("\n💰 Genesis Accounts:");
    for account in &genesis_state.config.initial_accounts {
        let balance = genesis_state.utxo_set.get_balance(&account.address);
        println!("   {} - {:.2} EDU - {}", 
            &account.address[..20], 
            balance as f64 / 1e8,
            account.description
        );
    }

    // Step 2: Create User Wallets
    println!("\n👥 Step 2: Creating User Wallets...");
    
    let alice_wallet = HDWallet::new(Some("alice secret seed phrase for testing blockchain".to_string()))?;
    let alice_keypair = alice_wallet.derive_keypair(0)?;
    let alice_address = alice_keypair.get_address();
    println!("   👩 Alice: {}", alice_address);
    
    let bob_wallet = HDWallet::new(Some("bob secret seed phrase for testing blockchain".to_string()))?;
    let bob_keypair = bob_wallet.derive_keypair(0)?;
    let bob_address = bob_keypair.get_address();
    println!("   👨 Bob: {}", bob_address);
    
    let carol_wallet = HDWallet::new(Some("carol secret seed phrase for testing blockchain".to_string()))?;
    let carol_keypair = carol_wallet.derive_keypair(0)?;
    let carol_address = carol_keypair.get_address();
    println!("   👩 Carol: {}", carol_address);

    // Step 3: Send initial distribution from Genesis Pool
    println!("\n💸 Step 3: Initial Token Distribution...");
    println!("   Distributing from Genesis Pool to users...\n");
    
    let genesis_pool_addr = "edu1qGenesis00000000000000000000";
    let mut blockchain = vec![genesis_state.genesis_block.clone()];
    let mut utxo_set = genesis_state.utxo_set.clone();
    
    // Find Genesis Pool UTXOs
    let genesis_utxos = utxo_set.get_utxos_for_address(genesis_pool_addr);
    if genesis_utxos.is_empty() {
        println!("❌ No UTXOs found for genesis pool!");
        return Ok(());
    }
    
    println!("   Found {} UTXO(s) for Genesis Pool", genesis_utxos.len());
    let genesis_utxo = &genesis_utxos[0];
    let genesis_balance = genesis_utxo.value();
    println!("   Genesis Pool balance: {:.2} EDU", genesis_balance as f64 / 1e8);

    // Create distribution transaction
    // Send: 100 EDU to Alice, 150 EDU to Bob, 200 EDU to Carol
    let distribution_tx = {
        let mut tx = Transaction::new(
            1,
            vec![TransactionInput {
                previous_output: genesis_utxo.outpoint().clone(),
                script_sig: vec![], // Genesis doesn't need signature
                sequence: 0xffffffff,
            }],
            vec![
                TransactionOutput::create_p2pkh(10000000000, &alice_address)?, // 100 EDU
                TransactionOutput::create_p2pkh(15000000000, &bob_address)?,   // 150 EDU
                TransactionOutput::create_p2pkh(20000000000, &carol_address)?, // 200 EDU
                // Change back to Genesis Pool
                TransactionOutput::create_p2pkh(
                    genesis_balance - 45000000000, // Return the rest
                    genesis_pool_addr
                )?,
            ],
        );
        tx
    };

    println!("   📤 Distribution Transaction:");
    println!("      → Alice:  100.00 EDU");
    println!("      → Bob:    150.00 EDU");
    println!("      → Carol:  200.00 EDU");
    println!("      → Change: {:.2} EDU back to pool", 
        (genesis_balance - 45000000000) as f64 / 1e8);

    // Mine Block 1 with distribution
    println!("\n⛏️  Step 4: Mining Block 1...");
    let block1 = mine_block(
        &blockchain.last().unwrap(),
        vec![distribution_tx.clone()],
        1,
    )?;
    
    println!("   ✅ Block 1 mined!");
    println!("      Hash: {}", hex::encode(block1.hash()?));
    println!("      Nonce: {}", block1.header.nonce);
    
    blockchain.push(block1);
    utxo_set.add_transaction(&distribution_tx, 1)?;
    
    // Check balances
    println!("\n💰 Balances After Distribution:");
    println!("   Alice: {:.2} EDU", utxo_set.get_balance(&alice_address) as f64 / 1e8);
    println!("   Bob:   {:.2} EDU", utxo_set.get_balance(&bob_address) as f64 / 1e8);
    println!("   Carol: {:.2} EDU", utxo_set.get_balance(&carol_address) as f64 / 1e8);

    // Step 5: Alice sends to Bob
    println!("\n💸 Step 5: Alice → Bob (25 EDU)...");
    
    let alice_utxos = utxo_set.get_utxos_for_address(&alice_address);
    let alice_utxo = &alice_utxos[0];
    
    let alice_to_bob_tx = {
        let mut tx = Transaction::new(
            1,
            vec![TransactionInput {
                previous_output: alice_utxo.outpoint().clone(),
                script_sig: vec![],
                sequence: 0xffffffff,
            }],
            vec![
                TransactionOutput::create_p2pkh(2500000000, &bob_address)?,    // 25 EDU to Bob
                TransactionOutput::create_p2pkh(7500000000, &alice_address)?,  // 75 EDU change
            ],
        );
        
        // Sign the transaction
        let tx_hash = tx.get_hash()?;
        let signature = alice_keypair.sign(&tx_hash)?;
        tx.inputs[0].script_sig = signature.to_vec();
        
        tx
    };
    
    println!("   📤 Transaction signed and ready");
    println!("      From: Alice");
    println!("      To: Bob");
    println!("      Amount: 25.00 EDU");

    // Mine Block 2
    println!("\n⛏️  Step 6: Mining Block 2...");
    let block2 = mine_block(
        &blockchain.last().unwrap(),
        vec![alice_to_bob_tx.clone()],
        2,
    )?;
    
    println!("   ✅ Block 2 mined!");
    println!("      Hash: {}", hex::encode(block2.hash()?));
    
    blockchain.push(block2);
    utxo_set.apply_transaction(&alice_to_bob_tx)?;
    
    println!("\n💰 Balances After Alice→Bob:");
    println!("   Alice: {:.2} EDU", utxo_set.get_balance(&alice_address) as f64 / 1e8);
    println!("   Bob:   {:.2} EDU", utxo_set.get_balance(&bob_address) as f64 / 1e8);
    println!("   Carol: {:.2} EDU", utxo_set.get_balance(&carol_address) as f64 / 1e8);

    // Step 7: Bob sends to Carol
    println!("\n💸 Step 7: Bob → Carol (50 EDU)...");
    
    let bob_utxos = utxo_set.get_utxos_for_address(&bob_address);
    
    // Bob has 2 UTXOs now (150 from distribution + 25 from Alice)
    // We'll use them both
    let mut bob_inputs = vec![];
    let mut bob_input_total = 0u64;
    
    for utxo in &bob_utxos {
        bob_inputs.push(TransactionInput {
            previous_output: utxo.outpoint().clone(),
            script_sig: vec![],
            sequence: 0xffffffff,
        });
        bob_input_total += utxo.value();
    }
    
    println!("   Using {} UTXOs totaling {:.2} EDU", bob_utxos.len(), bob_input_total as f64 / 1e8);
    
    let bob_to_carol_tx = {
        let mut tx = Transaction::new(
            1,
            bob_inputs,
            vec![
                TransactionOutput::create_p2pkh(5000000000, &carol_address)?,  // 50 EDU to Carol
                TransactionOutput::create_p2pkh(bob_input_total - 5000000000, &bob_address)?, // Change back to Bob
            ],
        );
        
        // Sign each input
        let tx_hash = tx.get_hash()?;
        let signature = bob_keypair.sign(&tx_hash)?;
        for input in &mut tx.inputs {
            input.script_sig = signature.to_vec();
        }
        
        tx
    };
    
    println!("   📤 Transaction signed");
    println!("      From: Bob");
    println!("      To: Carol");
    println!("      Amount: 50.00 EDU");

    // Mine Block 3
    println!("\n⛏️  Step 8: Mining Block 3...");
    let block3 = mine_block(
        &blockchain.last().unwrap(),
        vec![bob_to_carol_tx.clone()],
        3,
    )?;
    
    println!("   ✅ Block 3 mined!");
    println!("      Hash: {}", hex::encode(block3.hash()?));
    
    blockchain.push(block3);
    utxo_set.apply_transaction(&bob_to_carol_tx)?;
    
    println!("\n💰 Final Balances:");
    println!("   Alice: {:.2} EDU", utxo_set.get_balance(&alice_address) as f64 / 1e8);
    println!("   Bob:   {:.2} EDU", utxo_set.get_balance(&bob_address) as f64 / 1e8);
    println!("   Carol: {:.2} EDU", utxo_set.get_balance(&carol_address) as f64 / 1e8);

    // Summary
    println!("\n╔════════════════════════════════════════════╗");
    println!("║        Transaction Summary                 ║");
    println!("╚════════════════════════════════════════════╝");
    println!();
    println!("📊 Blockchain Status:");
    println!("   Blocks: {}", blockchain.len());
    println!("   Height: {}", blockchain.last().unwrap().header.height);
    println!("   Total Supply: {:.2} EDU", total_supply);
    println!();
    println!("📝 Transactions:");
    println!("   Block 0: Genesis (6 outputs)");
    println!("   Block 1: Distribution → 3 users");
    println!("   Block 2: Alice → Bob (25 EDU)");
    println!("   Block 3: Bob → Carol (50 EDU)");
    println!();
    println!("✅ All transactions successfully mined!");
    println!();

    Ok(())
}

/// Simple proof-of-work mining
fn mine_block(prev_block: &Block, transactions: Vec<Transaction>, height: u32) -> anyhow::Result<Block> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_secs() as u32;
    
    // Calculate merkle root from transactions
    let mut tx_hashes = Vec::new();
    for tx in &transactions {
        tx_hashes.push(tx.get_hash()?);
    }
    let merkle_root = if tx_hashes.is_empty() {
        [0u8; 32]
    } else {
        calculate_merkle_root(&tx_hashes)
    };
    
    let mut header = BlockHeader {
        version: 1,
        prev_block_hash: prev_block.hash()?,
        merkle_root,
        timestamp,
        difficulty_target: 0x1d00ffff, // Easy difficulty for demo
        nonce: 0,
        height,
    };
    
    // Mine until we find a valid hash (starts with 0x00)
    print!("   Mining");
    let mut attempts = 0;
    loop {
        let block = Block::new(header.clone(), transactions.clone());
        let hash = block.hash()?;
        
        // Check if hash meets difficulty (first 2 bytes should be 0x00)
        if hash[0] == 0x00 {
            println!(" Found! (attempts: {})", attempts);
            return Ok(block);
        }
        
        header.nonce += 1;
        attempts += 1;
        
        if attempts % 10000 == 0 {
            print!(".");
            std::io::Write::flush(&mut std::io::stdout())?;
        }
    }
}

fn calculate_merkle_root(hashes: &[[u8; 32]]) -> [u8; 32] {
    if hashes.is_empty() {
        return [0u8; 32];
    }
    
    if hashes.len() == 1 {
        return hashes[0];
    }
    
    let mut current_level = hashes.to_vec();
    
    while current_level.len() > 1 {
        let mut next_level = Vec::new();
        
        for chunk in current_level.chunks(2) {
            let combined = if chunk.len() == 2 {
                [chunk[0].to_vec(), chunk[1].to_vec()].concat()
            } else {
                [chunk[0].to_vec(), chunk[0].to_vec()].concat()
            };
            
            let mut hasher = Sha256::new();
            hasher.update(&combined);
            let result = hasher.finalize();
            let mut hash = [0u8; 32];
            hash.copy_from_slice(&result);
            next_level.push(hash);
        }
        
        current_level = next_level;
    }
    
    current_level[0]
}
