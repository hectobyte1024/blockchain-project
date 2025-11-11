use blockchain_core::{
    block::{Block, Blockchain, BlockHeader, mining, validation},
    transaction::{Transaction, TransactionInput, TransactionOutput, UTXOSet, TransactionBuilder},
    types::Hash256Wrapper,
    Result,
};

#[tokio::test]
async fn test_complete_blockchain_flow() -> Result<()> {
    println!("🧪 Testing Complete Blockchain Flow");
    
    // Initialize blockchain
    let mut blockchain = Blockchain::new();
    blockchain.initialize_genesis();
    println!("✓ Genesis block created and applied");
    
    assert_eq!(blockchain.get_height(), 1);
    assert!(blockchain.validate_chain());
    
    // Get genesis block details
    let genesis = blockchain.get_block(0).unwrap();
    println!("✓ Genesis block hash: {}", genesis.get_hash_string());
    println!("✓ Genesis coinbase reward: {}", genesis.get_block_reward());
    
    // Create a new transaction
    let genesis_hash = genesis.get_hash();
    let coinbase_tx = &genesis.transactions[0];
    
    // Create a simple transaction spending from genesis coinbase
    let tx_input = TransactionInput::new(
        Hash256Wrapper::from_string(&coinbase_tx.get_txid()),
        0, // First output
        vec![0x76, 0xa9], // Simplified script_sig
    );
    
    let tx_output1 = TransactionOutput::create_p2pkh(2_000_000_000, "address1")?; // 20 BTC
    let tx_output2 = TransactionOutput::create_p2pkh(2_999_999_000, "address2")?; // 29.99... BTC (with fee)
    
    let transaction = Transaction::new(2, vec![tx_input], vec![tx_output1, tx_output2]);
    println!("✓ Created transaction with {} inputs, {} outputs", 
             transaction.inputs.len(), transaction.outputs.len());
    
    // Create block template
    let prev_hash = genesis.get_hash();
    let mut block = Block::create_block_template(
        prev_hash,
        vec![transaction],
        "miner_address",
        0x207FFFFF, // Easy difficulty for testing
    )?;
    
    println!("✓ Created block template with {} transactions", block.get_transaction_count());
    
    // Mine the block
    println!("⛏️  Mining block...");
    let mining_result = mining::mine_block(&mut block, 100000);
    
    if mining_result.success {
        println!("✓ Block mined successfully!");
        println!("  - Nonce: {}", mining_result.nonce);
        println!("  - Hash: {}", mining_result.hash.to_hex());
        println!("  - Iterations: {}", mining_result.iterations);
        println!("  - Hash rate: {:.2} H/s", mining_result.hash_rate);
        
        // Add block to blockchain
        blockchain.add_block(block)?;
        println!("✓ Block added to blockchain");
        
        assert_eq!(blockchain.get_height(), 2);
        assert!(blockchain.validate_chain());
        
    } else {
        println!("⚠️  Mining failed within iteration limit");
    }
    
    // Test blockchain statistics
    let stats = blockchain.get_statistics();
    println!("📊 Blockchain Statistics:");
    println!("  - Height: {}", stats.height);
    println!("  - Total transactions: {}", stats.total_transactions);
    println!("  - Total value: {} satoshis", stats.total_value);
    println!("  - Average block time: {:.2} seconds", stats.average_block_time);
    println!("  - Current difficulty: {:.2}", stats.current_difficulty);
    println!("  - UTXO count: {}", stats.utxo_count);
    
    Ok(())
}

