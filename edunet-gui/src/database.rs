use sqlx::{SqlitePool, sqlite::SqlitePoolOptions, Row};
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use std::sync::Arc;
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbUser {
    pub id: i64,
    pub username: String,
    pub password_hash: String,
    pub wallet_address: String,
    pub private_key: String,
    pub created_at: DateTime<Utc>,
    pub last_login: Option<DateTime<Utc>>,
    pub email: Option<String>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbTransaction {
    pub id: Option<i64>,
    pub tx_hash: String,
    pub from_address: String,
    pub to_address: String,
    pub amount: i64,
    pub fee: i64,
    pub memo: Option<String>,
    pub block_height: Option<i64>,
    pub tx_index: Option<i64>,
    pub timestamp: i64,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbNft {
    pub id: Option<i64>,
    pub nft_id: String,
    pub name: String,
    pub description: Option<String>,
    pub image_url: Option<String>,
    pub creator_address: String,
    pub current_owner: String,
    pub metadata: Option<String>,
    pub mint_tx_hash: String,
    pub mint_timestamp: i64,
    pub is_burned: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbLoanApplication {
    pub id: Option<i64>,
    pub loan_id: String,
    pub applicant_username: String,
    pub applicant_address: String,
    pub full_name: String,
    pub university: String,
    pub field_of_study: String,
    pub gpa: Option<f64>,
    pub test_score: Option<i32>,
    pub achievements: Option<String>,
    pub requested_amount: i64,
    pub interest_rate: Option<f64>,
    pub repayment_term_months: Option<i32>,
    pub loan_purpose: Option<String>,
    pub graduation_year: Option<i32>,
    pub expected_career: Option<String>,
    pub expected_salary: Option<i64>,
    pub proof_of_potential_score: Option<f64>,
    pub status: String,
    pub funded_amount: i64,
    pub funding_tx_hash: Option<String>,
}

pub struct Database {
    pool: SqlitePool,
}

impl Database {
    /// Initialize database connection and run migrations
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;

        // Run migrations
        sqlx::query(include_str!("../migrations/001_initial.sql"))
            .execute(&pool)
            .await?;
        
        sqlx::query(include_str!("../migrations/002_production_schema.sql"))
            .execute(&pool)
            .await?;

        Ok(Self { pool })
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    // ==================== USER OPERATIONS ====================

    pub async fn create_user(&self, username: &str, password_hash: &str, wallet_address: &str, private_key: &str) -> Result<i64> {
        let result = sqlx::query(
            "INSERT INTO users (username, password_hash, wallet_address, private_key) VALUES (?, ?, ?, ?)"
        )
        .bind(username)
        .bind(password_hash)
        .bind(wallet_address)
        .bind(private_key)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    pub async fn get_user_by_username(&self, username: &str) -> Result<Option<DbUser>> {
        let user = sqlx::query_as!(
            DbUser,
            "SELECT id, username, password_hash, wallet_address, private_key, 
             created_at, last_login, email, is_active 
             FROM users WHERE username = ?",
            username
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(user)
    }

    pub async fn get_user_by_wallet(&self, wallet_address: &str) -> Result<Option<DbUser>> {
        let user = sqlx::query_as!(
            DbUser,
            "SELECT id, username, password_hash, wallet_address, private_key, 
             created_at, last_login, email, is_active 
             FROM users WHERE wallet_address = ?",
            wallet_address
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(user)
    }

    pub async fn update_last_login(&self, username: &str) -> Result<()> {
        sqlx::query("UPDATE users SET last_login = CURRENT_TIMESTAMP WHERE username = ?")
            .bind(username)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn list_all_users(&self) -> Result<Vec<DbUser>> {
        let users = sqlx::query_as!(
            DbUser,
            "SELECT id, username, password_hash, wallet_address, private_key, 
             created_at, last_login, email, is_active 
             FROM users WHERE is_active = 1 
             ORDER BY created_at DESC"
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(users)
    }

    // ==================== TRANSACTION OPERATIONS ====================

    pub async fn save_transaction(&self, tx: &DbTransaction) -> Result<i64> {
        let result = sqlx::query(
            "INSERT INTO transactions (tx_hash, from_address, to_address, amount, fee, memo, 
             block_height, tx_index, timestamp, status) 
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&tx.tx_hash)
        .bind(&tx.from_address)
        .bind(&tx.to_address)
        .bind(tx.amount)
        .bind(tx.fee)
        .bind(&tx.memo)
        .bind(tx.block_height)
        .bind(tx.tx_index)
        .bind(tx.timestamp)
        .bind(&tx.status)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    pub async fn get_transaction_by_hash(&self, tx_hash: &str) -> Result<Option<DbTransaction>> {
        let tx = sqlx::query_as!(
            DbTransaction,
            "SELECT id, tx_hash, from_address, to_address, amount, fee, memo, 
             block_height, tx_index, timestamp, status 
             FROM transactions WHERE tx_hash = ?",
            tx_hash
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(tx)
    }

    pub async fn get_transactions_by_address(&self, address: &str, limit: i64) -> Result<Vec<DbTransaction>> {
        let txs = sqlx::query_as!(
            DbTransaction,
            "SELECT id, tx_hash, from_address, to_address, amount, fee, memo, 
             block_height, tx_index, timestamp, status 
             FROM transactions 
             WHERE from_address = ? OR to_address = ? 
             ORDER BY timestamp DESC LIMIT ?",
            address, address, limit
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(txs)
    }

    pub async fn update_transaction_status(&self, tx_hash: &str, status: &str, block_height: Option<i64>) -> Result<()> {
        sqlx::query("UPDATE transactions SET status = ?, block_height = ? WHERE tx_hash = ?")
            .bind(status)
            .bind(block_height)
            .bind(tx_hash)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ==================== NFT OPERATIONS ====================

    pub async fn mint_nft(&self, nft: &DbNft) -> Result<i64> {
        let result = sqlx::query(
            "INSERT INTO nfts (nft_id, name, description, image_url, creator_address, 
             current_owner, metadata, mint_tx_hash, mint_timestamp) 
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&nft.nft_id)
        .bind(&nft.name)
        .bind(&nft.description)
        .bind(&nft.image_url)
        .bind(&nft.creator_address)
        .bind(&nft.current_owner)
        .bind(&nft.metadata)
        .bind(&nft.mint_tx_hash)
        .bind(nft.mint_timestamp)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    pub async fn transfer_nft(&self, nft_id: &str, from: &str, to: &str, tx_hash: &str, timestamp: i64) -> Result<()> {
        // Update NFT owner
        sqlx::query("UPDATE nfts SET current_owner = ? WHERE nft_id = ?")
            .bind(to)
            .bind(nft_id)
            .execute(&self.pool)
            .await?;

        // Record transfer
        sqlx::query("INSERT INTO nft_transfers (nft_id, from_address, to_address, tx_hash, timestamp) VALUES (?, ?, ?, ?, ?)")
            .bind(nft_id)
            .bind(from)
            .bind(to)
            .bind(tx_hash)
            .bind(timestamp)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn get_nfts_by_owner(&self, owner: &str) -> Result<Vec<DbNft>> {
        let nfts = sqlx::query_as!(
            DbNft,
            "SELECT id, nft_id, name, description, image_url, creator_address, 
             current_owner, metadata, mint_tx_hash, mint_timestamp, is_burned 
             FROM nfts WHERE current_owner = ? AND is_burned = 0 
             ORDER BY mint_timestamp DESC",
            owner
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(nfts)
    }

    pub async fn get_nft_by_id(&self, nft_id: &str) -> Result<Option<DbNft>> {
        let nft = sqlx::query_as!(
            DbNft,
            "SELECT id, nft_id, name, description, image_url, creator_address, 
             current_owner, metadata, mint_tx_hash, mint_timestamp, is_burned 
             FROM nfts WHERE nft_id = ?",
            nft_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(nft)
    }

    pub async fn list_all_nfts(&self, limit: i64) -> Result<Vec<DbNft>> {
        let nfts = sqlx::query_as!(
            DbNft,
            "SELECT id, nft_id, name, description, image_url, creator_address, 
             current_owner, metadata, mint_tx_hash, mint_timestamp, is_burned 
             FROM nfts WHERE is_burned = 0 
             ORDER BY mint_timestamp DESC LIMIT ?",
            limit
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(nfts)
    }

    // ==================== LOAN OPERATIONS ====================

    pub async fn create_loan_application(&self, loan: &DbLoanApplication) -> Result<i64> {
        let result = sqlx::query(
            "INSERT INTO loan_applications (
                loan_id, applicant_username, applicant_address, full_name, university, 
                field_of_study, gpa, test_score, achievements, requested_amount, 
                interest_rate, repayment_term_months, loan_purpose, graduation_year, 
                expected_career, expected_salary, proof_of_potential_score, status
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&loan.loan_id)
        .bind(&loan.applicant_username)
        .bind(&loan.applicant_address)
        .bind(&loan.full_name)
        .bind(&loan.university)
        .bind(&loan.field_of_study)
        .bind(loan.gpa)
        .bind(loan.test_score)
        .bind(&loan.achievements)
        .bind(loan.requested_amount)
        .bind(loan.interest_rate)
        .bind(loan.repayment_term_months)
        .bind(&loan.loan_purpose)
        .bind(loan.graduation_year)
        .bind(&loan.expected_career)
        .bind(loan.expected_salary)
        .bind(loan.proof_of_potential_score)
        .bind(&loan.status)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    pub async fn get_loan_by_id(&self, loan_id: &str) -> Result<Option<DbLoanApplication>> {
        let loan = sqlx::query_as!(
            DbLoanApplication,
            "SELECT id, loan_id, applicant_username, applicant_address, full_name, 
             university, field_of_study, gpa, test_score, achievements, requested_amount, 
             interest_rate, repayment_term_months, loan_purpose, graduation_year, 
             expected_career, expected_salary, proof_of_potential_score, status, 
             funded_amount, funding_tx_hash 
             FROM loan_applications WHERE loan_id = ?",
            loan_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(loan)
    }

    pub async fn list_loans_by_status(&self, status: &str, limit: i64) -> Result<Vec<DbLoanApplication>> {
        let loans = sqlx::query_as!(
            DbLoanApplication,
            "SELECT id, loan_id, applicant_username, applicant_address, full_name, 
             university, field_of_study, gpa, test_score, achievements, requested_amount, 
             interest_rate, repayment_term_months, loan_purpose, graduation_year, 
             expected_career, expected_salary, proof_of_potential_score, status, 
             funded_amount, funding_tx_hash 
             FROM loan_applications WHERE status = ? 
             ORDER BY created_at DESC LIMIT ?",
            status, limit
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(loans)
    }

    pub async fn fund_loan(&self, loan_id: &str, funder_address: &str, amount: i64, tx_hash: &str, timestamp: i64) -> Result<()> {
        // Record funding
        sqlx::query("INSERT INTO loan_funders (loan_id, funder_address, amount, tx_hash, timestamp) VALUES (?, ?, ?, ?, ?)")
            .bind(loan_id)
            .bind(funder_address)
            .bind(amount)
            .bind(tx_hash)
            .bind(timestamp)
            .execute(&self.pool)
            .await?;

        // Update loan funded amount
        sqlx::query("UPDATE loan_applications SET funded_amount = funded_amount + ?, updated_at = CURRENT_TIMESTAMP WHERE loan_id = ?")
            .bind(amount)
            .bind(loan_id)
            .execute(&self.pool)
            .await?;

        // Check if fully funded and update status
        let loan = self.get_loan_by_id(loan_id).await?;
        if let Some(loan) = loan {
            if loan.funded_amount >= loan.requested_amount {
                sqlx::query("UPDATE loan_applications SET status = 'funded', funding_tx_hash = ?, funded_at = CURRENT_TIMESTAMP WHERE loan_id = ?")
                    .bind(tx_hash)
                    .bind(loan_id)
                    .execute(&self.pool)
                    .await?;
            }
        }

        Ok(())
    }

    // ==================== BLOCKCHAIN STATE ====================

    pub async fn save_block(&self, height: i64, block_hash: &str, prev_hash: &str, merkle_root: &str, 
                            timestamp: i64, nonce: i64, difficulty: i64, block_data: &[u8]) -> Result<()> {
        sqlx::query(
            "INSERT OR REPLACE INTO blocks (height, block_hash, prev_hash, merkle_root, timestamp, nonce, difficulty, block_data) 
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(height)
        .bind(block_hash)
        .bind(prev_hash)
        .bind(merkle_root)
        .bind(timestamp)
        .bind(nonce)
        .bind(difficulty)
        .bind(block_data)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_blockchain_height(&self) -> Result<i64> {
        let height: (i64,) = sqlx::query_as("SELECT COALESCE(MAX(height), 0) FROM blocks")
            .fetch_one(&self.pool)
            .await?;
        Ok(height.0)
    }

    pub async fn get_setting(&self, key: &str) -> Result<Option<String>> {
        let row = sqlx::query("SELECT value FROM system_settings WHERE key = ?")
            .bind(key)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.map(|r| r.get("value")))
    }

    pub async fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        sqlx::query("INSERT OR REPLACE INTO system_settings (key, value, updated_at) VALUES (?, ?, CURRENT_TIMESTAMP)")
            .bind(key)
            .bind(value)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
