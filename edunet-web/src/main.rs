//! Edunet GUI Backend Server
//! 
//! Web interface for the Edunet blockchain platform for university students.
//! Provides marketplace, lending, NFT minting, and investment pool functionality.

use axum::{
    extract::{Path, Query, State, WebSocketUpgrade, ws::WebSocket},
    response::{Html, Json, IntoResponse, Response},
    routing::{get, post},
    Router,
    http::{HeaderMap, StatusCode},
    body::Body,
};
use serde::{Deserialize, Serialize};
use std::{sync::Arc, collections::HashMap};
use tokio::{net::TcpListener, sync::Mutex};
use sha2::{Sha256, Digest};
use tower_http::{
    cors::CorsLayer,
    services::ServeDir,
};
use tracing::{info, error};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use rand;


mod blockchain_integration;
mod user_auth;
mod database;

use crate::blockchain_integration::{BlockchainBackend, TransactionHistory, TransactionStatus};
use crate::user_auth::{UserManager, User, LoginRequest, RegisterRequest};
use crate::database::Database;

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub backend: Arc<BlockchainBackend>,
    pub user_manager: Arc<UserManager>,
    pub marketplace: Arc<MarketplaceManager>,
    pub database: Arc<Database>,
}

/// Student user model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Student {
    pub id: Uuid,
    pub email: String,
    pub name: String,
    pub university: String,
    pub wallet_address: String,
    pub reputation_score: f64,
    pub verified: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Transaction request for blockchain operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionRequest {
    pub from_address: String,
    pub to_address: String,
    pub amount: f64,
    pub transaction_type: String, // "marketplace", "loan", "nft", "investment"
    pub metadata: Option<serde_json::Value>,
}

/// Wallet balance response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletBalance {
    pub address: String,
    pub balance: f64,
    pub pending_balance: f64,
    pub confirmed_transactions: u32,
    pub pending_transactions: u32,
}

/// Wallet creation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWalletRequest {
    pub name: String,
}

/// Send transaction request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendTransactionRequest {
    pub recipient: String,  // Recipient address
    pub amount: f64, // EDU amount
    pub message: Option<String>,
}

/// Payment request creation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentRequestData {
    pub wallet_id: String,
    pub amount: Option<f64>,
    pub message: Option<String>,
}

/// QR code parsing request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseQrRequest {
    pub qr_data: String,
}

/// Marketplace item model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketItem {
    pub id: Uuid,
    pub seller_id: Uuid,
    pub title: String,
    pub description: String,
    pub category: String,
    pub price: f64,
    pub currency: String,
    pub item_type: String, // "physical", "digital", "service"
    pub status: String,    // "active", "sold", "draft"
    pub images: Option<String>, // JSON array of image URLs
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateMarketItemRequest {
    pub title: String,
    pub description: String,
    pub category: String,
    pub price: f64,
    #[serde(default = "default_currency")]
    pub currency: String,
    #[serde(default = "default_item_type")]
    pub item_type: String,
    pub images: Option<String>,
}

fn default_currency() -> String { "EDU".to_string() }
fn default_item_type() -> String { "physical".to_string() }

/// Dashboard statistics
#[derive(Debug, Serialize)]
pub struct DashboardStats {
    pub total_students: i64,
    pub active_listings: i64,
    pub total_loans: i64,
    pub minted_nfts: i64,
    pub funded_projects: i64,
    pub total_volume: f64,
}

/// Marketplace manager for handling marketplace operations
pub struct MarketplaceManager {
    /// In-memory storage for marketplace items
    /// In production, this would be replaced with database storage
    items: Arc<Mutex<HashMap<Uuid, MarketItem>>>,
    /// Connection to blockchain backend for transactions
    backend: Arc<BlockchainBackend>,
}

impl MarketplaceManager {
    /// Create new marketplace manager
    pub fn new(backend: Arc<BlockchainBackend>) -> Self {
        Self {
            items: Arc::new(Mutex::new(HashMap::new())),
            backend,
        }
    }

    /// Get all active marketplace items
    pub async fn get_all_items(&self) -> Result<Vec<MarketItem>, String> {
        let items = self.items.lock().await;
        let active_items: Vec<MarketItem> = items
            .values()
            .filter(|item| item.status == "active")
            .cloned()
            .collect();
        Ok(active_items)
    }

    /// Get specific marketplace item by ID
    pub async fn get_item(&self, id: Uuid) -> Result<Option<MarketItem>, String> {
        let items = self.items.lock().await;
        Ok(items.get(&id).cloned())
    }

    /// Create new marketplace item
    pub async fn create_item(&self, item: MarketItem) -> Result<(), String> {
        let mut items = self.items.lock().await;
        
        // Validate item data
        if item.title.trim().is_empty() {
            return Err("Title cannot be empty".to_string());
        }
        
        if item.price <= 0.0 {
            return Err("Price must be positive".to_string());
        }

        // Store item
        items.insert(item.id, item.clone());
        
        // TODO: In production, this would:
        // 1. Store item metadata on blockchain or IPFS
        // 2. Create smart contract for escrow
        // 3. Emit marketplace events
        
        info!("Created marketplace item: {} - {}", item.id, item.title);
        Ok(())
    }

    /// Update marketplace item
    pub async fn update_item(&self, id: Uuid, updated_item: MarketItem) -> Result<(), String> {
        let mut items = self.items.lock().await;
        
        if items.contains_key(&id) {
            items.insert(id, updated_item);
            Ok(())
        } else {
            Err("Item not found".to_string())
        }
    }

    /// Delete marketplace item
    pub async fn delete_item(&self, id: Uuid) -> Result<(), String> {
        let mut items = self.items.lock().await;
        
        if items.remove(&id).is_some() {
            Ok(())
        } else {
            Err("Item not found".to_string())
        }
    }

    /// Get items by seller
    pub async fn get_items_by_seller(&self, seller_id: Uuid) -> Result<Vec<MarketItem>, String> {
        let items = self.items.lock().await;
        let seller_items: Vec<MarketItem> = items
            .values()
            .filter(|item| item.seller_id == seller_id && item.status == "active")
            .cloned()
            .collect();
        Ok(seller_items)
    }

    /// Search items by category or title
    pub async fn search_items(&self, query: &str, category: Option<&str>) -> Result<Vec<MarketItem>, String> {
        let items = self.items.lock().await;
        let query_lower = query.to_lowercase();
        
        let filtered_items: Vec<MarketItem> = items
            .values()
            .filter(|item| {
                if item.status != "active" {
                    return false;
                }
                
                let title_match = item.title.to_lowercase().contains(&query_lower);
                let description_match = item.description.to_lowercase().contains(&query_lower);
                let category_match = category.map_or(true, |cat| item.category == cat);
                
                (title_match || description_match) && category_match
            })
            .cloned()
            .collect();
            
        Ok(filtered_items)
    }

    /// Get marketplace statistics
    pub async fn get_stats(&self) -> Result<serde_json::Value, String> {
        let items = self.items.lock().await;
        
        let active_count = items.values().filter(|item| item.status == "active").count();
        let total_value: f64 = items
            .values()
            .filter(|item| item.status == "active")
            .map(|item| item.price)
            .sum();
        
        let stats = serde_json::json!({
            "active_listings": active_count,
            "total_value": total_value,
            "total_items": items.len()
        });
        
        Ok(stats)
    }
}

