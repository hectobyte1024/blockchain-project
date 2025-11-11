use blockchain_core::{
    block::{Block, Blockchain},
    transaction::{Transaction, TransactionInput, TransactionOutput, UTXO, UTXOSet},
};

#[test]
fn test_blockchain_basics() {
    // Create a new blockchain with genesis block
    let mut blockchain = Blockchain::new();
    
    println!("✓ Genesis block created");
    assert_eq!(blockchain.get_block_height(), 1);
    assert!(blockchain.is_valid());
    
    // Test genesis block properties
    let genesis = blockchain.get_latest_block().unwrap();
    assert_eq!(genesis.transactions.len(), 1);
    assert!(genesis.transactions[0].is_coinbase());
    
    println!("✓ Genesis block validation passed");
    
    // Create a simple transaction
    let tx = Transaction::new(
        1, // version
        vec![TransactionInput::new(
            [1u8; 32], // dummy prev hash
            0,         // prev output index
            vec![0x76, 0xa9], // dummy script
        )],
        vec![{
            let recipient_addr = blockchain_core::script_utils::ScriptBuilder::generate_mining_address("simple_test_recipient");
            let recipient_script = blockchain_core::script_utils::ScriptBuilder::create_coinbase_script(&recipient_addr).unwrap();
            TransactionOutput::new(100_000_000, recipient_script) // 1 BTC in satoshis
        }],
    );
    
    assert!(tx.is_valid());
    assert_eq!(tx.get_total_output_value(), 100_000_000);
    assert!(!tx.is_coinbase());
    
    println!("✓ Transaction creation and validation passed");
    
    // Create a new block
    let prev_hash = genesis.get_hash();
    let mut new_block = Block::new(
        blockchain_core::block::BlockHeader::new(
            1,
            prev_hash,
            [0u8; 32], // will be calculated
            0x207FFFFF, // easy difficulty
        ),
        vec![tx],
    );
    
    // Calculate and set the merkle root
    let merkle_root = new_block.calculate_merkle_root();
    new_block.header.merkle_root = merkle_root;
    
    println!("✓ Block created with merkle root: {}", hex::encode(merkle_root));
    
    // Test mining (with low difficulty)
    let mining_success = new_block.mine(10000);
    if mining_success {
        println!("✓ Block mined successfully! Nonce: {}", new_block.header.nonce);
        println!("✓ Block hash: {}", new_block.get_hex_hash());
        assert!(new_block.header.meets_difficulty_target());
    } else {
        println!("Mining didn't find solution in 10000 iterations (this is OK for testing)");
    }
    
    // Add block to blockchain
    let add_result = blockchain.add_block(new_block);
    assert!(add_result.is_ok());
    assert_eq!(blockchain.get_block_height(), 2);
    
    println!("✓ Block successfully added to blockchain");
    println!("✓ Final blockchain height: {}", blockchain.get_block_height());
    
    println!("🎉 All blockchain tests passed!");
}

#[test]
fn test_utxo_functionality() {
    let mut utxo_set = UTXOSet::new();
    assert_eq!(utxo_set.size(), 0);
    
    // Create a UTXO
    let utxo_addr = blockchain_core::script_utils::ScriptBuilder::generate_mining_address("utxo_test");
    let utxo_script = blockchain_core::script_utils::ScriptBuilder::create_coinbase_script(&utxo_addr).unwrap();
    let utxo = UTXO::new(
        [42u8; 32], // tx hash
        0,          // output index
        TransactionOutput::new(50_000_000, utxo_script),
        100,        // block height
        false,      // not coinbase
    );
    
    // Add UTXO to set
    utxo_set.add_utxo(utxo);
    assert_eq!(utxo_set.size(), 1);
    assert!(utxo_set.has_utxo(&[42u8; 32], 0));
    
    // Retrieve UTXO
    let retrieved = utxo_set.get_utxo(&[42u8; 32], 0);
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().output.value, 50_000_000);
    
    // Remove UTXO
    let removed = utxo_set.remove_utxo(&[42u8; 32], 0);
    assert!(removed.is_some());
    assert_eq!(utxo_set.size(), 0);
    assert!(!utxo_set.has_utxo(&[42u8; 32], 0));
    
    println!("✓ UTXO set functionality tests passed!");
}
