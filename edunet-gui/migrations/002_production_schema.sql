-- EduNet Production Database Schema
-- SQLite database for persistent storage

-- Users table
CREATE TABLE IF NOT EXISTS users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    username TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    wallet_address TEXT UNIQUE NOT NULL,
    private_key TEXT NOT NULL,  -- Encrypted in production
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    last_login TIMESTAMP,
    email TEXT,
    is_active BOOLEAN DEFAULT 1
);

CREATE INDEX idx_users_username ON users(username);
CREATE INDEX idx_users_wallet ON users(wallet_address);

-- Blockchain state table
CREATE TABLE IF NOT EXISTS blocks (
    height INTEGER PRIMARY KEY,
    block_hash TEXT UNIQUE NOT NULL,
    prev_hash TEXT NOT NULL,
    merkle_root TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    nonce INTEGER NOT NULL,
    difficulty INTEGER NOT NULL,
    block_data BLOB NOT NULL,  -- Serialized block
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_blocks_hash ON blocks(block_hash);

-- Transactions table
CREATE TABLE IF NOT EXISTS transactions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    tx_hash TEXT UNIQUE NOT NULL,
    from_address TEXT NOT NULL,
    to_address TEXT NOT NULL,
    amount INTEGER NOT NULL,  -- In satoshis
    fee INTEGER NOT NULL,
    memo TEXT,
    block_height INTEGER,
    tx_index INTEGER,
    timestamp INTEGER NOT NULL,
    status TEXT DEFAULT 'pending',  -- pending, confirmed, failed
    tx_data BLOB,  -- Serialized transaction
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (block_height) REFERENCES blocks(height)
);

CREATE INDEX idx_transactions_hash ON transactions(tx_hash);
CREATE INDEX idx_transactions_from ON transactions(from_address);
CREATE INDEX idx_transactions_to ON transactions(to_address);
CREATE INDEX idx_transactions_status ON transactions(status);

-- UTXO set table
CREATE TABLE IF NOT EXISTS utxos (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    tx_hash TEXT NOT NULL,
    output_index INTEGER NOT NULL,
    address TEXT NOT NULL,
    amount INTEGER NOT NULL,
    script_pubkey TEXT NOT NULL,
    is_spent BOOLEAN DEFAULT 0,
    spent_in_tx TEXT,
    block_height INTEGER,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(tx_hash, output_index)
);

CREATE INDEX idx_utxos_address ON utxos(address, is_spent);
CREATE INDEX idx_utxos_spent ON utxos(is_spent);

-- NFT Registry table
CREATE TABLE IF NOT EXISTS nfts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    nft_id TEXT UNIQUE NOT NULL,  -- Transaction hash that created it
    name TEXT NOT NULL,
    description TEXT,
    image_url TEXT,
    creator_address TEXT NOT NULL,
    current_owner TEXT NOT NULL,
    metadata TEXT,  -- JSON metadata
    mint_tx_hash TEXT NOT NULL,
    mint_timestamp INTEGER NOT NULL,
    utxo_tx_hash TEXT,  -- Current UTXO containing this NFT
    utxo_index INTEGER,
    is_burned BOOLEAN DEFAULT 0,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (mint_tx_hash) REFERENCES transactions(tx_hash)
);

CREATE INDEX idx_nfts_id ON nfts(nft_id);
CREATE INDEX idx_nfts_owner ON nfts(current_owner);
CREATE INDEX idx_nfts_creator ON nfts(creator_address);

-- NFT Transfer History
CREATE TABLE IF NOT EXISTS nft_transfers (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    nft_id TEXT NOT NULL,
    from_address TEXT NOT NULL,
    to_address TEXT NOT NULL,
    tx_hash TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (nft_id) REFERENCES nfts(nft_id),
    FOREIGN KEY (tx_hash) REFERENCES transactions(tx_hash)
);

CREATE INDEX idx_nft_transfers_nft ON nft_transfers(nft_id);

-- Loan Applications table
CREATE TABLE IF NOT EXISTS loan_applications (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    loan_id TEXT UNIQUE NOT NULL,
    applicant_username TEXT NOT NULL,
    applicant_address TEXT NOT NULL,
    full_name TEXT NOT NULL,
    university TEXT NOT NULL,
    field_of_study TEXT NOT NULL,
    gpa REAL,
    test_score INTEGER,
    achievements TEXT,
    requested_amount INTEGER NOT NULL,  -- In EDU (satoshis)
    interest_rate REAL,
    repayment_term_months INTEGER,
    loan_purpose TEXT,
    graduation_year INTEGER,
    expected_career TEXT,
    expected_salary INTEGER,
    proof_of_potential_score REAL,
    status TEXT DEFAULT 'pending',  -- pending, approved, funded, rejected, repaid
    funded_amount INTEGER DEFAULT 0,
    funding_tx_hash TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    funded_at TIMESTAMP,
    FOREIGN KEY (applicant_username) REFERENCES users(username)
);

CREATE INDEX idx_loans_id ON loan_applications(loan_id);
CREATE INDEX idx_loans_applicant ON loan_applications(applicant_address);
CREATE INDEX idx_loans_status ON loan_applications(status);

-- Loan Funders table (many-to-many: multiple funders can fund one loan)
CREATE TABLE IF NOT EXISTS loan_funders (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    loan_id TEXT NOT NULL,
    funder_address TEXT NOT NULL,
    amount INTEGER NOT NULL,
    tx_hash TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (loan_id) REFERENCES loan_applications(loan_id),
    FOREIGN KEY (tx_hash) REFERENCES transactions(tx_hash)
);

CREATE INDEX idx_loan_funders_loan ON loan_funders(loan_id);
CREATE INDEX idx_loan_funders_funder ON loan_funders(funder_address);

-- Marketplace items table
CREATE TABLE IF NOT EXISTS marketplace_items (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    item_id TEXT UNIQUE NOT NULL,
    seller_address TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT,
    category TEXT NOT NULL,
    price INTEGER NOT NULL,  -- In satoshis
    image_url TEXT,
    status TEXT DEFAULT 'available',  -- available, sold, removed
    views INTEGER DEFAULT 0,
    sold_to TEXT,
    sold_tx_hash TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    sold_at TIMESTAMP
);

CREATE INDEX idx_marketplace_seller ON marketplace_items(seller_address);
CREATE INDEX idx_marketplace_status ON marketplace_items(status);
CREATE INDEX idx_marketplace_category ON marketplace_items(category);

-- System settings table
CREATE TABLE IF NOT EXISTS system_settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Insert initial settings
INSERT OR IGNORE INTO system_settings (key, value) VALUES 
    ('blockchain_height', '0'),
    ('genesis_hash', ''),
    ('total_supply', '2000000000000'),  -- 2M EDU in satoshis
    ('last_sync', '0');
