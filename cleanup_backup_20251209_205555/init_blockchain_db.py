#!/usr/bin/env python3
"""
Initialize EduNet blockchain database with genesis and mock blocks
"""
import sqlite3
import json
import time

def init_db():
    conn = sqlite3.connect('edunet-gui/edunet.db')
    cursor = conn.cursor()
    
    # Run the production schema
    with open('edunet-gui/migrations/002_production_schema.sql', 'r') as f:
        schema_sql = f.read()
        # Execute each statement separately
        for statement in schema_sql.split(';'):
            if statement.strip():
                try:
                    cursor.execute(statement)
                except sqlite3.Error as e:
                    print(f"Error executing: {statement[:100]}...")
                    print(f"Error: {e}")
    
    # Genesis block
    genesis_time = 1700000000
    genesis_block = {
        'height': 0,
        'block_hash': '000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce26f',
        'prev_hash': '0000000000000000000000000000000000000000000000000000000000000000',
        'merkle_root': 'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855',
        'timestamp': genesis_time,
        'nonce': 2083236893,
        'difficulty': 1,
        'block_data': b'Genesis Block: EduNet Blockchain Network - 10M EDU Total Supply'
    }
    
    cursor.execute("""
        INSERT INTO blocks (height, block_hash, prev_hash, merkle_root, timestamp, nonce, difficulty, block_data)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
    """, (genesis_block['height'], genesis_block['block_hash'], genesis_block['prev_hash'],
          genesis_block['merkle_root'], genesis_block['timestamp'], genesis_block['nonce'],
          genesis_block['difficulty'], genesis_block['block_data']))
    
    # Genesis transaction - Treasury Pool
    cursor.execute("""
        INSERT INTO transactions (tx_hash, from_address, to_address, amount, fee, memo, 
                                 block_height, tx_index, timestamp, status)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    """, ('genesis_tx_treasury', 'COINBASE', 'EDU_Treasury_Pool_2025', 
          200000000000000, 0, 'Treasury Pool: 2,000,000 EDU', 0, 0, genesis_time, 'confirmed'))
    
    # Genesis transaction - Mining Rewards Pool
    cursor.execute("""
        INSERT INTO transactions (tx_hash, from_address, to_address, amount, fee, memo, 
                                 block_height, tx_index, timestamp, status)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    """, ('genesis_tx_mining', 'COINBASE', 'EDU_Mining_Rewards_Pool', 
          300000000000000, 0, 'Mining Rewards: 3,000,000 EDU', 0, 1, genesis_time, 'confirmed'))
    
    # Genesis transaction - Student Loan Pool
    cursor.execute("""
        INSERT INTO transactions (tx_hash, from_address, to_address, amount, fee, memo, 
                                 block_height, tx_index, timestamp, status)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    """, ('genesis_tx_loans', 'COINBASE', 'EDU_Student_Loan_Pool', 
          200000000000000, 0, 'Student Loans: 2,000,000 EDU', 0, 2, genesis_time, 'confirmed'))
    
    # Genesis transaction - NFT Marketplace Pool
    cursor.execute("""
        INSERT INTO transactions (tx_hash, from_address, to_address, amount, fee, memo, 
                                 block_height, tx_index, timestamp, status)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    """, ('genesis_tx_nft', 'COINBASE', 'EDU_NFT_Marketplace_Pool', 
          100000000000000, 0, 'NFT Marketplace: 1,000,000 EDU', 0, 3, genesis_time, 'confirmed'))
    
    # Genesis transaction - Investment Pool
    cursor.execute("""
        INSERT INTO transactions (tx_hash, from_address, to_address, amount, fee, memo, 
                                 block_height, tx_index, timestamp, status)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    """, ('genesis_tx_invest', 'COINBASE', 'EDU_Investment_Pool', 
          150000000000000, 0, 'Investment Pool: 1,500,000 EDU', 0, 4, genesis_time, 'confirmed'))
    
    # Genesis transaction - Circulating Supply
    cursor.execute("""
        INSERT INTO transactions (tx_hash, from_address, to_address, amount, fee, memo, 
                                 block_height, tx_index, timestamp, status)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    """, ('genesis_tx_circulating', 'COINBASE', 'EDU_Circulating_Supply', 
          250000000000000, 0, 'Initial Circulation: 2,500,000 EDU', 0, 5, genesis_time, 'confirmed'))
    
    print("✅ Genesis block created with 10,000,000 EDU distributed across 6 pools")
    
    # Block 1 - Student Transactions
    prev_hash = genesis_block['block_hash']
    block1_time = genesis_time + 600
    block1_hash = '00000000839a8e6886ab5951d76f411475428afc90947ee320161bbf18eb6048'
    
    cursor.execute("""
        INSERT INTO blocks (height, block_hash, prev_hash, merkle_root, timestamp, nonce, difficulty, block_data)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
    """, (1, block1_hash, prev_hash, 'merkle1', block1_time, 1234567, 1, b'Student registration block'))
    
    # Transactions in block 1
    cursor.execute("""
        INSERT INTO transactions (tx_hash, from_address, to_address, amount, fee, memo, 
                                 block_height, tx_index, timestamp, status)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    """, ('tx_block1_1', 'EDU_Circulating_Supply', 'alice_wallet_0x1a2b3c', 
          10000000000000, 100000000, 'Student registration bonus: Alice', 1, 0, block1_time, 'confirmed'))
    
    cursor.execute("""
        INSERT INTO transactions (tx_hash, from_address, to_address, amount, fee, memo, 
                                 block_height, tx_index, timestamp, status)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    """, ('tx_block1_2', 'EDU_Circulating_Supply', 'bob_wallet_0x4d5e6f', 
          10000000000000, 100000000, 'Student registration bonus: Bob', 1, 1, block1_time, 'confirmed'))
    
    cursor.execute("""
        INSERT INTO transactions (tx_hash, from_address, to_address, amount, fee, memo, 
                                 block_height, tx_index, timestamp, status)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    """, ('tx_block1_3', 'EDU_Circulating_Supply', 'carol_wallet_0x7g8h9i', 
          10000000000000, 100000000, 'Student registration bonus: Carol', 1, 2, block1_time, 'confirmed'))
    
    print("✅ Block 1 created: Student registration bonuses (3 transactions)")
    
    # Block 2 - NFT Marketplace Activity
    prev_hash = block1_hash
    block2_time = block1_time + 600
    block2_hash = '000000006a625f06636b8bb6ac7b960a8d03705d1ace08b1a19da3fdcc99ddbd'
    
    cursor.execute("""
        INSERT INTO blocks (height, block_hash, prev_hash, merkle_root, timestamp, nonce, difficulty, block_data)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
    """, (2, block2_hash, prev_hash, 'merkle2', block2_time, 2345678, 1, b'NFT marketplace transactions'))
    
    # NFT minting and transfers
    cursor.execute("""
        INSERT INTO transactions (tx_hash, from_address, to_address, amount, fee, memo, 
                                 block_height, tx_index, timestamp, status)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    """, ('tx_nft_mint_1', 'alice_wallet_0x1a2b3c', 'alice_wallet_0x1a2b3c', 
          0, 50000000, 'NFT Mint: Digital Art #001', 2, 0, block2_time, 'confirmed'))
    
    cursor.execute("""
        INSERT INTO transactions (tx_hash, from_address, to_address, amount, fee, memo, 
                                 block_height, tx_index, timestamp, status)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    """, ('tx_nft_sale_1', 'alice_wallet_0x1a2b3c', 'bob_wallet_0x4d5e6f', 
          500000000000, 50000000, 'NFT Sale: Digital Art #001 - 500 EDU', 2, 1, block2_time, 'confirmed'))
    
    print("✅ Block 2 created: NFT minting and marketplace (2 transactions)")
    
    # Block 3 - Loan Applications
    prev_hash = block2_hash
    block3_time = block2_time + 600
    block3_hash = '00000000cd9dd115c84f99c3d4be08fb7ed7166e80d4ba9e5ca4c91e0f18c4af'
    
    cursor.execute("""
        INSERT INTO blocks (height, block_hash, prev_hash, merkle_root, timestamp, nonce, difficulty, block_data)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
    """, (3, block3_hash, prev_hash, 'merkle3', block3_time, 3456789, 1, b'Student loan funding'))
    
    # Loan funding transactions
    cursor.execute("""
        INSERT INTO transactions (tx_hash, from_address, to_address, amount, fee, memo, 
                                 block_height, tx_index, timestamp, status)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    """, ('tx_loan_fund_1', 'EDU_Student_Loan_Pool', 'carol_wallet_0x7g8h9i', 
          5000000000000, 100000000, 'Student Loan Approved: 5000 EDU - CS Research', 3, 0, block3_time, 'confirmed'))
    
    cursor.execute("""
        INSERT INTO transactions (tx_hash, from_address, to_address, amount, fee, memo, 
                                 block_height, tx_index, timestamp, status)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    """, ('tx_loan_fund_2', 'EDU_Student_Loan_Pool', 'bob_wallet_0x4d5e6f', 
          3000000000000, 100000000, 'Student Loan Approved: 3000 EDU - Engineering Project', 3, 1, block3_time, 'confirmed'))
    
    print("✅ Block 3 created: Student loan funding (2 transactions)")
    
    # Block 4 - P2P Transfers and Fees
    prev_hash = block3_hash
    block4_time = block3_time + 600
    block4_hash = '000000004ebadb55ee9096c9a2f8880e09da59c0d68b1c228da88e48844a1485'
    
    cursor.execute("""
        INSERT INTO blocks (height, block_hash, prev_hash, merkle_root, timestamp, nonce, difficulty, block_data)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
    """, (4, block4_hash, prev_hash, 'merkle4', block4_time, 4567890, 1, b'P2P transfers'))
    
    # Peer to peer transactions
    cursor.execute("""
        INSERT INTO transactions (tx_hash, from_address, to_address, amount, fee, memo, 
                                 block_height, tx_index, timestamp, status)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    """, ('tx_p2p_1', 'alice_wallet_0x1a2b3c', 'carol_wallet_0x7g8h9i', 
          250000000000, 50000000, 'Payment for tutoring services', 4, 0, block4_time, 'confirmed'))
    
    cursor.execute("""
        INSERT INTO transactions (tx_hash, from_address, to_address, amount, fee, memo, 
                                 block_height, tx_index, timestamp, status)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    """, ('tx_p2p_2', 'bob_wallet_0x4d5e6f', 'alice_wallet_0x1a2b3c', 
          100000000000, 50000000, 'Book purchase', 4, 1, block4_time, 'confirmed'))
    
    cursor.execute("""
        INSERT INTO transactions (tx_hash, from_address, to_address, amount, fee, memo, 
                                 block_height, tx_index, timestamp, status)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    """, ('tx_p2p_3', 'carol_wallet_0x7g8h9i', 'bob_wallet_0x4d5e6f', 
          75000000000, 50000000, 'Lab equipment share', 4, 2, block4_time, 'confirmed'))
    
    print("✅ Block 4 created: P2P transfers (3 transactions)")
    
    # Update system settings
    cursor.execute("""
        UPDATE system_settings 
        SET value = ?, updated_at = CURRENT_TIMESTAMP 
        WHERE key = ?
    """, ('4', 'blockchain_height'))
    
    cursor.execute("""
        UPDATE system_settings 
        SET value = ?, updated_at = CURRENT_TIMESTAMP 
        WHERE key = ?
    """, (block4_hash, 'genesis_hash'))
    
    cursor.execute("""
        UPDATE system_settings 
        SET value = ?, updated_at = CURRENT_TIMESTAMP 
        WHERE key = ?
    """, ('1000000000000000', 'total_supply'))  # 10M EDU
    
    conn.commit()
    conn.close()
    
    print("\n" + "="*60)
    print("✅ Blockchain initialized successfully!")
    print("="*60)
    print(f"📊 Total Supply: 10,000,000 EDU")
    print(f"📦 Blocks: 5 (Genesis + 4 transaction blocks)")
    print(f"💳 Transactions: 17 total")
    print(f"   - 6 genesis allocations")
    print(f"   - 3 student registrations")
    print(f"   - 2 NFT operations")
    print(f"   - 2 loan fundings")
    print(f"   - 3 P2P transfers")
    print(f"   - 1 extra transaction")
    print("="*60)

if __name__ == '__main__':
    init_db()
