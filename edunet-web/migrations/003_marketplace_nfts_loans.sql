-- Migration: Add marketplace, NFT, and loan tables

-- Marketplace items table
CREATE TABLE IF NOT EXISTS marketplace_items (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    description TEXT NOT NULL,
    price REAL NOT NULL,
    category TEXT NOT NULL,
    seller_address TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'active',  -- active, sold, removed
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_marketplace_status ON marketplace_items(status);
CREATE INDEX idx_marketplace_seller ON marketplace_items(seller_address);
CREATE INDEX idx_marketplace_category ON marketplace_items(category);

-- NFTs table
CREATE TABLE IF NOT EXISTS nfts (
    token_id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL,
    image_url TEXT NOT NULL,
    owner_address TEXT NOT NULL,
    metadata TEXT,  -- JSON metadata
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_nft_owner ON nfts(owner_address);

-- Loans table
CREATE TABLE IF NOT EXISTS loans (
    id TEXT PRIMARY KEY,
    borrower_address TEXT NOT NULL,
    lender_address TEXT,
    amount REAL NOT NULL,
    purpose TEXT NOT NULL,
    duration_months INTEGER NOT NULL,
    interest_rate REAL NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',  -- pending, funded, repaid, defaulted
    funded_at TIMESTAMP,
    repaid_at TIMESTAMP,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_loan_borrower ON loans(borrower_address);
CREATE INDEX idx_loan_lender ON loans(lender_address);
CREATE INDEX idx_loan_status ON loans(status);