#[tokio::test]
async fn test_utxo_management() -> Result<()> {
    println!("🧪 Testing UTXO Management");
    
    let mut utxo_set = UTXOSet::new();
    
    // Create proper test address and script
    let test_address = blockchain_core::script_utils::ScriptBuilder::generate_mining_address("test_utxo");
    let test_script = blockchain_core::script_utils::ScriptBuilder::create_coinbase_script(&test_address)
        .unwrap_or_else(|_| vec![0x76, 0xa9, 0x14, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x88, 0xac]);
    
    // Create test UTXO
    let utxo = blockchain_core::transaction::UTXO::new(
        Hash256Wrapper::from_string("test_tx"),
        0,
        TransactionOutput::new(100_000_000, test_script),
        100,
        false,
    );
    
    // Add UTXO
    utxo_set.add_utxo(utxo.clone());
    println!("✓ Added UTXO to set");
    
    assert_eq!(utxo_set.size(), 1);
    assert_eq!(utxo_set.get_total_value(), 100_000_000);
    
    // Check UTXO exists
    let tx_hash = Hash256Wrapper::from_string("test_tx");
    assert!(utxo_set.has_utxo(&tx_hash, 0));
    
    let retrieved = utxo_set.get_utxo(&tx_hash, 0);
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().output.value, 100_000_000);
    println!("✓ Retrieved UTXO from set");
    
    // Test transaction application  
    let recipient1_addr = blockchain_core::script_utils::ScriptBuilder::generate_mining_address("recipient1");
    let recipient2_addr = blockchain_core::script_utils::ScriptBuilder::generate_mining_address("recipient2");
    let script1 = blockchain_core::script_utils::ScriptBuilder::create_coinbase_script(&recipient1_addr).unwrap();
    let script2 = blockchain_core::script_utils::ScriptBuilder::create_coinbase_script(&recipient2_addr).unwrap();
    
    let input = TransactionInput::new(tx_hash.clone(), 0, vec![]);
    let output1 = TransactionOutput::new(50_000_000, script1);
    let output2 = TransactionOutput::new(49_999_000, script2); // 1000 sat fee
    
    let tx = Transaction::new(2, vec![input], vec![output1, output2]);
    
    utxo_set.apply_transaction(&tx, 101)?;
    println!("✓ Applied transaction to UTXO set");
    
    // Original UTXO should be spent
    assert!(!utxo_set.has_utxo(&tx_hash, 0));
    
    // New UTXOs should exist
    let new_tx_hash = Hash256Wrapper::from_string(&tx.get_txid());
    assert!(utxo_set.has_utxo(&new_tx_hash, 0));
    assert!(utxo_set.has_utxo(&new_tx_hash, 1));
    
    assert_eq!(utxo_set.size(), 2);
    assert_eq!(utxo_set.get_total_value(), 99_999_000); // Original minus fee
    
    println!("✓ UTXO set correctly updated");
    
    Ok(())
}

#[tokio::test]
async fn test_transaction_builder() -> Result<()> {
    println!("🧪 Testing Transaction Builder");
    
    // Create mock private/public key pair
    let private_key = blockchain_core::types::PrivateKeyWrapper::new(vec![1u8; 32]);
    let public_key = blockchain_core::types::PublicKeyWrapper::new(vec![2u8; 33]);
    
    // Create previous output to spend
    let prev_addr = blockchain_core::script_utils::ScriptBuilder::generate_mining_address("prev_output");
    let prev_script = blockchain_core::script_utils::ScriptBuilder::create_coinbase_script(&prev_addr).unwrap();
    let prev_output = TransactionOutput::new(100_000_000, prev_script);
    
    // Build transaction
    let tx = TransactionBuilder::new(2)
        .add_input(
            Hash256Wrapper::from_string("prev_tx"),
            0,
            prev_output,
            private_key,
            public_key,
        )
        .add_output("recipient_address".to_string(), 50_000_000)?
        .set_fee_rate(1000) // 1 sat/vB
        .set_locktime(0)
        .finalize_with_change("change_address".to_string())?
        .build()?;
    
    println!("✓ Built transaction with transaction builder");
    println!("  - Inputs: {}", tx.inputs.len());
    println!("  - Outputs: {}", tx.outputs.len());
    println!("  - Total output value: {}", tx.get_total_output_value());
    println!("  - Estimated size: {} bytes", tx.estimate_size());
    
    assert!(tx.is_valid());
    assert_eq!(tx.inputs.len(), 1);
    assert!(tx.outputs.len() >= 1); // At least recipient, maybe change
    
    Ok(())
}

