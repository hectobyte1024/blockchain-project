//! Edunet GUI Backend Server
//! 
//! Web interface for the Edunet blockchain platform for university students.
//! Provides marketplace, lending, NFT minting, and investment pool functionality.

use axum::{
    extract::{Path, State},
    response::{Html, Json, IntoResponse},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::{
    cors::CorsLayer,
    services::ServeDir,
};
use tracing::{info, error};
use uuid::Uuid;
use chrono::{DateTime, Utc};

// Blockchain integration
use blockchain_network::NetworkManager;

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub blockchain: Arc<NetworkManager>,
}

/// Student user model
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
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

/// Marketplace item model
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
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

/// Loan application model
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct LoanApplication {
    pub id: Uuid,
    pub student_id: Uuid,
    pub amount_requested: f64,
    pub purpose: String,
    pub proof_of_potential_score: f64,
    pub academic_data: String, // JSON containing GPA, courses, etc.
    pub status: String, // "pending", "approved", "rejected", "funded"
    pub interest_rate: Option<f64>,
    pub term_months: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// NFT model
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct NFT {
    pub id: Uuid,
    pub creator_id: Uuid,
    pub title: String,
    pub description: String,
    pub content_type: String, // "image", "music", "video", "document", "software"
    pub content_url: String,
    pub metadata: String, // JSON metadata
    pub token_id: Option<String>,
    pub minted: bool,
    pub price: Option<f64>,
    pub royalty_percentage: f64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Investment project model
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct InvestmentProject {
    pub id: Uuid,
    pub creator_id: Uuid,
    pub title: String,
    pub description: String,
    pub project_type: String, // "thesis", "hackathon", "startup", "research"
    pub funding_goal: f64,
    pub current_funding: f64,
    pub deadline: DateTime<Utc>,
    pub status: String, // "active", "funded", "completed", "cancelled"
    pub documents: Option<String>, // JSON array of document URLs
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

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

    info!("Starting Edunet GUI backend server...");

    // Initialize database
    let database_url = "sqlite:edunet.db";
    let db = SqlitePool::connect(database_url).await?;
    
    // Run migrations
    sqlx::migrate!("./migrations").run(&db).await?;

    // Initialize blockchain network (simplified for demo)
    let blockchain_config = blockchain_network::NetworkConfig::default();
    let blockchain = Arc::new(NetworkManager::new(blockchain_config)?);

    let state = AppState { db, blockchain };

    // Build the application router
    let app = Router::new()
        // Static files
        .nest_service("/static", ServeDir::new("static"))
        
        // Main pages
        .route("/", get(dashboard_page))
        .route("/marketplace", get(marketplace_page))
        .route("/loans", get(loans_page))
        .route("/nfts", get(nfts_page))
        .route("/invest", get(invest_page))
        
        // API routes
        .route("/api/students", get(get_students).post(create_student))
        .route("/api/students/:id", get(get_student))
        .route("/api/marketplace", get(get_market_items).post(create_market_item))
        .route("/api/marketplace/:id", get(get_market_item))
        .route("/api/loans", get(get_loans).post(create_loan))
        .route("/api/loans/:id", get(get_loan))
        .route("/api/nfts", get(get_nfts).post(create_nft))
        .route("/api/nfts/:id", get(get_nft))
        .route("/api/projects", get(get_projects).post(create_project))
        .route("/api/projects/:id", get(get_project))
        .route("/api/dashboard/stats", get(get_dashboard_stats))
        
        // Blockchain integration
        .route("/api/blockchain/balance/:address", get(get_wallet_balance))
        .route("/api/blockchain/transaction", post(create_transaction))
        
        .layer(CorsLayer::permissive())
        .with_state(state);

    // Start the server
    let listener = TcpListener::bind("127.0.0.1:3000").await?;
    info!("Edunet server running on http://127.0.0.1:3000");
    
    axum::serve(listener, app).await?;
    
    Ok(())
}

// Page handlers
async fn dashboard_page() -> impl IntoResponse {
    Html(include_str!("../templates/dashboard.html"))
}

async fn marketplace_page() -> impl IntoResponse {
    Html(include_str!("../templates/marketplace.html"))
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

// API handlers
async fn get_students(State(_state): State<AppState>) -> impl IntoResponse {
    // Mock data for demo
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
        }
    ];
    Json(ApiResponse::success(students))
}

async fn create_student(
    State(_state): State<AppState>,
    Json(student): Json<Student>,
) -> impl IntoResponse {
    // Mock implementation - in production this would insert into database
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
    // Mock implementation
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

async fn get_market_items(State(_state): State<AppState>) -> impl IntoResponse {
    // Mock data for demo
    let items = vec![
        MarketItem {
            id: Uuid::new_v4(),
            seller_id: Uuid::new_v4(),
            title: "Advanced Calculus Textbook".to_string(),
            description: "Excellent condition textbook".to_string(),
            category: "textbooks".to_string(),
            price: 85.0,
            currency: "EDU".to_string(),
            item_type: "physical".to_string(),
            status: "active".to_string(),
            images: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    ];
    Json(ApiResponse::success(items))
}

async fn create_market_item(
    State(_state): State<AppState>,
    Json(_item): Json<MarketItem>,
) -> impl IntoResponse {
    // Mock implementation
    let id = Uuid::new_v4();
    Json(ApiResponse::success(id))
}

async fn get_market_item(
    State(_state): State<AppState>,
    Path(_id): Path<Uuid>,
) -> impl IntoResponse {
    // Mock implementation
    let item = MarketItem {
        id: Uuid::new_v4(),
        seller_id: Uuid::new_v4(),
        title: "Advanced Calculus Textbook".to_string(),
        description: "Excellent condition textbook".to_string(),
        category: "textbooks".to_string(),
        price: 85.0,
        currency: "EDU".to_string(),
        item_type: "physical".to_string(),
        status: "active".to_string(),
        images: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    Json(ApiResponse::success(item))
}

async fn get_loans(State(state): State<AppState>) -> impl IntoResponse {
    match sqlx::query_as::<_, LoanApplication>("SELECT * FROM loan_applications ORDER BY created_at DESC")
        .fetch_all(&state.db)
        .await
    {
        Ok(loans) => Json(ApiResponse::success(loans)),
        Err(e) => {
            error!("Failed to fetch loans: {}", e);
            Json(ApiResponse::<Vec<LoanApplication>>::error("Failed to fetch loans".to_string()))
        }
    }
}

async fn create_loan(
    State(state): State<AppState>,
    Json(loan): Json<LoanApplication>,
) -> impl IntoResponse {
    let id = Uuid::new_v4();
    let now = Utc::now();
    
    match sqlx::query!(
        r#"
        INSERT INTO loan_applications (id, student_id, amount_requested, purpose, 
                                     proof_of_potential_score, academic_data, status, 
                                     interest_rate, term_months, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
        id,
        loan.student_id,
        loan.amount_requested,
        loan.purpose,
        loan.proof_of_potential_score,
        loan.academic_data,
        "pending",
        loan.interest_rate,
        loan.term_months,
        now,
        now
    )
    .execute(&state.db)
    .await
    {
        Ok(_) => Json(ApiResponse::success(id)),
        Err(e) => {
            error!("Failed to create loan application: {}", e);
            Json(ApiResponse::<Uuid>::error("Failed to create loan application".to_string()))
        }
    }
}

async fn get_loan(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match sqlx::query_as::<_, LoanApplication>("SELECT * FROM loan_applications WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db)
        .await
    {
        Ok(Some(loan)) => Json(ApiResponse::success(loan)),
        Ok(None) => Json(ApiResponse::<LoanApplication>::error("Loan not found".to_string())),
        Err(e) => {
            error!("Failed to fetch loan: {}", e);
            Json(ApiResponse::<LoanApplication>::error("Failed to fetch loan".to_string()))
        }
    }
}

async fn update_loan(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(loan): Json<LoanApplication>,
) -> impl IntoResponse {
    let now = Utc::now();
    
    match sqlx::query!(
        r#"
        UPDATE loan_applications 
        SET amount_requested = ?, purpose = ?, proof_of_potential_score = ?, 
            academic_data = ?, status = ?, interest_rate = ?, term_months = ?, updated_at = ?
        WHERE id = ?
        "#,
        loan.amount_requested,
        loan.purpose,
        loan.proof_of_potential_score,
        loan.academic_data,
        loan.status,
        loan.interest_rate,
        loan.term_months,
        now,
        id
    )
    .execute(&state.db)
    .await
    {
        Ok(result) => {
            if result.rows_affected() > 0 {
                Json(ApiResponse::success("Loan updated successfully"))
            } else {
                Json(ApiResponse::<String>::error("Loan not found".to_string()))
            }
        }
        Err(e) => {
            error!("Failed to update loan: {}", e);
            Json(ApiResponse::<String>::error("Failed to update loan".to_string()))
        }
    }
}

async fn get_nfts(State(state): State<AppState>) -> impl IntoResponse {
    match sqlx::query_as::<_, NFT>("SELECT * FROM nfts ORDER BY created_at DESC")
        .fetch_all(&state.db)
        .await
    {
        Ok(nfts) => Json(ApiResponse::success(nfts)),
        Err(e) => {
            error!("Failed to fetch NFTs: {}", e);
            Json(ApiResponse::<Vec<NFT>>::error("Failed to fetch NFTs".to_string()))
        }
    }
}

async fn create_nft(
    State(state): State<AppState>,
    Json(nft): Json<NFT>,
) -> impl IntoResponse {
    let id = Uuid::new_v4();
    let now = Utc::now();
    
    match sqlx::query!(
        r#"
        INSERT INTO nfts (id, creator_id, title, description, content_type, content_url,
                         metadata, token_id, minted, price, royalty_percentage, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
        id,
        nft.creator_id,
        nft.title,
        nft.description,
        nft.content_type,
        nft.content_url,
        nft.metadata,
        nft.token_id,
        false,
        nft.price,
        nft.royalty_percentage,
        now,
        now
    )
    .execute(&state.db)
    .await
    {
        Ok(_) => Json(ApiResponse::success(id)),
        Err(e) => {
            error!("Failed to create NFT: {}", e);
            Json(ApiResponse::<Uuid>::error("Failed to create NFT".to_string()))
        }
    }
}

async fn get_nft(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match sqlx::query_as::<_, NFT>("SELECT * FROM nfts WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db)
        .await
    {
        Ok(Some(nft)) => Json(ApiResponse::success(nft)),
        Ok(None) => Json(ApiResponse::<NFT>::error("NFT not found".to_string())),
        Err(e) => {
            error!("Failed to fetch NFT: {}", e);
            Json(ApiResponse::<NFT>::error("Failed to fetch NFT".to_string()))
        }
    }
}

async fn update_nft(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(nft): Json<NFT>,
) -> impl IntoResponse {
    let now = Utc::now();
    
    match sqlx::query!(
        r#"
        UPDATE nfts 
        SET title = ?, description = ?, content_type = ?, content_url = ?,
            metadata = ?, token_id = ?, minted = ?, price = ?, royalty_percentage = ?, updated_at = ?
        WHERE id = ?
        "#,
        nft.title,
        nft.description,
        nft.content_type,
        nft.content_url,
        nft.metadata,
        nft.token_id,
        nft.minted,
        nft.price,
        nft.royalty_percentage,
        now,
        id
    )
    .execute(&state.db)
    .await
    {
        Ok(result) => {
            if result.rows_affected() > 0 {
                Json(ApiResponse::success("NFT updated successfully"))
            } else {
                Json(ApiResponse::<String>::error("NFT not found".to_string()))
            }
        }
        Err(e) => {
            error!("Failed to update NFT: {}", e);
            Json(ApiResponse::<String>::error("Failed to update NFT".to_string()))
        }
    }
}

async fn get_projects(State(state): State<AppState>) -> impl IntoResponse {
    match sqlx::query_as::<_, InvestmentProject>("SELECT * FROM investment_projects WHERE status = 'active' ORDER BY created_at DESC")
        .fetch_all(&state.db)
        .await
    {
        Ok(projects) => Json(ApiResponse::success(projects)),
        Err(e) => {
            error!("Failed to fetch projects: {}", e);
            Json(ApiResponse::<Vec<InvestmentProject>>::error("Failed to fetch projects".to_string()))
        }
    }
}

async fn create_project(
    State(state): State<AppState>,
    Json(project): Json<InvestmentProject>,
) -> impl IntoResponse {
    let id = Uuid::new_v4();
    let now = Utc::now();
    
    match sqlx::query!(
        r#"
        INSERT INTO investment_projects (id, creator_id, title, description, project_type,
                                       funding_goal, current_funding, deadline, status, documents, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
        id,
        project.creator_id,
        project.title,
        project.description,
        project.project_type,
        project.funding_goal,
        0.0,
        project.deadline,
        "active",
        project.documents,
        now,
        now
    )
    .execute(&state.db)
    .await
    {
        Ok(_) => Json(ApiResponse::success(id)),
        Err(e) => {
            error!("Failed to create project: {}", e);
            Json(ApiResponse::<Uuid>::error("Failed to create project".to_string()))
        }
    }
}

async fn get_project(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match sqlx::query_as::<_, InvestmentProject>("SELECT * FROM investment_projects WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db)
        .await
    {
        Ok(Some(project)) => Json(ApiResponse::success(project)),
        Ok(None) => Json(ApiResponse::<InvestmentProject>::error("Project not found".to_string())),
        Err(e) => {
            error!("Failed to fetch project: {}", e);
            Json(ApiResponse::<InvestmentProject>::error("Failed to fetch project".to_string()))
        }
    }
}

async fn update_project(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(project): Json<InvestmentProject>,
) -> impl IntoResponse {
    let now = Utc::now();
    
    match sqlx::query!(
        r#"
        UPDATE investment_projects 
        SET title = ?, description = ?, project_type = ?, funding_goal = ?,
            current_funding = ?, deadline = ?, status = ?, documents = ?, updated_at = ?
        WHERE id = ?
        "#,
        project.title,
        project.description,
        project.project_type,
        project.funding_goal,
        project.current_funding,
        project.deadline,
        project.status,
        project.documents,
        now,
        id
    )
    .execute(&state.db)
    .await
    {
        Ok(result) => {
            if result.rows_affected() > 0 {
                Json(ApiResponse::success("Project updated successfully"))
            } else {
                Json(ApiResponse::<String>::error("Project not found".to_string()))
            }
        }
        Err(e) => {
            error!("Failed to update project: {}", e);
            Json(ApiResponse::<String>::error("Failed to update project".to_string()))
        }
    }
}

async fn get_dashboard_stats(State(state): State<AppState>) -> impl IntoResponse {
    // Execute multiple queries concurrently
    let (students_result, listings_result, loans_result, nfts_result, projects_result) = tokio::join!(
        sqlx::query!("SELECT COUNT(*) as count FROM students").fetch_one(&state.db),
        sqlx::query!("SELECT COUNT(*) as count FROM market_items WHERE status = 'active'").fetch_one(&state.db),
        sqlx::query!("SELECT COUNT(*) as count FROM loan_applications").fetch_one(&state.db),
        sqlx::query!("SELECT COUNT(*) as count FROM nfts WHERE minted = 1").fetch_one(&state.db),
        sqlx::query!("SELECT COUNT(*) as count FROM investment_projects WHERE status = 'funded'").fetch_one(&state.db)
    );

    match (students_result, listings_result, loans_result, nfts_result, projects_result) {
        (Ok(students), Ok(listings), Ok(loans), Ok(nfts), Ok(projects)) => {
            let stats = DashboardStats {
                total_students: students.count,
                active_listings: listings.count,
                total_loans: loans.count,
                minted_nfts: nfts.count,
                funded_projects: projects.count,
                total_volume: 0.0, // TODO: Calculate from transactions
            };
            Json(ApiResponse::success(stats))
        }
        _ => {
            error!("Failed to fetch dashboard statistics");
            Json(ApiResponse::<DashboardStats>::error("Failed to fetch statistics".to_string()))
        }
    }
}

async fn get_wallet_balance(
    State(state): State<AppState>,
    Path(address): Path<String>,
) -> impl IntoResponse {
    // TODO: Integrate with blockchain to get actual balance
    // For demo purposes, return a mock balance
    let balance = 127.5;
    Json(ApiResponse::success(balance))
}

async fn create_transaction(
    State(state): State<AppState>,
    Json(tx_data): Json<serde_json::Value>,
) -> impl IntoResponse {
    // TODO: Integrate with blockchain to create actual transaction
    // For demo purposes, return success
    info!("Creating transaction: {:?}", tx_data);
    Json(ApiResponse::success("Transaction created successfully"))
}