/// API response wrapper
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub message: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            message: None,
        }
    }

    pub fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            message: Some(message),
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();
    let mut is_bootstrap = false;
    let mut bootstrap_server = None;

    for (i, arg) in args.iter().enumerate() {
        match arg.as_str() {
            "--bootstrap" => is_bootstrap = true,
            "--connect" => {
                if let Some(address) = args.get(i + 1) {
                    bootstrap_server = Some(address.clone());
                }
            },
            _ => {}
        }
    }

    info!("🌐 Starting EduNet GUI with blockchain backend...");
    
    // Initialize database
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite:./edunet-gui/edunet.db".to_string());
    info!("📊 Connecting to database: {}", database_url);
    let database = Arc::new(Database::new(&database_url).await?);
    info!("✅ Database initialized");
    
    // Initialize blockchain backend
    let backend = BlockchainBackend::new(is_bootstrap, bootstrap_server, database.clone()).await?;
    
    // Initialize user management system with database
    let user_manager = Arc::new(UserManager::new(backend.wallets.clone(), database.clone()));
    
    // Load existing users from database
    user_manager.load_users_from_db().await.map_err(|e| anyhow::anyhow!(e))?;
    
    // Create demo users for testing (if they don't exist)
    user_manager.create_demo_users().await.map_err(|e| anyhow::anyhow!(e))?;
    
    let backend = Arc::new(backend);
    let marketplace = Arc::new(MarketplaceManager::new(backend.clone()));
    
    let state = AppState {
        backend,
        user_manager,
        marketplace,
        database,
    };

    if is_bootstrap {
        info!("🌟 Running as BOOTSTRAP NODE");
    } else {
        info!("👥 Running as CLIENT NODE");
    }

    // Build the application router
    let app = Router::new()
        // Static files
        .nest_service("/static", ServeDir::new("edunet-web/static"))
        
        // Authentication pages
        .route("/", get(login_page))
        .route("/login", get(login_page))
        .route("/register", get(register_page))
        .route("/dashboard", get(dashboard_handler))
        .route("/users", get(users_overview_page))
        .route("/test", get(test_html_handler))
        
        // Main pages (require authentication)
        .route("/marketplace", get(marketplace_page))
        .route("/loans", get(loans_page))
        .route("/nfts", get(nfts_page))
        .route("/invest", get(invest_page))
        .route("/wallet", get(wallet_page))
        .route("/explorer", get(blockchain_explorer_page))
        .route("/architecture", get(architecture_page))
        
        // WebSocket for real-time updates
        .route("/ws", get(websocket_handler))
        
        // Authentication API
        .route("/api/auth/login", post(api_login))
        .route("/api/auth/register", post(api_register))
        .route("/api/auth/logout", post(api_logout))
        .route("/api/auth/me", get(api_current_user))
        
        // API routes
        .route("/api/students", get(get_students).post(create_student))
        .route("/api/students/:id", get(get_student))
        .route("/api/marketplace", get(get_market_items).post(create_market_item))
        .route("/api/marketplace/:id", get(get_market_item))
        .route("/api/dashboard/stats", get(dashboard_stats_handler))
        .route("/api/user/info", get(get_current_user_info))
        .route("/api/users", get(get_all_users))
        .route("/api/users/stats", get(get_user_statistics))
        
        // Add blockchain API routes with state extraction
        .route("/api/v1/wallets", post(blockchain_create_wallet))
        .route("/api/v1/wallets/:address/balance", get(blockchain_get_balance))
        .route("/api/v1/network/status", get(blockchain_network_status))
        .route("/api/v1/mining/stats", get(blockchain_mining_stats))
        
        // Dashboard Quick Actions API routes
        .route("/api/blockchain/send-transaction", post(api_send_transaction))
        .route("/api/blockchain/transactions", get(api_get_transactions))
        .route("/api/blockchain/transactions/pending", get(api_get_pending_transactions))
        .route("/api/blockchain/transactions/recent", get(api_get_recent_transactions))
        .route("/api/blockchain/mine", post(api_mine_block))
        .route("/api/blockchain/network-status", get(api_network_status))
        .route("/api/blockchain/sync-status", get(api_sync_status))
        .route("/api/blockchain/sync", post(api_sync_blockchain))
        
        // NFT API routes
        .route("/api/nft/mint", post(api_nft_mint))
        .route("/api/nft/list", get(api_nft_list))
        .route("/api/nft/owned/:address", get(api_nft_owned))
        .route("/api/nft/:id", get(api_nft_get))
        .route("/api/nft/transfer", post(api_nft_transfer))
        
        // Loan API routes
        .route("/api/loan/apply", post(api_loan_apply))
        .route("/api/loan/list", get(api_loan_list))
        .route("/api/loan/:id", get(api_loan_get))
        .route("/api/loan/fund", post(api_loan_fund))
        
        .layer(CorsLayer::permissive())
        .with_state(state);

    // Start the server
    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], 8080));
    info!("🚀 EduNet server running on http://{}", addr);
    
    let listener = TcpListener::bind(addr).await?;
    
    axum::serve(listener, app).await?;
    
    Ok(())
}

/// Generate SHA256 hash of input string
fn sha256_hash(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let result = hasher.finalize();
    format!("{:x}", result)
}

// Helper function to get user from session
async fn get_current_user(headers: &HeaderMap, state: &AppState) -> Result<user_auth::User, String> {
    tracing::debug!("🔍 Checking authentication headers...");
    let session_token = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(|auth| {
            tracing::debug!("📋 Found authorization header: {}", auth);
            auth.strip_prefix("Bearer ")
        })
        .or_else(|| {
            headers
                .get("cookie")
                .and_then(|value| value.to_str().ok())
                .and_then(|cookies| {
                    tracing::debug!("🍪 Found cookie header: {}", cookies);
                    cookies.split(';')
                        .find_map(|cookie| {
                            let cookie = cookie.trim();
                            cookie.strip_prefix("session=")
                        })
                })
        })
        .ok_or("No session token found")?;

    tracing::debug!("🎫 Using session token: {}", session_token);

    state.user_manager.get_user_by_session(session_token).await
}

// Page handlers
async fn login_page() -> impl IntoResponse {
    Html(include_str!("../templates/login.html"))
}

async fn register_page() -> impl IntoResponse {
    Html(include_str!("../templates/register.html"))
}

async fn users_overview_page() -> impl IntoResponse {
    Html(include_str!("../templates/users_overview.html"))
}

async fn test_html_handler() -> Html<&'static str> {
    Html("<!DOCTYPE html><html><head><title>Test</title></head><body><h1 style='color: red; font-size: 48px;'>BIG RED TEST</h1><p>If you see this as styled text (big red heading), HTML works. If you see raw HTML tags, it's broken.</p></body></html>")
}

