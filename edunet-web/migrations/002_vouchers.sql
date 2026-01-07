-- Add voucher system

CREATE TABLE IF NOT EXISTS vouchers (
    code TEXT PRIMARY KEY,
    amount INTEGER NOT NULL DEFAULT 2000000000, -- 20 EDU in satoshis
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    redeemed_at TEXT,
    redeemed_by TEXT,
    is_active INTEGER NOT NULL DEFAULT 1,
    FOREIGN KEY (redeemed_by) REFERENCES users(id)
);

CREATE TABLE IF NOT EXISTS user_vouchers (
    user_id TEXT NOT NULL,
    voucher_code TEXT NOT NULL,
    redeemed_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    amount INTEGER NOT NULL,
    tx_hash TEXT,
    PRIMARY KEY (user_id, voucher_code),
    FOREIGN KEY (user_id) REFERENCES users(id),
    FOREIGN KEY (voucher_code) REFERENCES vouchers(code)
);

-- Add voucher tracking to users table
ALTER TABLE users ADD COLUMN voucher_claimed INTEGER DEFAULT 0;
ALTER TABLE users ADD COLUMN voucher_tx_hash TEXT;

CREATE INDEX idx_vouchers_active ON vouchers(is_active);
CREATE INDEX idx_vouchers_redeemed ON vouchers(redeemed_by);
CREATE INDEX idx_user_vouchers_user ON user_vouchers(user_id);
