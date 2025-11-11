// User Authentication and Multi-Wallet Management
// File: edunet-gui/src/user_auth.rs

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use sha2::{Sha256, Digest};
use blockchain_core::wallet::{WalletManager, Wallet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub wallet_id: Uuid,
    pub wallet_address: String,
    pub university: Option<String>,
    pub student_id: Option<String>,
    pub reputation_score: f64,
    pub created_at: DateTime<Utc>,
    pub last_login: Option<DateTime<Utc>>,
    pub is_verified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    pub password: String,
    pub university: Option<String>,
    pub student_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSession {
    pub session_id: String,
    pub user_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

pub struct UserManager {
    users: Arc<RwLock<HashMap<Uuid, User>>>,
    sessions: Arc<RwLock<HashMap<String, UserSession>>>,
    username_to_id: Arc<RwLock<HashMap<String, Uuid>>>,
    email_to_id: Arc<RwLock<HashMap<String, Uuid>>>,
    wallet_manager: Arc<tokio::sync::RwLock<WalletManager>>,
}

impl UserManager {
    pub fn new(wallet_manager: Arc<tokio::sync::RwLock<WalletManager>>) -> Self {
        Self {
            users: Arc::new(RwLock::new(HashMap::new())),
            sessions: Arc::new(RwLock::new(HashMap::new())),
            username_to_id: Arc::new(RwLock::new(HashMap::new())),
            email_to_id: Arc::new(RwLock::new(HashMap::new())),
            wallet_manager,
        }
    }

    // Hash password with salt
    fn hash_password(password: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(password.as_bytes());
        hasher.update(b"edunet_salt_2025"); // Static salt for demo
        hex::encode(hasher.finalize())
    }

    // Verify password
    fn verify_password(password: &str, hash: &str) -> bool {
        Self::hash_password(password) == hash
    }

    // Generate session token
    fn generate_session_token() -> String {
        format!("sess_{}", Uuid::new_v4().simple())
    }

    // Register new user with automatic wallet creation
    pub async fn register_user(&self, request: RegisterRequest) -> Result<User, String> {
        let mut users = self.users.write().await;
        let mut username_map = self.username_to_id.write().await;
        let mut email_map = self.email_to_id.write().await;

        // Check if username or email already exists
        if username_map.contains_key(&request.username) {
            return Err("Username already exists".to_string());
        }
        if email_map.contains_key(&request.email) {
            return Err("Email already registered".to_string());
        }

        // Create wallet for new user
        let mut wallet_manager = self.wallet_manager.write().await;
        let wallet = wallet_manager.create_wallet(format!("{}'s Wallet", request.username))
            .map_err(|e| format!("Failed to create wallet: {}", e))?;

        let user_id = Uuid::new_v4();
        let password_hash = Self::hash_password(&request.password);

        let user = User {
            id: user_id,
            username: request.username.clone(),
            email: request.email.clone(),
            password_hash,
            wallet_id: wallet.id,
            wallet_address: wallet.address.clone(),
            university: request.university,
            student_id: request.student_id,
            reputation_score: 100.0, // Starting reputation
            created_at: Utc::now(),
            last_login: None,
            is_verified: false,
        };

        // Store user and mappings
        users.insert(user_id, user.clone());
        username_map.insert(request.username, user_id);
        email_map.insert(request.email, user_id);

        tracing::info!("✅ New user registered: {} with wallet {}", user.username, user.wallet_address);
        Ok(user)
    }

    // Login user and create session
    pub async fn login_user(&self, request: LoginRequest) -> Result<(User, String), String> {
        let username_map = self.username_to_id.read().await;
        let user_id = username_map.get(&request.username)
            .ok_or("Invalid username or password")?;

        let mut users = self.users.write().await;
        let user = users.get_mut(user_id)
            .ok_or("User not found")?;

        // Verify password
        if !Self::verify_password(&request.password, &user.password_hash) {
            return Err("Invalid username or password".to_string());
        }

        // Update last login
        user.last_login = Some(Utc::now());

        // Create session
        let session_token = Self::generate_session_token();
        let session = UserSession {
            session_id: session_token.clone(),
            user_id: user.id,
            created_at: Utc::now(),
            expires_at: Utc::now() + chrono::Duration::hours(24), // 24 hour session
        };

        let mut sessions = self.sessions.write().await;
        sessions.insert(session_token.clone(), session);

        tracing::info!("✅ User logged in: {} ({})", user.username, user.wallet_address);
        Ok((user.clone(), session_token))
    }

    // Get user by session token
    pub async fn get_user_by_session(&self, session_token: &str) -> Result<User, String> {
        let sessions = self.sessions.read().await;
        let session = sessions.get(session_token)
            .ok_or("Invalid session")?;

        // Check if session expired
        if Utc::now() > session.expires_at {
            return Err("Session expired".to_string());
        }

        let users = self.users.read().await;
        let user = users.get(&session.user_id)
            .ok_or("User not found")?;

        Ok(user.clone())
    }

    // Logout user (remove session)
    pub async fn logout_user(&self, session_token: &str) -> Result<(), String> {
        let mut sessions = self.sessions.write().await;
        sessions.remove(session_token)
            .ok_or("Session not found")?;
        
        tracing::info!("✅ User logged out");
        Ok(())
    }

    // Get user's wallet
    pub async fn get_user_wallet(&self, user: &User) -> Result<Wallet, String> {
        let wallet_manager = self.wallet_manager.read().await;
        let wallet = wallet_manager.get_wallet(&user.wallet_id)
            .ok_or("Wallet not found")?;
        Ok(wallet.clone())
    }

    // List all users (admin function)
    pub async fn list_users(&self) -> Vec<User> {
        let users = self.users.read().await;
        users.values().cloned().collect()
    }

    // Clean expired sessions
    pub async fn clean_expired_sessions(&self) {
        let mut sessions = self.sessions.write().await;
        let now = Utc::now();
        sessions.retain(|_, session| now <= session.expires_at);
    }

    // Update user reputation
    pub async fn update_reputation(&self, user_id: &Uuid, delta: f64) -> Result<(), String> {
        let mut users = self.users.write().await;
        let user = users.get_mut(user_id)
            .ok_or("User not found")?;
        
        user.reputation_score = (user.reputation_score + delta).max(0.0).min(100.0);
        Ok(())
    }

    // Get user statistics
    pub async fn get_user_stats(&self) -> serde_json::Value {
        let users = self.users.read().await;
        let sessions = self.sessions.read().await;
        
        let total_users = users.len();
        let verified_users = users.values().filter(|u| u.is_verified).count();
        let active_sessions = sessions.len();
        let avg_reputation = if total_users > 0 {
            users.values().map(|u| u.reputation_score).sum::<f64>() / total_users as f64
        } else {
            0.0
        };

        serde_json::json!({
            "total_users": total_users,
            "verified_users": verified_users,
            "active_sessions": active_sessions,
            "average_reputation": avg_reputation
        })
    }

    // Create demo users for testing
    pub async fn create_demo_users(&self) -> Result<(), String> {
        // Demo user 1: Alice
        let alice_request = RegisterRequest {
            username: "alice".to_string(),
            email: "alice@stanford.edu".to_string(),
            password: "password123".to_string(),
            university: Some("Stanford University".to_string()),
            student_id: Some("STU001".to_string()),
        };

        // Demo user 2: Bob  
        let bob_request = RegisterRequest {
            username: "bob".to_string(),
            email: "bob@mit.edu".to_string(),
            password: "password123".to_string(),
            university: Some("MIT".to_string()),
            student_id: Some("MIT002".to_string()),
        };

        // Demo user 3: Carol
        let carol_request = RegisterRequest {
            username: "carol".to_string(),
            email: "carol@berkeley.edu".to_string(),
            password: "password123".to_string(),
            university: Some("UC Berkeley".to_string()),
            student_id: Some("UCB003".to_string()),
        };

        // Register demo users (ignore errors if they already exist)
        let _ = self.register_user(alice_request).await;
        let _ = self.register_user(bob_request).await;
        let _ = self.register_user(carol_request).await;

        tracing::info!("✅ Demo users created (alice, bob, carol) - password: password123");
        Ok(())
    }
}