#[tokio::test]
async fn test_block_validation() -> Result<()> {
    println!("🧪 Testing Block Validation");
    
    // Create valid block
    let mut block = Block::create_genesis_block("Test validation");
    println!("✓ Created test block");
    
    // Test structure validation
    assert!(block.validate_structure());
    println!("✓ Block structure validation passed");
    
    // Test Merkle root validation
    assert!(block.validate_merkle_root());
    println!("✓ Merkle root validation passed");
    
    // Test size validation
    assert!(validation::validate_block_size(&block));
    println!("✓ Block size validation passed");
    
    // Test proof of work (should fail with easy difficulty)
    block.header.difficulty_target = 0x207FFFFF; // Very easy
    if !block.validate_proof_of_work() {
        // Mine to make it valid
        let mining_result = mining::mine_block(&mut block, 10000);
        if mining_result.success {
            assert!(block.validate_proof_of_work());
            println!("✓ Proof of work validation passed after mining");
        } else {
            println!("⚠️  Could not mine valid proof of work in time limit");
        }
    }
    
    // Test complete validation
    let utxo_set = UTXOSet::new();
    if validation::validate_block_complete(&block, None, &utxo_set, 0) {
        println!("✓ Complete block validation passed");
    }
    
    Ok(())
}

#[tokio::test]
async fn test_mining_performance() -> Result<()> {
    println!("🧪 Testing Mining Performance");
    
    let mut block = Block::create_genesis_block("Performance test");
    
    // Test different difficulty levels
    let difficulties = vec![
        (0x207FFFFF, "Very Easy"),
        (0x1F7FFFFF, "Easy"),
        (0x1E7FFFFF, "Medium"),
    ];
    
    for (difficulty, name) in difficulties {
        block.header.difficulty_target = difficulty;
        block.header.nonce = 0; // Reset nonce
        
        println!("⛏️  Mining with {} difficulty (0x{:08X})", name, difficulty);
        
        let start_time = std::time::Instant::now();
        let result = mining::mine_block(&mut block, 50000);
        let elapsed = start_time.elapsed();
        
        if result.success {
            println!("  ✓ Success in {:.3}s", elapsed.as_secs_f64());
            println!("    - Nonce found: {}", result.nonce);
            println!("    - Iterations: {}", result.iterations);
            println!("    - Hash rate: {:.0} H/s", result.hash_rate);
            println!("    - Final hash: {}", result.hash.to_hex()[..16]);
        } else {
            println!("  ✗ Failed after {:.3}s ({} iterations)", elapsed.as_secs_f64(), result.iterations);
        }
        
        println!();
    }
    
    Ok(())
}

#[tokio::test]
async fn test_blockchain_reorganization() -> Result<()> {
    println!("🧪 Testing Blockchain Reorganization Simulation");
    
    // Create main chain
    let mut main_chain = Blockchain::new();
    main_chain.initialize_genesis();
    
    // Add a few blocks to main chain
    for i in 1..=3 {
        let prev_hash = main_chain.get_latest_block().unwrap().get_hash();
        let mut block = Block::create_block_template(
            prev_hash,
            vec![], // Empty transactions for simplicity
            "main_miner",
            0x207FFFFF,
        )?;
        
        // Mine the block
        let result = mining::mine_block(&mut block, 10000);
        if result.success {
            main_chain.add_block(block)?;
            println!("✓ Added block {} to main chain", i);
        }
    }
    
    println!("✓ Main chain height: {}", main_chain.get_height());
    println!("✓ Main chain total work: {:.2}", main_chain.get_total_work());
    
    // Simulate fork - create alternative chain from genesis
    let mut fork_chain = Blockchain::new();
    fork_chain.initialize_genesis();
    
    // Add blocks to fork with easier difficulty (more blocks)
    for i in 1..=4 {
        let prev_hash = fork_chain.get_latest_block().unwrap().get_hash();
        let mut block = Block::create_block_template(
            prev_hash,
            vec![],
            "fork_miner",
            0x20FFFFFF, // Slightly easier
        )?;
        
        let result = mining::mine_block(&mut block, 10000);
        if result.success {
            fork_chain.add_block(block)?;
            println!("✓ Added block {} to fork", i);
        }
    }
    
    println!("✓ Fork chain height: {}", fork_chain.get_height());
    println!("✓ Fork chain total work: {:.2}", fork_chain.get_total_work());
    
    // Compare chains
    if fork_chain.get_total_work() > main_chain.get_total_work() {
        println!("🔄 Fork has more work - would trigger reorganization");
    } else {
        println!("📏 Main chain has more work - no reorganization needed");
    }
    
    Ok(())
}

