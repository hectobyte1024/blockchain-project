-- Initial database schema for Edunet platform
-- Students table
CREATE TABLE IF NOT EXISTS students (
    id TEXT PRIMARY KEY,
    email TEXT UNIQUE NOT NULL,
    name TEXT NOT NULL,
    university TEXT NOT NULL,
    wallet_address TEXT UNIQUE NOT NULL,
    reputation_score REAL DEFAULT 0.0,
    verified BOOLEAN DEFAULT FALSE,
    created_at DATETIME NOT NULL,
    updated_at DATETIME NOT NULL
);

-- Marketplace items table
CREATE TABLE IF NOT EXISTS market_items (
    id TEXT PRIMARY KEY,
    seller_id TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT NOT NULL,
    category TEXT NOT NULL,
    price REAL NOT NULL,
    currency TEXT DEFAULT 'EDU',
    item_type TEXT NOT NULL CHECK (item_type IN ('physical', 'digital', 'service')),
    status TEXT DEFAULT 'active' CHECK (status IN ('active', 'sold', 'draft', 'cancelled')),
    images TEXT, -- JSON array of image URLs
    created_at DATETIME NOT NULL,
    updated_at DATETIME NOT NULL,
    FOREIGN KEY (seller_id) REFERENCES students(id)
);

-- Loan applications table
CREATE TABLE IF NOT EXISTS loan_applications (
    id TEXT PRIMARY KEY,
    student_id TEXT NOT NULL,
    amount_requested REAL NOT NULL,
    purpose TEXT NOT NULL,
    proof_of_potential_score REAL NOT NULL,
    academic_data TEXT NOT NULL, -- JSON containing GPA, courses, etc.
    status TEXT DEFAULT 'pending' CHECK (status IN ('pending', 'approved', 'rejected', 'funded', 'repaid')),
    interest_rate REAL,
    term_months INTEGER,
    created_at DATETIME NOT NULL,
    updated_at DATETIME NOT NULL,
    FOREIGN KEY (student_id) REFERENCES students(id)
);

-- NFTs table
CREATE TABLE IF NOT EXISTS nfts (
    id TEXT PRIMARY KEY,
    creator_id TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT NOT NULL,
    content_type TEXT NOT NULL CHECK (content_type IN ('image', 'music', 'video', 'document', 'software')),
    content_url TEXT NOT NULL,
    metadata TEXT NOT NULL, -- JSON metadata
    token_id TEXT UNIQUE,
    minted BOOLEAN DEFAULT FALSE,
    price REAL,
    royalty_percentage REAL DEFAULT 10.0,
    created_at DATETIME NOT NULL,
    updated_at DATETIME NOT NULL,
    FOREIGN KEY (creator_id) REFERENCES students(id)
);

-- Investment projects table
CREATE TABLE IF NOT EXISTS investment_projects (
    id TEXT PRIMARY KEY,
    creator_id TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT NOT NULL,
    project_type TEXT NOT NULL CHECK (project_type IN ('thesis', 'hackathon', 'startup', 'research', 'patent')),
    funding_goal REAL NOT NULL,
    current_funding REAL DEFAULT 0.0,
    deadline DATETIME NOT NULL,
    status TEXT DEFAULT 'active' CHECK (status IN ('active', 'funded', 'completed', 'cancelled')),
    documents TEXT, -- JSON array of document URLs
    created_at DATETIME NOT NULL,
    updated_at DATETIME NOT NULL,
    FOREIGN KEY (creator_id) REFERENCES students(id)
);

-- Investments table (for tracking individual investments in projects)
CREATE TABLE IF NOT EXISTS investments (
    id TEXT PRIMARY KEY,
    investor_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    amount REAL NOT NULL,
    investment_date DATETIME NOT NULL,
    equity_percentage REAL,
    status TEXT DEFAULT 'active' CHECK (status IN ('active', 'withdrawn', 'matured')),
    created_at DATETIME NOT NULL,
    FOREIGN KEY (investor_id) REFERENCES students(id),
    FOREIGN KEY (project_id) REFERENCES investment_projects(id)
);

-- Transactions table (for tracking all platform transactions)
CREATE TABLE IF NOT EXISTS transactions (
    id TEXT PRIMARY KEY,
    from_address TEXT NOT NULL,
    to_address TEXT NOT NULL,
    amount REAL NOT NULL,
    currency TEXT DEFAULT 'EDU',
    transaction_type TEXT NOT NULL CHECK (transaction_type IN ('marketplace', 'loan', 'nft', 'investment', 'fee')),
    reference_id TEXT, -- ID of the related item (market_item, loan, nft, etc.)
    blockchain_tx_hash TEXT,
    status TEXT DEFAULT 'pending' CHECK (status IN ('pending', 'confirmed', 'failed')),
    created_at DATETIME NOT NULL
);

-- Create indexes for better performance
CREATE INDEX IF NOT EXISTS idx_students_email ON students(email);
CREATE INDEX IF NOT EXISTS idx_students_university ON students(university);
CREATE INDEX IF NOT EXISTS idx_market_items_seller ON market_items(seller_id);
CREATE INDEX IF NOT EXISTS idx_market_items_category ON market_items(category);
CREATE INDEX IF NOT EXISTS idx_market_items_status ON market_items(status);
CREATE INDEX IF NOT EXISTS idx_loans_student ON loan_applications(student_id);
CREATE INDEX IF NOT EXISTS idx_loans_status ON loan_applications(status);
CREATE INDEX IF NOT EXISTS idx_nfts_creator ON nfts(creator_id);
CREATE INDEX IF NOT EXISTS idx_nfts_minted ON nfts(minted);
CREATE INDEX IF NOT EXISTS idx_projects_creator ON investment_projects(creator_id);
CREATE INDEX IF NOT EXISTS idx_projects_status ON investment_projects(status);
CREATE INDEX IF NOT EXISTS idx_investments_investor ON investments(investor_id);
CREATE INDEX IF NOT EXISTS idx_investments_project ON investments(project_id);
CREATE INDEX IF NOT EXISTS idx_transactions_from ON transactions(from_address);
CREATE INDEX IF NOT EXISTS idx_transactions_to ON transactions(to_address);
CREATE INDEX IF NOT EXISTS idx_transactions_type ON transactions(transaction_type);