async fn dashboard_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> impl IntoResponse {
    // Try to get authenticated user, but don't fail if not authenticated
    // The JavaScript will handle authentication and redirect if needed
    let user = match get_current_user(&headers, &state).await {
        Ok(user) => {
            tracing::info!("✅ User authenticated for dashboard: {}", user.username);
            Some(user)
        },
        Err(e) => {
            tracing::debug!("ℹ️  Dashboard accessed without authentication: {}", e);
            None
        },
    };

    // For demonstration, we'll use default values when no user is authenticated
    let (username, wallet_address, balance, reputation, university, username_initial) = 
        if let Some(user) = &user {
            let user_wallet = state.user_manager.get_user_wallet(user).await
                .unwrap_or_else(|_| blockchain_core::wallet::Wallet {
                    id: user.wallet_id,
                    name: format!("{}'s Wallet", user.username),
                    private_key: [0u8; 32],
                    public_key: [0u8; 33],
                    address: user.wallet_address.clone(),
                    balance: 100, // Demo balance (u64)
                    created_at: chrono::Utc::now(),
                });
            
            (
                user.username.clone(),
                user.wallet_address.clone(),
                user_wallet.balance,
                user.reputation_score as f64,
                user.university.clone().unwrap_or_else(|| "Not specified".to_string()),
                user.username.chars().next().unwrap_or('U').to_uppercase().to_string()
            )
        } else {
            // Default values for unauthenticated access - JavaScript will handle redirect
            (
                "Guest".to_string(),
                "Loading...".to_string(),
                0u64,
                0.0,
                "Please log in".to_string(),
                "G".to_string()
            )
        };

    // Get real-time blockchain data  
    let _network_status = state.backend.get_network_status().await
        .unwrap_or_else(|_| serde_json::json!({}));

    // Create a functional dashboard that we know works, but with more features
    let dashboard_html = format!(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>EduNet Dashboard - {username}</title>
    <style>
        * {{ margin: 0; padding: 0; box-sizing: border-box; }}
        body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; background: #f5f5f5; }}
        
        .navbar {{
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: white;
            padding: 15px 0;
            box-shadow: 0 2px 10px rgba(0,0,0,0.1);
        }}
        
        .nav-content {{
            max-width: 1200px;
            margin: 0 auto;
            padding: 0 20px;
            display: flex;
            justify-content: space-between;
            align-items: center;
        }}
        
        .nav-left {{ display: flex; align-items: center; }}
        .logo {{ font-size: 24px; font-weight: bold; margin-right: 30px; }}
        
        .nav-links {{ display: flex; gap: 20px; }}
        .nav-links a {{
            color: white;
            text-decoration: none;
            padding: 8px 16px;
            border-radius: 6px;
            transition: background 0.3s;
        }}
        .nav-links a:hover {{ background: rgba(255,255,255,0.1); }}
        
        .user-info {{
            display: flex;
            align-items: center;
            gap: 10px;
        }}
        
        .user-avatar {{
            width: 36px;
            height: 36px;
            background: rgba(255,255,255,0.2);
            border-radius: 50%;
            display: flex;
            align-items: center;
            justify-content: center;
            font-weight: bold;
        }}
        
        .container {{
            max-width: 1200px;
            margin: 0 auto;
            padding: 20px;
        }}
        
        .dashboard-grid {{
            display: grid;
            grid-template-columns: 1fr 1fr 1fr;
            gap: 20px;
            margin-bottom: 30px;
        }}
        
        .card {{
            background: white;
            padding: 25px;
            border-radius: 12px;
            box-shadow: 0 4px 6px rgba(0, 0, 0, 0.05);
            border: 1px solid #e5e7eb;
        }}
        
        .card h3 {{
            color: #1f2937;
            margin-bottom: 15px;
            font-size: 18px;
        }}
        
        .balance-amount {{
            font-size: 32px;
            font-weight: bold;
            color: #059669;
            margin: 10px 0;
        }}
        
        .wallet-address {{
            font-family: 'Courier New', monospace;
            background: #f3f4f6;
            padding: 8px 12px;
            border-radius: 6px;
            font-size: 14px;
            word-break: break-all;
        }}
        
        .reputation-score {{
            font-size: 24px;
            font-weight: bold;
            color: #dc2626;
            margin: 10px 0;
        }}
        
        .btn {{
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: white;
            border: none;
            padding: 12px 24px;
            border-radius: 8px;
            cursor: pointer;
            font-size: 16px;
            margin: 10px 5px 0 0;
            transition: transform 0.2s;
        }}
        
        .btn:hover {{ transform: translateY(-2px); }}
    </style>
</head>
<body>
    <nav class="navbar">
        <div class="nav-content">
            <div class="nav-left">
                <div class="logo">🎓 EduNet</div>
                <div class="nav-links">
                    <a href="/explorer">⛓️ Explorer</a>
                    <a href="/marketplace">Marketplace</a>
                    <a href="/loans">Loans</a>
                    <a href="/nfts">NFTs</a>
                    <a href="/invest">Invest</a>
                </div>
            </div>
            <div class="user-info">
                <span>Welcome, {username}!</span>
                <div class="user-avatar">{username_initial}</div>
            </div>
        </div>
    </nav>
    
    <div class="container">
        <div class="dashboard-grid">
            <div class="card">
                <h3>💰 Wallet Balance</h3>
                <div class="balance-amount" id="wallet-balance">{balance} EDU</div>
                <button class="btn" onclick="refreshBalance()">Refresh Balance</button>
            </div>
            
            <div class="card">
                <h3>🏛️ Profile Information</h3>
                <p><strong>University:</strong> {university}</p>
                <p><strong>Reputation Score:</strong> <span class="reputation-score">{reputation}</span></p>
                <button class="btn" onclick="viewProfile()">Edit Profile</button>
            </div>
            
            <div class="card">
                <h3>🔗 Wallet Address</h3>
                <div class="wallet-address" id="wallet-address">{wallet_address}</div>
                <button class="btn" onclick="copyAddress()">Copy Address</button>
            </div>
        </div>
        
        <div class="card">
            <h3>🚀 Quick Actions <span style="background: #dc2626; color: white; padding: 2px 6px; border-radius: 4px; font-size: 10px; font-weight: bold;">REAL ECDSA</span></h3>
            <button class="btn" onclick="sendTransaction()">Send REAL Transaction</button>
            <button class="btn" onclick="viewTransactions()">View History</button>
            <button class="btn" onclick="mineBlock()">Mine REAL Block</button>
            <button class="btn" onclick="connectPeers()">Network Status</button>
            <button class="btn" onclick="checkSyncStatus()">Sync Status</button>
            <button class="btn" onclick="triggerSync()" id="syncBtn">Sync Blockchain</button>
            <button class="btn" onclick="window.location.href='/explorer'">⛓️ Blockchain Explorer</button>
            <button class="btn" onclick="window.location.href='/architecture'">🏗️ System Architecture</button>
        </div>
    </div>
    
    <script>
        // Initialize dashboard
        console.log('EduNet Dashboard loaded for user: {username}');
        
        // Get auth token from session storage or cookies
        function getAuthToken() {{
            return localStorage.getItem('session_token') || sessionStorage.getItem('session_token');
        }}
        
        // Helper function for authenticated API calls
        async function authFetch(url, options = {{}}) {{
            const token = getAuthToken();
            return fetch(url, {{
                ...options,
                headers: {{
                    'Authorization': `Bearer ${{token}}`,
                    'Content-Type': 'application/json',
                    ...options.headers
                }}
            }});
        }}
        
        async function refreshBalance() {{
            console.log('Refreshing balance...');
            try {{
                const response = await authFetch('/api/auth/me');
                const data = await response.json();
                if (data.success) {{
                    document.getElementById('wallet-balance').textContent = `${{data.data.wallet.balance}} EDU`;
                    console.log('✅ Balance updated:', data.data.wallet.balance);
                }} else {{
                    console.error('❌ Failed to fetch balance:', data.message);
                }}
            }} catch (error) {{
                console.error('❌ Balance refresh error:', error);
                alert('Failed to refresh balance. Please try again.');
            }}
        }}
        
        function viewProfile() {{
            console.log('Opening profile editor...');
            const profileData = `
Username: {username}
University: {university}  
Reputation: {reputation}
Wallet: {wallet_address}
            `.trim();
            
            alert(`Profile Information:\\n\\n${{profileData}}\\n\\nProfile editing will be implemented in the next version!`);
        }}
        
        function copyAddress() {{
            const address = document.getElementById('wallet-address').textContent;
            navigator.clipboard.writeText(address).then(() => {{
                alert('✅ Wallet address copied to clipboard!');
                console.log('📋 Copied address:', address);
            }}).catch(() => {{
                // Fallback for older browsers
                const textArea = document.createElement('textarea');
                textArea.value = address;
                document.body.appendChild(textArea);
                textArea.select();
                document.execCommand('copy');
                document.body.removeChild(textArea);
                alert('✅ Wallet address copied to clipboard!');
            }});
        }}
        
        async function sendTransaction() {{
            console.log('Opening send transaction dialog...');
            const recipient = prompt('Enter recipient wallet address:');
            if (!recipient) return;
            
            const amount = prompt('Enter amount to send (EDU):');
            if (!amount || isNaN(amount) || parseFloat(amount) <= 0) {{
                alert('❌ Please enter a valid amount');
                return;
            }}
            
            try {{
                const response = await authFetch('/api/blockchain/send-transaction', {{
                    method: 'POST',
                    body: JSON.stringify({{
                        recipient: recipient,
                        amount: parseFloat(amount)
                    }})
                }});
                
                const data = await response.json();
                if (data.success) {{
                    alert(`✅ Transaction sent successfully!\\nTx Hash: ${{data.transaction_hash || 'Generated'}}`);
                    refreshBalance(); // Update balance after transaction
                }} else {{
                    alert(`❌ Transaction failed: ${{data.message}}`);
                }}
            }} catch (error) {{
                console.error('❌ Transaction error:', error);
                alert('❌ Failed to send transaction. Please try again.');
            }}
        }}
        
        async function viewTransactions() {{
            console.log('Loading transaction history...');
            try {{
                const response = await authFetch('/api/blockchain/transactions');
                const data = await response.json();
                
                if (data.success && data.transactions && data.transactions.length > 0) {{
                    showTransactionModal(data.transactions);
                }} else if (data.success) {{
                    alert('📜 No transactions found yet\\n\\nYour transaction history will appear here once you:\\n• Send EDU tokens to someone\\n• Receive EDU tokens\\n• Mine blocks on the network');
                }} else {{
                    alert('❌ Failed to load transaction history: ' + (data.message || 'Unknown error'));
                }}
            }} catch (error) {{
                console.error('❌ Transaction history error:', error);
                alert('📜 Unable to connect to blockchain\\n\\nPlease check your connection and try again.');
            }}
        }}
        
        function showTransactionModal(transactions) {{
            let modalHTML = `
                <div style="position: fixed; top: 0; left: 0; width: 100%; height: 100%; background: rgba(0,0,0,0.8); z-index: 1000; display: flex; align-items: center; justify-content: center;" onclick="this.remove()">
                    <div style="background: white; padding: 20px; border-radius: 10px; max-width: 600px; max-height: 80vh; overflow-y: auto;" onclick="event.stopPropagation()">
                        <h3 style="margin-top: 0;">📜 Transaction History</h3>
                        <div style="margin-bottom: 20px;">
                            <strong>${{transactions.length}}</strong> transaction(s) found
                        </div>
                        <div style="max-height: 400px; overflow-y: auto;">
                `;
            
            transactions.forEach((tx, index) => {{
                const date = new Date(tx.timestamp).toLocaleString();
                const amount = tx.amount_edu || (tx.amount / 100000000); // Convert to EDU if needed
                const type = tx.transaction_type || 'Unknown';
                const status = tx.status === 'Confirmed' ? '✅' : tx.status === 'Pending' ? '🔄' : '❓';
                
                modalHTML += `
                    <div style="border: 1px solid #ddd; margin: 10px 0; padding: 15px; border-radius: 5px; background: #f9f9f9;">
                        <div style="font-weight: bold; margin-bottom: 5px;">
                            ${{status}} ${{type.charAt(0).toUpperCase() + type.slice(1)}} - ${{amount}} EDU
                        </div>
                        <div style="font-size: 0.9em; color: #666;">
                            <div>Hash: ${{tx.hash.slice(0, 16)}}...</div>
                            <div>From: ${{tx.from_address.slice(0, 20)}}...</div>
                            <div>To: ${{tx.to_address.slice(0, 20)}}...</div>
                            <div>Date: ${{date}}</div>
                            ${{tx.block_height ? `<div>Block: #${{tx.block_height}}</div>` : ''}}
                            ${{tx.confirmations ? `<div>Confirmations: ${{tx.confirmations}}</div>` : ''}}
                        </div>
                    </div>
                `;
            }});
            
            modalHTML += `
                        </div>
                        <div style="text-align: center; margin-top: 20px;">
                            <button onclick="this.closest('div[style*=\"position: fixed\"]').remove()" style="padding: 10px 20px; background: #007bff; color: white; border: none; border-radius: 5px; cursor: pointer;">
                                Close
                            </button>
                        </div>
                    </div>
                </div>
            `;
            
            document.body.insertAdjacentHTML('beforeend', modalHTML);
        }}
        
        async function mineBlock() {{
            console.log('Starting mining process...');
            const confirmMine = confirm('⛏️  Start mining a new block?\\n\\nThis will:\\n• Process pending transactions\\n• Earn mining rewards\\n• Secure the network\\n\\nContinue?');
            
            if (!confirmMine) return;
            
            try {{
                const response = await authFetch('/api/blockchain/mine', {{
                    method: 'POST'
                }});
                
                const data = await response.json();
                if (data.success) {{
                    alert(`⛏️ ✅ Block mined successfully!\\n\\nBlock Hash: ${{data.block_hash || 'Generated'}}\\nReward: ${{data.reward || 50}} EDU\\nTransactions: ${{data.transactions_count || 0}}`);
                    refreshBalance(); // Update balance after mining reward
                }} else {{
                    alert(`❌ Mining failed: ${{data.message}}`);
                }}
            }} catch (error) {{
                console.error('❌ Mining error:', error);
                alert('❌ Mining failed. Please ensure you are connected to the network and try again.');
            }}
        }}
        
        async function connectPeers() {{
            console.log('Checking network status...');
            try {{
                const response = await authFetch('/api/blockchain/network-status');
                const data = await response.json();
                
                if (data.success) {{
                    const networkInfo = `
🌐 Network Status

Connected Peers: ${{data.peer_count || 0}}
Block Height: ${{data.block_height || 0}}
Network Hash Rate: ${{data.hash_rate || 'Calculating...'}}
Pending Transactions: ${{data.pending_tx || 0}}
Node Status: ${{data.node_status || 'Active'}}
                    `.trim();
                    alert(networkInfo);
                }} else {{
                    throw new Error('Network data unavailable');
                }}
            }} catch (error) {{
                console.error('❌ Network status error:', error);
                alert('❌ Unable to fetch network status. Please check your connection and try again.');
            }}
        }}
        
        // Check blockchain synchronization status
        async function checkSyncStatus() {{
            console.log('Checking sync status...');
            try {{
                const response = await authFetch('/api/blockchain/sync-status');
                const data = await response.json();
                
                const syncInfo = `
🔄 Blockchain Sync Status

Syncing: ${{data.is_syncing ? 'Yes' : 'No'}}
Local Height: ${{data.local_height || 0}}
Network Height: ${{data.network_height || 0}}
Progress: ${{data.progress_percent || 0}}%
Speed: ${{data.blocks_per_second || 0}} blocks/sec
ETA: ${{data.eta_seconds || 0}} seconds
Connected Peers: ${{data.peers_connected || 0}}
                `.trim();
                alert(syncInfo);
            }} catch (error) {{
                console.error('❌ Sync status error:', error);
                alert('❌ Unable to fetch sync status. Please try again.');
            }}
        }}
        
        // Trigger blockchain synchronization
        async function triggerSync() {{
            const syncBtn = document.getElementById('syncBtn');
            const originalText = syncBtn.textContent;
            
            syncBtn.textContent = 'Syncing...';
            syncBtn.disabled = true;
            
            try {{
                const response = await authFetch('/api/blockchain/sync', {{
                    method: 'POST',
                    headers: {{ 'Content-Type': 'application/json' }},
                    body: JSON.stringify({{}})
                }});
                const data = await response.json();
                
                if (data.success) {{
                    alert('🔄 Blockchain sync started successfully!\\n\\nThe node will now download missing blocks from the network.');
                }} else {{
                    throw new Error(data.message || 'Sync failed');
                }}
            }} catch (error) {{
                console.error('❌ Sync trigger error:', error);
                alert('❌ Failed to start sync: ' + error.message);
            }} finally {{
                syncBtn.textContent = originalText;
                syncBtn.disabled = false;
            }}
        }}
        
        // Auto-refresh balance every 30 seconds
        setInterval(refreshBalance, 30000);
        
        // Initialize on page load
        document.addEventListener('DOMContentLoaded', function() {{
            console.log('🚀 EduNet Dashboard initialized for {username}');
        }});
    </script>
</body>
</html>"#, 
        username = username,
        username_initial = username_initial,
        balance = balance,
        university = university,
        reputation = reputation,
        wallet_address = wallet_address
    );
    
    Html(dashboard_html)
}

async fn marketplace_page() -> impl IntoResponse {
    (
        [
            ("Cache-Control", "no-cache, no-store, must-revalidate"),
            ("Pragma", "no-cache"),
            ("Expires", "0"),
        ],
        Html(include_str!("../templates/marketplace.html"))
    )
}

async fn loans_page() -> impl IntoResponse {
    Html(include_str!("../templates/loans.html"))
}

async fn nfts_page() -> impl IntoResponse {
    Html(include_str!("../templates/nfts.html"))
}

async fn invest_page() -> impl IntoResponse {
    Html(include_str!("../templates/invest.html"))
}

async fn wallet_page() -> impl IntoResponse {
    Html(include_str!("../templates/wallet.html"))
}

async fn blockchain_explorer_page() -> impl IntoResponse {
    Html(include_str!("../templates/blockchain_explorer.html"))
}

async fn architecture_page() -> impl IntoResponse {
    Html(include_str!("../templates/architecture.html"))
}

// API handlers with mock data
async fn get_students(State(_state): State<AppState>) -> impl IntoResponse {
    let students = vec![
        Student {
            id: Uuid::new_v4(),
            email: "alice@stanford.edu".to_string(),
            name: "Alice Chen".to_string(),
            university: "Stanford".to_string(),
            wallet_address: "0x1234...".to_string(),
            reputation_score: 98.5,
            verified: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        },
        Student {
            id: Uuid::new_v4(),
            email: "bob@mit.edu".to_string(),
            name: "Bob Martinez".to_string(),
            university: "MIT".to_string(),
            wallet_address: "0x5678...".to_string(),
            reputation_score: 97.2,
            verified: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    ];
    Json(ApiResponse::success(students))
}

async fn create_student(
    State(_state): State<AppState>,
    Json(student): Json<Student>,
) -> impl IntoResponse {
    let created_student = Student {
        id: Uuid::new_v4(),
        email: student.email,
        name: student.name,
        university: student.university,
        wallet_address: student.wallet_address,
        reputation_score: student.reputation_score,
        verified: student.verified,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    Json(ApiResponse::success(created_student))
}

async fn get_student(
    State(_state): State<AppState>,
    Path(_id): Path<Uuid>,
) -> impl IntoResponse {
    let student = Student {
        id: Uuid::new_v4(),
        email: "alice@stanford.edu".to_string(),
        name: "Alice Chen".to_string(),
        university: "Stanford".to_string(),
        wallet_address: "0x1234...".to_string(),
        reputation_score: 98.5,
        verified: true,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    Json(ApiResponse::success(student))
}

async fn get_market_items(State(state): State<AppState>) -> impl IntoResponse {
    // Get marketplace items from database/blockchain storage
    match state.marketplace.get_all_items().await {
        Ok(items) => Json(ApiResponse::success(items)),
        Err(e) => {
            error!("Failed to get marketplace items: {}", e);
            // Return empty list instead of mock data for real blockchain system
            Json(ApiResponse::success(Vec::<MarketItem>::new()))
        }
    }
}

async fn create_market_item(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(request): Json<CreateMarketItemRequest>,
) -> impl IntoResponse {
    // Get current user
    let user = match get_current_user(&headers, &state).await {
        Ok(user) => user,
        Err(_) => return Json(ApiResponse::error("Authentication required".to_string())),
    };
    
    // Create item with generated fields
    let item = MarketItem {
        id: Uuid::new_v4(),
        seller_id: user.id,
        title: request.title,
        description: request.description,
        category: request.category,
        price: request.price,
        currency: request.currency,
        item_type: request.item_type,
        status: "active".to_string(),
        images: request.images,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    
    // Store item in marketplace
    match state.marketplace.create_item(item.clone()).await {
        Ok(()) => {
            info!("Created marketplace item: {} - {}", item.id, item.title);
            Json(ApiResponse::success(item))
        }
        Err(e) => {
            error!("Failed to create marketplace item: {}", e);
            Json(ApiResponse::error("Failed to create marketplace item".to_string()))
        }
    }
}

async fn get_market_item(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    // Get specific marketplace item by ID
    match state.marketplace.get_item(id).await {
        Ok(Some(item)) => Json(ApiResponse::success(item)),
        Ok(None) => Json(ApiResponse::error("Item not found".to_string())),
        Err(e) => {
            error!("Failed to get marketplace item {}: {}", id, e);
            Json(ApiResponse::error("Failed to retrieve item".to_string()))
        }
    }
}

async fn dashboard_stats_handler(State(state): State<AppState>) -> impl IntoResponse {
    // Get real blockchain statistics
    let network_status = state.backend.get_network_status().await
        .unwrap_or_else(|_| serde_json::json!({}));
    
    // Get real marketplace statistics
    let marketplace_stats = state.marketplace.get_stats().await
        .unwrap_or_else(|_| serde_json::json!({"active_listings": 0}));
    
    let active_listings = marketplace_stats.get("active_listings")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    
    let stats = DashboardStats {
        total_students: 1, // Single-user system for now
        active_listings,
        total_loans: 0, // No loan system implemented yet
        minted_nfts: 0, // No NFT system implemented yet
        funded_projects: 0, // No project funding system yet
        total_volume: network_status.get("mempool_size")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0),
    };
    Json(ApiResponse::success(stats))
}



// ============================================================================
// WEBSOCKET HANDLER FOR REAL-TIME UPDATES
// ============================================================================

async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> Response {
    ws.on_upgrade(|socket| websocket_connection(socket, state))
}

async fn websocket_connection(mut socket: WebSocket, state: AppState) {
    info!("🔌 New WebSocket connection established");
    
    // Send initial blockchain status
    if let Ok(status) = state.backend.get_network_status().await {
        let message = serde_json::json!({
            "type": "network_status",
            "data": status
        });
        
        if socket.send(axum::extract::ws::Message::Text(
            serde_json::to_string(&message).unwrap()
        )).await.is_err() {
            return;
        }
    }

    // Set up periodic updates
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(5));
    
    loop {
        tokio::select! {
            _ = interval.tick() => {
                // Send periodic updates
                if let Ok(status) = state.backend.get_network_status().await {
                    let message = serde_json::json!({
                        "type": "network_update",
                        "data": status,
                        "timestamp": chrono::Utc::now().to_rfc3339()
                    });
                    
                    if socket.send(axum::extract::ws::Message::Text(
                        serde_json::to_string(&message).unwrap()
                    )).await.is_err() {
                        break;
                    }
                }
            }
            
            msg = socket.recv() => {
                match msg {
                    Some(Ok(axum::extract::ws::Message::Text(text))) => {
                        // Handle client messages
                        if let Ok(request) = serde_json::from_str::<serde_json::Value>(&text) {
                            handle_websocket_request(&mut socket, &state, request).await;
                        }
                    }
                    Some(Ok(axum::extract::ws::Message::Close(_))) => break,
                    _ => {}
                }
            }
        }
    }
    
    info!("🔌 WebSocket connection closed");
}

async fn handle_websocket_request(
    socket: &mut WebSocket,
    state: &AppState,
    request: serde_json::Value,
) {
    let request_type = request.get("type").and_then(|v| v.as_str()).unwrap_or("");
    
    let response = match request_type {
        "get_balance" => {
            if let Some(address) = request.get("address").and_then(|v| v.as_str()) {
                match state.backend.get_wallet_balance(address).await {
                    Ok(balance) => serde_json::json!({
                        "type": "balance_response",
                        "address": address,
                        "balance": balance
                    }),
                    Err(_) => serde_json::json!({
                        "type": "error",
                        "message": "Failed to get balance"
                    })
                }
            } else {
                serde_json::json!({
                    "type": "error",
                    "message": "Invalid address"
                })
            }
        },
        
        "start_mining" => {
            if let Some(address) = request.get("miner_address").and_then(|v| v.as_str()) {
                match state.backend.start_mining(address.to_string()).await {
                    Ok(_) => serde_json::json!({
                        "type": "mining_started",
                        "miner_address": address
                    }),
                    Err(_) => serde_json::json!({
                        "type": "error",
                        "message": "Failed to start mining"
                    })
                }
            } else {
                serde_json::json!({
                    "type": "error",
                    "message": "Invalid miner address"
                })
            }
        },
        
        "stop_mining" => {
            match state.backend.stop_mining().await {
                Ok(_) => serde_json::json!({
                    "type": "mining_stopped"
                }),
                Err(_) => serde_json::json!({
                    "type": "error", 
                    "message": "Failed to stop mining"
                })
            }
        },
        
        _ => serde_json::json!({
            "type": "error",
            "message": "Unknown request type"
        })
    };
    
    let _ = socket.send(axum::extract::ws::Message::Text(
        serde_json::to_string(&response).unwrap()
    )).await;
}

// ============================================================================
// AUTHENTICATION API HANDLERS
// ============================================================================

async fn api_login(
    State(state): State<AppState>,
    Json(request): Json<LoginRequest>,
) -> impl IntoResponse {
    tracing::info!("🔐 Login attempt for username: {}", request.username);
    match state.user_manager.login_user(request).await {
        Ok((user, session_token)) => {
            tracing::info!("✅ Login successful for user: {}, session: {}", user.username, session_token);
            let response = serde_json::json!({
                "success": true,
                "user": {
                    "id": user.id,
                    "username": user.username,
                    "email": user.email,
                    "wallet_address": user.wallet_address,
                    "university": user.university,
                    "reputation_score": user.reputation_score,
                    "created_at": user.created_at
                },
                "session_token": session_token
            });
            
            let mut headers = HeaderMap::new();
            headers.insert(
                "set-cookie",
                format!("session={}; Path=/; HttpOnly; Max-Age=86400; SameSite=Lax; Secure=false", session_token)
                    .parse().unwrap()
            );
            
            (StatusCode::OK, headers, Json(response))
        }
        Err(error) => {
            let response = serde_json::json!({
                "success": false,
                "error": error
            });
            (StatusCode::UNAUTHORIZED, HeaderMap::new(), Json(response))
        }
    }
}

async fn api_register(
    State(state): State<AppState>,
    Json(request): Json<RegisterRequest>,
) -> impl IntoResponse {
    match state.user_manager.register_user(request).await {
        Ok(user) => {
            let response = serde_json::json!({
                "success": true,
                "user": {
                    "id": user.id,
                    "username": user.username,
                    "email": user.email,
                    "wallet_address": user.wallet_address,
                    "university": user.university,
                    "reputation_score": user.reputation_score,
                    "created_at": user.created_at
                },
                "message": "User registered successfully! Please login."
            });
            Json(ApiResponse::success(response))
        }
        Err(error) => {
            Json(ApiResponse::error(error))
        }
    }
}

async fn api_logout(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let session_token = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(|auth| auth.strip_prefix("Bearer "))
        .or_else(|| {
            headers
                .get("cookie")
                .and_then(|value| value.to_str().ok())
                .and_then(|cookies| {
                    cookies.split(';')
                        .find_map(|cookie| {
                            let cookie = cookie.trim();
                            cookie.strip_prefix("session=")
                        })
                })
        });

    if let Some(token) = session_token {
        let _ = state.user_manager.logout_user(token).await;
    }

    let mut headers = HeaderMap::new();
    headers.insert(
        "set-cookie",
        "session=; Path=/; HttpOnly; Max-Age=0; SameSite=Lax".parse().unwrap()
    );

    (StatusCode::OK, headers, Json(ApiResponse::success("Logged out successfully")))
}

async fn api_current_user(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> impl IntoResponse {
    match get_current_user(&headers, &state).await {
        Ok(user) => {
            let user_wallet = state.user_manager.get_user_wallet(&user).await.ok();
            
            let response = serde_json::json!({
                "id": user.id,
                "username": user.username,
                "email": user.email,
                "wallet_address": user.wallet_address,
                "university": user.university,
                "reputation_score": user.reputation_score,
                "created_at": user.created_at,
                "last_login": user.last_login,
                "wallet": user_wallet.map(|w| serde_json::json!({
                    "id": w.id,
                    "name": w.name,
                    "address": w.address,
                    "balance": w.balance
                }))
            });
            Json(ApiResponse::success(response))
        }
        Err(error) => {
            Json(ApiResponse::error(error))
        }
    }
}

async fn get_all_users(State(state): State<AppState>) -> impl IntoResponse {
    let users = state.user_manager.list_users().await;
    
    let user_list: Vec<serde_json::Value> = users
        .into_iter()
        .map(|user| serde_json::json!({
            "id": user.id,
            "username": user.username,
            "wallet_address": user.wallet_address,
            "university": user.university,
            "reputation_score": user.reputation_score,
            "created_at": user.created_at,
            "last_login": user.last_login,
            "is_verified": user.is_verified
        }))
        .collect();

    Json(ApiResponse::success(user_list))
}

async fn get_current_user_info(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> impl IntoResponse {
    tracing::info!("🔍 /api/user/info called with headers: {:?}", headers.keys().collect::<Vec<_>>());
    
    // Log cookies specifically
    if let Some(cookie_header) = headers.get("cookie") {
        if let Ok(cookie_str) = cookie_header.to_str() {
            tracing::info!("🍪 Cookie header: {}", cookie_str);
        }
    } else {
        tracing::info!("❌ No cookie header found");
    }
    
    // Log authorization header specifically
    if let Some(auth_header) = headers.get("authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            tracing::info!("🔐 Authorization header: {}", auth_str);
        }
    } else {
        tracing::info!("❌ No authorization header found");
    }
    
    match get_current_user(&headers, &state).await {
        Ok(user) => {
            tracing::info!("✅ User found: {}", user.username);
            
            let user_wallet = state.user_manager.get_user_wallet(&user).await
                .unwrap_or_else(|_| blockchain_core::wallet::Wallet {
                    id: user.wallet_id,
                    name: format!("{}'s Wallet", user.username),
                    private_key: [0u8; 32],
                    public_key: [0u8; 33],
                    address: user.wallet_address.clone(),
                    balance: 100, // Demo balance (u64)
                    created_at: chrono::Utc::now(),
                });
            
            let user_info = serde_json::json!({
                "id": user.id,
                "username": user.username,
                "email": user.email,
                "wallet_address": user.wallet_address,
                "university": user.university,
                "reputation_score": user.reputation_score,
                "created_at": user.created_at,
                "wallet": {
                    "balance": user_wallet.balance,
                    "address": user_wallet.address
                }
            });
            
            tracing::info!("📤 Sending user info response: {}", serde_json::to_string_pretty(&user_info).unwrap_or_default());
            
            Json(ApiResponse::success(user_info))
        }
        Err(error) => {
            tracing::error!("❌ Failed to get current user: {}", error);
            Json(ApiResponse::error(error))
        }
    }
}

// Blockchain API wrappers
async fn blockchain_create_wallet(
    State(state): State<AppState>,
    Json(req): Json<serde_json::Value>,
) -> impl IntoResponse {
    let username = req.get("username").and_then(|v| v.as_str()).unwrap_or("unknown");
    match state.backend.wallets.write().await.create_wallet(username.to_string()) {
        Ok(wallet) => Json(serde_json::json!({
            "address": wallet.address,
            "username": username,
            "created_at": chrono::Utc::now().to_rfc3339()
        })),
        Err(_) => Json(serde_json::json!({
            "error": "Failed to create wallet"
        }))
    }
}

async fn blockchain_get_balance(
    State(state): State<AppState>,
    Path(address): Path<String>,
) -> impl IntoResponse {
    // Get REAL blockchain balance from PRODUCTION UTXO set
    match state.backend.get_wallet_balance(&address).await {
        Ok(balance_satoshis) => {
            let balance_edu = balance_satoshis as f64 / 100_000_000.0;
            info!("💰 REAL balance for {}: {} EDU ({} satoshis)", address, balance_edu, balance_satoshis);
            
            Json(serde_json::json!({ 
                "balance": balance_edu,
                "balance_satoshis": balance_satoshis,
                "address": address,
                "balance_type": "PRODUCTION_UTXO_VALIDATED",
                "last_updated": chrono::Utc::now().to_rfc3339()
            }))
        },
        Err(e) => {
            error!("❌ Failed to get real balance for {}: {}", address, e);
            Json(serde_json::json!({ 
                "balance": 0.0,
                "balance_satoshis": 0,
                "address": address,
                "error": format!("Failed to fetch real blockchain balance: {}", e)
            }))
        }
    }
}

async fn blockchain_network_status(State(state): State<AppState>) -> impl IntoResponse {
    let peer_count = state.backend.network.get_connected_peers().await.len();
    Json(serde_json::json!({
        "connected_peers": peer_count,
        "network_status": "connected"
    }))
}

async fn blockchain_mining_stats(State(_state): State<AppState>) -> impl IntoResponse {
    Json(serde_json::json!({
        "is_mining": false,
        "blocks_mined": 0,
        "hash_rate": 0.0,
        "difficulty": 1.0
    }))
}

// Dashboard Quick Actions API Handlers



async fn api_send_transaction(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(req): Json<SendTransactionRequest>,
) -> impl IntoResponse {
    // Get authenticated user
    let user = match get_current_user(&headers, &state).await {
        Ok(user) => user,
        Err(_) => return Json(serde_json::json!({
            "success": false,
            "message": "Authentication required"
        }))
    };
    
    info!("💸 REAL TRANSACTION: {} EDU from {} to {}", req.amount, user.username, req.recipient);
    
    // Convert EDU to satoshis (1 EDU = 100,000,000 satoshis)
    let amount_satoshis = (req.amount * 100_000_000.0) as u64;
    
    // Send REAL blockchain transaction with ECDSA signatures
    match state.backend.send_transaction(&user.wallet_address, &req.recipient, amount_satoshis, req.message).await {
        Ok(tx_hash) => {
            info!("✅ REAL transaction sent with hash: {}", tx_hash);
            Json(serde_json::json!({
                "success": true,
                "message": "REAL transaction sent with ECDSA signature",
                "transaction_hash": tx_hash,
                "amount": req.amount,
                "amount_satoshis": amount_satoshis,
                "recipient": req.recipient,
                "from_address": user.wallet_address,
                "transaction_type": "PRODUCTION_ECDSA_SIGNED"
            }))
        },
        Err(e) => {
            error!("❌ Failed to send real transaction: {}", e);
            Json(serde_json::json!({
                "success": false,
                "message": format!("Failed to send transaction: {}", e)
            }))
        }
    }
}

async fn api_get_transactions(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> impl IntoResponse {
    // Get authenticated user
    let user = match get_current_user(&headers, &state).await {
        Ok(user) => user,
        Err(_) => return Json(serde_json::json!({
            "success": false,
            "message": "Authentication required"
        }))
    };
    
    info!("📜 Get transactions for user: {} (wallet: {})", user.username, user.wallet_address);
    
    // Use the actual wallet address from the user object
    let user_address = &user.wallet_address;
    
    // Get transaction history from blockchain
    match state.backend.get_transaction_history(user_address).await {
        Ok(transactions) => {
            // Also get pending transactions from mempool
            let pending_transactions = state.backend.get_mempool_transactions(user_address).await
                .unwrap_or_default();
            
            // Combine confirmed and pending transactions
            let mut all_transactions = transactions;
            all_transactions.extend(pending_transactions);
            
            // Sort by timestamp (newest first)
            all_transactions.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
            
            info!("📜 Returning {} transactions for user {}", all_transactions.len(), user.username);
            
            Json(serde_json::json!({
                "success": true,
                "transactions": all_transactions
            }))
        }
        Err(e) => {
            error!("Failed to get transaction history for {}: {}", user.username, e);
            Json(serde_json::json!({
                "success": false,
                "message": "Failed to retrieve transaction history"
            }))
        }
    }
}

async fn api_get_pending_transactions(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> impl IntoResponse {
    // Get authenticated user
    let user = match get_current_user(&headers, &state).await {
        Ok(user) => user,
        Err(_) => return Json(serde_json::json!({
            "success": false,
            "message": "Authentication required"
        }))
    };
    
    info!("🔄 Get pending transactions for user: {}", user.username);
    
    // Get user's wallet address
    let user_address = format!("edu1q{}", sha256_hash(&user.username)[0..40].to_lowercase());
    
    // Get pending transactions from mempool
    match state.backend.get_mempool_transactions(&user_address).await {
        Ok(pending_transactions) => {
            info!("🔄 Found {} pending transactions for user {}", pending_transactions.len(), user.username);
            
            Json(serde_json::json!({
                "success": true,
                "pending_transactions": pending_transactions
            }))
        }
        Err(e) => {
            error!("Failed to get pending transactions for {}: {}", user.username, e);
            Json(serde_json::json!({
                "success": false,
                "message": "Failed to retrieve pending transactions"
            }))
        }
    }
}

async fn api_get_recent_transactions(
    _headers: HeaderMap,
    State(state): State<AppState>,
) -> impl IntoResponse {
    info!("📊 Get recent network transactions");
    
    // Get recent transactions across the network (public data)
    match state.backend.get_recent_transactions(50).await {
        Ok(recent_transactions) => {
            info!("📊 Found {} recent network transactions", recent_transactions.len());
            
            Json(serde_json::json!({
                "success": true,
                "recent_transactions": recent_transactions,
                "network_activity": recent_transactions.len()
            }))
        }
        Err(e) => {
            error!("Failed to get recent transactions: {}", e);
            Json(serde_json::json!({
                "success": false,
                "message": "Failed to retrieve recent transactions"
            }))
        }
    }
}

async fn api_mine_block(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> impl IntoResponse {
    // Get authenticated user
    let user = match get_current_user(&headers, &state).await {
        Ok(user) => user,
        Err(_) => return Json(serde_json::json!({
            "success": false,
            "message": "Authentication required"
        }))
    };
    
    info!("⛏️ REAL MINING: Starting block mining for user: {}", user.username);
    
    // Mine REAL block with ECDSA transaction validation
    match state.backend.mine_block(user.wallet_address.clone()).await {
        Ok((block_hash, reward, tx_count)) => {
            let reward_edu = reward as f64 / 100_000_000.0;
            info!("✅ REAL block mined: {} (reward: {} EDU, txs: {})", block_hash, reward_edu, tx_count);
            
            Json(serde_json::json!({
                "success": true,
                "message": "REAL block mined with ECDSA validation",
                "block_hash": block_hash,
                "reward": reward_edu,
                "reward_satoshis": reward,
                "transactions_count": tx_count,
                "miner_address": user.wallet_address,
                "mining_type": "PRODUCTION_CONSENSUS_VALIDATED"
            }))
        },
        Err(e) => {
            error!("❌ Failed to mine real block: {}", e);
            Json(serde_json::json!({
                "success": false,
                "message": format!("Mining failed: {}", e)
            }))
        }
    }
}

async fn api_network_status(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> impl IntoResponse {
    // Get authenticated user (optional for network status)
    let _user = get_current_user(&headers, &state).await;
    
    info!("🌐 Get REAL network status from production blockchain");
    
    // Get REAL network status from blockchain backend
    match state.backend.get_network_status().await {
        Ok(network_status) => {
            info!("✅ Retrieved real network status: {:?}", network_status);
            
            Json(serde_json::json!({
                "success": true,
                "peer_count": network_status["connected_peers"],
                "block_height": network_status["block_height"],
                "best_block_hash": network_status["best_block_hash"],
                "difficulty": network_status["difficulty"],
                "hash_rate": network_status["hash_rate"],
                "pending_tx": network_status["mempool_size"],
                "mempool_bytes": network_status["mempool_bytes"],
                "node_status": "PRODUCTION_ACTIVE",
                "blockchain_type": network_status["blockchain_type"],
                "is_mining": network_status["is_mining"],
                "blocks_mined": network_status["blocks_mined"],
                "total_work": network_status["total_work"],
                "network_uptime": network_status["network_uptime"],
                "raw_network_data": network_status
            }))
        },
        Err(e) => {
            error!("❌ Failed to get real network status: {}", e);
            Json(serde_json::json!({
                "success": false,
                "message": format!("Failed to get network status: {}", e)
            }))
        }
    }
}

async fn get_user_statistics(State(state): State<AppState>) -> impl IntoResponse {
    let stats = state.user_manager.get_user_stats().await;
    Json(ApiResponse::success(stats))
}

// ==================== NFT API HANDLERS ====================

#[derive(Debug, Deserialize)]
struct NftMintRequest {
    name: String,
    description: Option<String>,
    image_url: Option<String>,
    metadata: Option<String>,
}

async fn api_nft_mint(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(request): Json<NftMintRequest>,
) -> impl IntoResponse {
    let user = match get_current_user(&headers, &state).await {
        Ok(u) => u,
        Err(_) => return Json(serde_json::json!({
            "success": false,
            "message": "Authentication required"
        })),
    };

    info!("🎨 Minting NFT: {} for user {}", request.name, user.username);

    // Create NFT transaction (1 satoshi UTXO with NFT metadata)
    let nft_id = format!("nft_{}", Uuid::new_v4().simple());
    let timestamp = Utc::now().timestamp();

    // Send 1 satoshi transaction to represent NFT
    let tx_result = state.backend.send_transaction(
        &user.wallet_address,
        &user.wallet_address, // NFT owned by creator initially
        1, // 1 satoshi
        Some(format!("NFT_MINT:{}", nft_id))
    ).await;

    match tx_result {
        Ok(tx_hash) => {
            // Save NFT to database
            let nft = crate::database::DbNft {
                id: None,
                nft_id: nft_id.clone(),
                name: request.name.clone(),
                description: request.description.clone(),
                image_url: request.image_url.clone(),
                creator_address: user.wallet_address.clone(),
                current_owner: user.wallet_address.clone(),
                metadata: request.metadata.clone(),
                mint_tx_hash: tx_hash.clone(),
                mint_timestamp: timestamp,
                is_burned: false,
            };

            match state.database.mint_nft(&nft).await {
                Ok(_) => {
                    info!("✅ NFT minted: {} (tx: {})", nft_id, tx_hash);
                    Json(serde_json::json!({
                        "success": true,
                        "nft_id": nft_id,
                        "tx_hash": tx_hash,
                        "message": "NFT minted successfully"
                    }))
                }
                Err(e) => {
                    error!("❌ Failed to save NFT to database: {}", e);
                    Json(serde_json::json!({
                        "success": false,
                        "message": format!("Failed to save NFT: {}", e)
                    }))
                }
            }
        }
        Err(e) => {
            error!("❌ Failed to create NFT transaction: {}", e);
            Json(serde_json::json!({
                "success": false,
                "message": format!("Failed to mint NFT: {}", e)
            }))
        }
    }
}

async fn api_nft_list(State(state): State<AppState>) -> impl IntoResponse {
    match state.database.list_all_nfts(100).await {
        Ok(nfts) => Json(serde_json::json!({
            "success": true,
            "nfts": nfts
        })),
        Err(e) => {
            error!("❌ Failed to list NFTs: {}", e);
            Json(serde_json::json!({
                "success": false,
                "message": format!("Failed to list NFTs: {}", e)
            }))
        }
    }
}

async fn api_nft_owned(
    Path(address): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    match state.database.get_nfts_by_owner(&address).await {
        Ok(nfts) => Json(serde_json::json!({
            "success": true,
            "nfts": nfts,
            "count": nfts.len()
        })),
        Err(e) => {
            error!("❌ Failed to get NFTs for {}: {}", address, e);
            Json(serde_json::json!({
                "success": false,
                "message": format!("Failed to get NFTs: {}", e)
            }))
        }
    }
}

async fn api_nft_get(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    match state.database.get_nft_by_id(&id).await {
        Ok(Some(nft)) => Json(serde_json::json!({
            "success": true,
            "nft": nft
        })),
        Ok(None) => Json(serde_json::json!({
            "success": false,
            "message": "NFT not found"
        })),
        Err(e) => {
            error!("❌ Failed to get NFT {}: {}", id, e);
            Json(serde_json::json!({
                "success": false,
                "message": format!("Failed to get NFT: {}", e)
            }))
        }
    }
}

#[derive(Debug, Deserialize)]
struct NftTransferRequest {
    nft_id: String,
    to_address: String,
}

async fn api_nft_transfer(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(request): Json<NftTransferRequest>,
) -> impl IntoResponse {
    let user = match get_current_user(&headers, &state).await {
        Ok(u) => u,
        Err(_) => return Json(serde_json::json!({
            "success": false,
            "message": "Authentication required"
        })),
    };

    // Get NFT to verify ownership
    let nft = match state.database.get_nft_by_id(&request.nft_id).await {
        Ok(Some(n)) => n,
        Ok(None) => return Json(serde_json::json!({
            "success": false,
            "message": "NFT not found"
        })),
        Err(e) => return Json(serde_json::json!({
            "success": false,
            "message": format!("Error: {}", e)
        })),
    };

    // Verify user owns the NFT
    if nft.current_owner != user.wallet_address {
        return Json(serde_json::json!({
            "success": false,
            "message": "You don't own this NFT"
        }));
    }

    // Transfer NFT (send 1 satoshi transaction)
    match state.backend.send_transaction(
        &user.wallet_address,
        &request.to_address,
        1,
        Some(format!("NFT_TRANSFER:{}", request.nft_id))
    ).await {
        Ok(tx_hash) => {
            // Update NFT ownership in database
            match state.database.transfer_nft(
                &request.nft_id,
                &user.wallet_address,
                &request.to_address,
                &tx_hash,
                Utc::now().timestamp()
            ).await {
                Ok(_) => {
                    info!("✅ NFT transferred: {} to {}", request.nft_id, request.to_address);
                    Json(serde_json::json!({
                        "success": true,
                        "tx_hash": tx_hash,
                        "message": "NFT transferred successfully"
                    }))
                }
                Err(e) => {
                    error!("❌ Failed to update NFT ownership: {}", e);
                    Json(serde_json::json!({
                        "success": false,
                        "message": format!("Failed to update NFT: {}", e)
                    }))
                }
            }
        }
        Err(e) => {
            error!("❌ Failed to transfer NFT: {}", e);
            Json(serde_json::json!({
                "success": false,
                "message": format!("Failed to transfer NFT: {}", e)
            }))
        }
    }
}

// ==================== LOAN API HANDLERS ====================

#[derive(Debug, Deserialize)]
struct LoanApplicationRequest {
    full_name: String,
    university: String,
    field_of_study: String,
    gpa: Option<f64>,
    test_score: Option<i32>,
    achievements: Option<String>,
    requested_amount: i64,
    interest_rate: Option<f64>,
    repayment_term_months: Option<i32>,
    loan_purpose: Option<String>,
    graduation_year: Option<i32>,
    expected_career: Option<String>,
    expected_salary: Option<i64>,
}

async fn api_loan_apply(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(request): Json<LoanApplicationRequest>,
) -> impl IntoResponse {
    let user = match get_current_user(&headers, &state).await {
        Ok(u) => u,
        Err(_) => return Json(serde_json::json!({
            "success": false,
            "message": "Authentication required"
        })),
    };

    info!("📋 Loan application from {}: {} EDU", user.username, request.requested_amount as f64 / 100_000_000.0);

    // Calculate Proof-of-Potential score
    let mut score = 5.0; // Base score
    if let Some(gpa) = request.gpa {
        score += (gpa / 4.0) * 2.5; // Up to 2.5 points for GPA
    }
    if let Some(test_score) = request.test_score {
        score += (test_score as f64 / 1600.0) * 2.5; // Up to 2.5 points for test scores
    }

    let loan_id = format!("loan_{}", Uuid::new_v4().simple());
    
    let loan = crate::database::DbLoanApplication {
        id: None,
        loan_id: loan_id.clone(),
        applicant_username: user.username.clone(),
        applicant_address: user.wallet_address.clone(),
        full_name: request.full_name,
        university: request.university,
        field_of_study: request.field_of_study,
        gpa: request.gpa,
        test_score: request.test_score.map(|s| s as i64),
        achievements: request.achievements,
        requested_amount: request.requested_amount,
        interest_rate: request.interest_rate,
        repayment_term_months: request.repayment_term_months.map(|m| m as i64),
        loan_purpose: request.loan_purpose,
        graduation_year: request.graduation_year.map(|y| y as i64),
        expected_career: request.expected_career,
        expected_salary: request.expected_salary.map(|s| s as i64),
        proof_of_potential_score: Some(score),
        status: "pending".to_string(),
        funded_amount: Some(0),
        funding_tx_hash: None,
    };

    match state.database.create_loan_application(&loan).await {
        Ok(_) => {
            info!("✅ Loan application created: {} (score: {:.1})", loan_id, score);
            Json(serde_json::json!({
                "success": true,
                "loan_id": loan_id,
                "proof_of_potential_score": score,
                "message": "Loan application submitted successfully"
            }))
        }
        Err(e) => {
            error!("❌ Failed to create loan application: {}", e);
            Json(serde_json::json!({
                "success": false,
                "message": format!("Failed to submit loan application: {}", e)
            }))
        }
    }
}

async fn api_loan_list(
    Query(params): Query<HashMap<String, String>>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let status = params.get("status").map(|s| s.as_str()).unwrap_or("pending");
    let limit = params.get("limit").and_then(|s| s.parse::<i64>().ok()).unwrap_or(50);

    match state.database.list_loans_by_status(status, limit).await {
        Ok(loans) => Json(serde_json::json!({
            "success": true,
            "loans": loans,
            "count": loans.len()
        })),
        Err(e) => {
            error!("❌ Failed to list loans: {}", e);
            Json(serde_json::json!({
                "success": false,
                "message": format!("Failed to list loans: {}", e)
            }))
        }
    }
}

async fn api_loan_get(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    match state.database.get_loan_by_id(&id).await {
        Ok(Some(loan)) => Json(serde_json::json!({
            "success": true,
            "loan": loan
        })),
        Ok(None) => Json(serde_json::json!({
            "success": false,
            "message": "Loan not found"
        })),
        Err(e) => {
            error!("❌ Failed to get loan {}: {}", id, e);
            Json(serde_json::json!({
                "success": false,
                "message": format!("Failed to get loan: {}", e)
            }))
        }
    }
}

#[derive(Debug, Deserialize)]
struct LoanFundRequest {
    loan_id: String,
    amount: i64,
}

async fn api_loan_fund(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(request): Json<LoanFundRequest>,
) -> impl IntoResponse {
    let user = match get_current_user(&headers, &state).await {
        Ok(u) => u,
        Err(_) => return Json(serde_json::json!({
            "success": false,
            "message": "Authentication required"
        })),
    };

    // Get loan details
    let loan = match state.database.get_loan_by_id(&request.loan_id).await {
        Ok(Some(l)) => l,
        Ok(None) => return Json(serde_json::json!({
            "success": false,
            "message": "Loan not found"
        })),
        Err(e) => return Json(serde_json::json!({
            "success": false,
            "message": format!("Error: {}", e)
        })),
    };

    if loan.status != "pending" && loan.status != "approved" {
        return Json(serde_json::json!({
            "success": false,
            "message": "Loan is not available for funding"
        }));
    }

    info!("💰 Funding loan {} with {} EDU from {}", request.loan_id, request.amount as f64 / 100_000_000.0, user.username);

    // Send funding transaction
    match state.backend.send_transaction(
        &user.wallet_address,
        &loan.applicant_address,
        request.amount as u64,
        Some(format!("LOAN_FUNDING:{}", request.loan_id))
    ).await {
        Ok(tx_hash) => {
            // Record funding in database
            match state.database.fund_loan(
                &request.loan_id,
                &user.wallet_address,
                request.amount,
                &tx_hash,
                Utc::now().timestamp()
            ).await {
                Ok(_) => {
                    info!("✅ Loan funded: {} by {} (tx: {})", request.loan_id, user.username, tx_hash);
                    Json(serde_json::json!({
                        "success": true,
                        "tx_hash": tx_hash,
                        "message": "Loan funded successfully"
                    }))
                }
                Err(e) => {
                    error!("❌ Failed to record loan funding: {}", e);
                    Json(serde_json::json!({
                        "success": false,
                        "message": format!("Failed to record funding: {}", e)
                    }))
                }
            }
        }
        Err(e) => {
            error!("❌ Failed to send funding transaction: {}", e);
            Json(serde_json::json!({
                "success": false,
                "message": format!("Failed to fund loan: {}", e)
            }))
        }
    }
}

/// Get blockchain synchronization status
async fn api_sync_status(State(state): State<AppState>) -> impl IntoResponse {
    let sync_status = state.backend.get_sync_status().await;
    Json(sync_status)
}

/// Trigger blockchain synchronization
async fn api_sync_blockchain(State(state): State<AppState>) -> impl IntoResponse {
    match state.backend.sync_blockchain().await {
        Ok(_) => {
            info!("🔄 Manual blockchain sync triggered");
            Json(serde_json::json!({
                "success": true,
                "message": "Blockchain synchronization started"
            }))
        }
        Err(e) => {
            error!("❌ Failed to trigger sync: {}", e);
            Json(serde_json::json!({
                "success": false,
                "message": format!("Failed to start sync: {}", e)
            }))
        }
    }
}