#[tokio::test] 
async fn test_transaction_validation() -> Result<()> {
    println!("🧪 Testing Transaction Validation");
    
    // Test valid transaction
    let valid_input = TransactionInput::new(
        Hash256Wrapper::from_string("valid_tx"),
        0,
        vec![0x47, 0x30, 0x44], // Mock signature
    );
    
    let valid_addr = blockchain_core::script_utils::ScriptBuilder::generate_mining_address("valid_recipient");
    let valid_script = blockchain_core::script_utils::ScriptBuilder::create_coinbase_script(&valid_addr).unwrap();
    let valid_output = TransactionOutput::new(100_000, valid_script);
    let valid_tx = Transaction::new(2, vec![valid_input], vec![valid_output]);
    
    assert!(valid_tx.is_valid());
    println!("✓ Valid transaction passed validation");
    
    // Test invalid transactions
    
    // Empty inputs
    let empty_input_tx = Transaction::new(2, vec![], vec![valid_output.clone()]);
    assert!(!empty_input_tx.is_valid());
    println!("✓ Empty inputs correctly rejected");
    
    // Empty outputs  
    let empty_output_tx = Transaction::new(2, vec![valid_input.clone()], vec![]);
    assert!(!empty_output_tx.is_valid());
    println!("✓ Empty outputs correctly rejected");
    
    // Dust output
    let dust_addr = blockchain_core::script_utils::ScriptBuilder::generate_mining_address("dust_recipient");
    let dust_script = blockchain_core::script_utils::ScriptBuilder::create_coinbase_script(&dust_addr).unwrap();
    let dust_output = TransactionOutput::new(100, dust_script); // Below dust threshold
    assert!(!dust_output.is_valid());
    println!("✓ Dust output correctly rejected");
    
    // Test coinbase transaction
    let coinbase = Transaction::create_coinbase(
        5_000_000_000,
        100_000,
        "miner",
        b"coinbase data".to_vec(),
    )?;
    
    assert!(coinbase.is_coinbase());
    assert!(coinbase.is_valid());
    println!("✓ Coinbase transaction validation passed");
    
    Ok(())
}

#[tokio::test]
async fn test_difficulty_adjustment() -> Result<()> {
    println!("🧪 Testing Difficulty Adjustment");
    
    let mut blockchain = Blockchain::new();
    blockchain.initialize_genesis();
    
    let initial_difficulty = blockchain.get_current_difficulty();
    println!("✓ Initial difficulty: 0x{:08X}", initial_difficulty);
    
    // In a real implementation, difficulty would adjust based on block times
    // Here we just test the interface
    blockchain.set_difficulty_target(0x1E00FFFF);
    let new_difficulty = blockchain.get_current_difficulty();
    
    assert_ne!(initial_difficulty, new_difficulty);
    println!("✓ Difficulty adjustment interface works");
    println!("  - Old: 0x{:08X}", initial_difficulty);
    println!("  - New: 0x{:08X}", new_difficulty);
    
    Ok(())
}

// Helper function to run all tests
pub async fn run_all_blockchain_tests() -> Result<()> {
    println!("🚀 Running Comprehensive Blockchain Tests\n");
    
    test_complete_blockchain_flow().await?;
    println!();
    
    test_utxo_management().await?;
    println!();
    
    test_transaction_builder().await?;
    println!();
    
    test_block_validation().await?;
    println!();
    
    test_mining_performance().await?;
    println!();
    
    test_blockchain_reorganization().await?;
    println!();
    
    test_transaction_validation().await?;
    println!();
    
    test_difficulty_adjustment().await?;
    
    println!("\n🎉 All blockchain tests completed successfully!");
    
    Ok(())
}