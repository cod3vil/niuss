use axum::{
    extract::{State, Path},
    http::{StatusCode, HeaderMap},
    response::{IntoResponse, Response},
    routing::{get, post, put, delete},
    Json, Router,
};
use redis::aio::ConnectionManager;
use serde_json::json;
use sqlx::PgPool;
use tower_http::cors::{CorsLayer, Any};
use std::sync::Arc;
use std::time::Duration;

use crate::cache::RedisCache;
use crate::config::Config;
use crate::db;
use crate::models::{AuthResponse, LoginRequest, RegisterRequest, CoinTransaction, User};
use crate::utils::{
    generate_referral_code, generate_token, hash_password,
    validate_email, validate_password, verify_password, verify_token,
};

// Import traffic module
use crate::traffic;

// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub db_pool: PgPool,
    pub redis_cache: RedisCache,
    pub config: Arc<Config>,
}

// Custom error type for API responses
pub enum ApiError {
    BadRequest(String),
    Unauthorized(String),
    NotFound(String),
    Conflict(String),
    InternalServerError(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            ApiError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg),
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            ApiError::Conflict(msg) => (StatusCode::CONFLICT, msg),
            ApiError::InternalServerError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        let body = Json(json!({
            "error": {
                "code": format!("{:?}", status),
                "message": error_message,
            }
        }));

        (status, body).into_response()
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> Self {
        tracing::error!("Database error: {:?}", err);
        ApiError::InternalServerError("Database error occurred".to_string())
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        tracing::error!("Internal error: {:?}", err);
        ApiError::InternalServerError(err.to_string())
    }
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Extract client IP address from request headers
/// 
/// Checks X-Forwarded-For header first (for proxied requests), then falls back to X-Real-IP.
/// Returns None if no IP address can be extracted.
fn extract_client_ip(headers: &HeaderMap) -> Option<String> {
    // Check X-Forwarded-For header first (for proxied requests)
    if let Some(forwarded) = headers.get("x-forwarded-for") {
        if let Ok(forwarded_str) = forwarded.to_str() {
            // Take the first IP in the comma-separated chain
            if let Some(first_ip) = forwarded_str.split(',').next() {
                let trimmed = first_ip.trim();
                if !trimmed.is_empty() {
                    return Some(trimmed.to_string());
                }
            }
        }
    }
    
    // Check X-Real-IP header as fallback
    if let Some(real_ip) = headers.get("x-real-ip") {
        if let Ok(ip_str) = real_ip.to_str() {
            let trimmed = ip_str.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    
    None
}

/// Async helper to log access without blocking the response
/// 
/// This function spawns an async task to log subscription access attempts to the database.
/// It is designed to be non-blocking and resilient - logging failures will not affect
/// the main request flow.
async fn log_access_async(
    state: &AppState,
    user_id: i64,
    token: &str,
    ip_address: &str,
    user_agent: Option<&str>,
    status: &str,
) {
    // Clone necessary data for async task
    let pool = state.db_pool.clone();
    let token = token.to_string();
    let ip = ip_address.to_string();
    let ua = user_agent.map(|s| s.to_string());
    let status = status.to_string();
    
    // Spawn async task to avoid blocking
    tokio::spawn(async move {
        if let Err(e) = db::create_access_log(
            &pool,
            user_id,
            &token,
            &ip,
            ua.as_deref(),
            &status,
        ).await {
            tracing::error!("Failed to log access: {:?}", e);
        }
    });
}

// ============================================================================
// Router Configuration
// ============================================================================

pub fn create_router(
    db_pool: PgPool,
    redis_conn: ConnectionManager,
    config: Config,
) -> Router {
    let redis_cache = RedisCache::new(redis_conn.clone());
    
    let state = AppState {
        db_pool,
        redis_cache,
        config: Arc::new(config.clone()),
    };

    // Configure CORS with specific allowed origins
    let cors = if config.cors_origins.contains(&"*".to_string()) {
        // Allow all origins if "*" is specified
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any)
            .max_age(Duration::from_secs(3600))
    } else {
        // Parse allowed origins
        let allowed_origins: Vec<_> = config.cors_origins
            .iter()
            .filter_map(|origin| origin.parse::<axum::http::HeaderValue>().ok())
            .collect();
        
        CorsLayer::new()
            .allow_origin(allowed_origins)
            .allow_methods([
                axum::http::Method::GET,
                axum::http::Method::POST,
                axum::http::Method::PUT,
                axum::http::Method::DELETE,
                axum::http::Method::OPTIONS,
            ])
            .allow_headers([
                axum::http::header::AUTHORIZATION,
                axum::http::header::CONTENT_TYPE,
                axum::http::header::ACCEPT,
            ])
            .max_age(Duration::from_secs(3600))
    };

    Router::new()
        .route("/health", get(health_check))
        .route("/api/auth/register", post(register_handler))
        .route("/api/auth/login", post(login_handler))
        .route("/api/auth/refresh", post(refresh_handler))
        .route("/api/user/balance", get(get_balance_handler))
        .route("/api/packages", get(get_packages_handler))
        .route("/api/packages/:id/purchase", post(purchase_package_handler))
        .route("/api/orders", get(get_orders_handler))
        .route("/api/orders/:id", get(get_order_by_id_handler))
        .route("/api/user/referral", get(get_referral_handler))
        .route("/api/user/referral/stats", get(get_referral_stats_handler))
        .route("/api/user/traffic", get(get_user_traffic_handler))
        .route("/api/subscription/link", get(get_subscription_link_handler))
        .route("/sub/:token", get(get_subscription_config_handler))
        // Admin node management endpoints
        .route("/api/admin/nodes", get(admin_list_nodes_handler))
        .route("/api/admin/nodes", post(admin_create_node_handler))
        .route("/api/admin/nodes/:id", put(admin_update_node_handler))
        .route("/api/admin/nodes/:id", delete(admin_delete_node_handler))
        // Node agent endpoints
        .route("/api/node/config", get(node_get_config_handler))
        .route("/api/node/heartbeat", post(node_heartbeat_handler))
        // Admin user management endpoints
        .route("/api/admin/users", get(admin_list_users_handler))
        .route("/api/admin/users/:id", get(admin_get_user_handler))
        .route("/api/admin/users/:id/status", put(admin_update_user_status_handler))
        .route("/api/admin/users/:id/balance", put(admin_update_user_balance_handler))
        .route("/api/admin/users/:id/traffic", put(admin_update_user_traffic_handler))
        // Admin order management endpoints
        .route("/api/admin/orders", get(admin_list_orders_handler))
        .route("/api/admin/orders/:id", get(admin_get_order_handler))
        // Admin statistics endpoints
        .route("/api/admin/stats/overview", get(admin_stats_overview_handler))
        .route("/api/admin/stats/revenue", get(admin_stats_revenue_handler))
        .route("/api/admin/stats/traffic", get(admin_stats_traffic_handler))
        // Admin Clash configuration endpoints
        // Note: Clash proxy management endpoints have been removed as part of node-proxy unification
        // Proxies are now managed through the /api/admin/nodes endpoints
        .route("/api/admin/clash/proxy-groups", get(admin_list_clash_proxy_groups_handler))
        .route("/api/admin/clash/proxy-groups", post(admin_create_clash_proxy_group_handler))
        .route("/api/admin/clash/proxy-groups/:id", put(admin_update_clash_proxy_group_handler))
        .route("/api/admin/clash/proxy-groups/:id", delete(admin_delete_clash_proxy_group_handler))
        .route("/api/admin/clash/rules", get(admin_list_clash_rules_handler))
        .route("/api/admin/clash/rules", post(admin_create_clash_rule_handler))
        .route("/api/admin/clash/rules/:id", put(admin_update_clash_rule_handler))
        .route("/api/admin/clash/rules/:id", delete(admin_delete_clash_rule_handler))
        .route("/api/admin/clash/generate", get(admin_generate_clash_config_handler))
        // Admin access logs endpoints
        .route("/api/admin/access-logs", get(admin_query_access_logs_handler))
        .layer(cors)
        .with_state(state)
}

async fn health_check() -> &'static str {
    "OK"
}

// ============================================================================
// Authentication Handlers
// ============================================================================

/// POST /api/auth/register - Register a new user
async fn register_handler(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> Result<Json<AuthResponse>, ApiError> {
    // Validate email format
    validate_email(&payload.email)
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    // Validate password strength
    validate_password(&payload.password)
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    // Check if email already exists
    if let Some(_) = db::get_user_by_email(&state.db_pool, &payload.email).await? {
        return Err(ApiError::Conflict("Email already exists".to_string()));
    }

    // Check referral code if provided
    let referred_by = if let Some(ref code) = payload.referral_code {
        if let Some(referrer) = db::get_user_by_referral_code(&state.db_pool, code).await? {
            Some(referrer.id)
        } else {
            return Err(ApiError::BadRequest("Invalid referral code".to_string()));
        }
    } else {
        None
    };

    // Hash password
    let password_hash = hash_password(&payload.password)
        .map_err(|e| ApiError::InternalServerError(e.to_string()))?;

    // Generate unique referral code
    let mut referral_code = generate_referral_code();
    // Ensure uniqueness (retry if collision)
    while db::get_user_by_referral_code(&state.db_pool, &referral_code)
        .await?
        .is_some()
    {
        referral_code = generate_referral_code();
    }

    // Create user
    let user = db::create_user(
        &state.db_pool,
        &payload.email,
        &password_hash,
        Some(&referral_code),
        referred_by,
    )
    .await?;

    // Generate JWT token
    let token = generate_token(
        user.id,
        &user.email,
        user.is_admin,
        &state.config.jwt_secret,
        state.config.jwt_expiration,
    )
    .map_err(|e| ApiError::InternalServerError(e.to_string()))?;

    Ok(Json(AuthResponse {
        token,
        user: user.into(),
    }))
}

/// POST /api/auth/login - Login with email and password
async fn login_handler(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, ApiError> {
    // Validate email format
    validate_email(&payload.email)
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    // Get user by email
    let user = db::get_user_by_email(&state.db_pool, &payload.email)
        .await?
        .ok_or_else(|| ApiError::Unauthorized("Invalid credentials".to_string()))?;

    // Check if user is disabled
    if user.status == "disabled" {
        return Err(ApiError::Unauthorized("Account is disabled".to_string()));
    }

    // Verify password
    let is_valid = verify_password(&payload.password, &user.password_hash)
        .map_err(|e| ApiError::InternalServerError(e.to_string()))?;

    if !is_valid {
        return Err(ApiError::Unauthorized("Invalid credentials".to_string()));
    }

    // Generate JWT token
    let token = generate_token(
        user.id,
        &user.email,
        user.is_admin,
        &state.config.jwt_secret,
        state.config.jwt_expiration,
    )
    .map_err(|e| ApiError::InternalServerError(e.to_string()))?;

    Ok(Json(AuthResponse {
        token,
        user: user.into(),
    }))
}

/// POST /api/auth/refresh - Refresh JWT token
async fn refresh_handler(
    State(state): State<AppState>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Extract token from request body
    let old_token = payload
        .get("token")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ApiError::BadRequest("Token is required".to_string()))?;

    // Verify old token
    let claims = verify_token(old_token, &state.config.jwt_secret)
        .map_err(|_| ApiError::Unauthorized("Invalid or expired token".to_string()))?;

    // Get user to ensure they still exist and are active
    let user = db::get_user_by_id(&state.db_pool, claims.sub)
        .await?
        .ok_or_else(|| ApiError::Unauthorized("User not found".to_string()))?;

    if user.status == "disabled" {
        return Err(ApiError::Unauthorized("Account is disabled".to_string()));
    }

    // Generate new token
    let new_token = generate_token(
        user.id,
        &user.email,
        user.is_admin,
        &state.config.jwt_secret,
        state.config.jwt_expiration,
    )
    .map_err(|e| ApiError::InternalServerError(e.to_string()))?;

    Ok(Json(json!({
        "token": new_token,
    })))
}

// ============================================================================
// Coin Balance Management
// ============================================================================

/// Add coins to user balance (recharge)
pub async fn add_coins(
    pool: &PgPool,
    user_id: i64,
    amount: i64,
    description: Option<&str>,
) -> Result<(User, CoinTransaction), ApiError> {
    if amount <= 0 {
        return Err(ApiError::BadRequest("Amount must be positive".to_string()));
    }

    // Start a transaction
    let mut tx = pool.begin().await?;

    // Get current user
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1 FOR UPDATE")
        .bind(user_id)
        .fetch_one(&mut *tx)
        .await?;

    // Calculate new balance
    let new_balance = user.coin_balance + amount;

    // Update user balance
    let updated_user = sqlx::query_as::<_, User>(
        r#"
        UPDATE users
        SET coin_balance = $2, updated_at = NOW()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(new_balance)
    .fetch_one(&mut *tx)
    .await?;

    // Create transaction record
    let transaction = sqlx::query_as::<_, CoinTransaction>(
        r#"
        INSERT INTO coin_transactions (user_id, amount, type, description)
        VALUES ($1, $2, 'recharge', $3)
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(amount)
    .bind(description)
    .fetch_one(&mut *tx)
    .await?;

    // Commit transaction
    tx.commit().await?;

    Ok((updated_user, transaction))
}

/// Deduct coins from user balance (purchase)
pub async fn deduct_coins(
    pool: &PgPool,
    user_id: i64,
    amount: i64,
    description: Option<&str>,
) -> Result<(User, CoinTransaction), ApiError> {
    if amount <= 0 {
        return Err(ApiError::BadRequest("Amount must be positive".to_string()));
    }

    // Start a transaction
    let mut tx = pool.begin().await?;

    // Get current user with row lock
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1 FOR UPDATE")
        .bind(user_id)
        .fetch_one(&mut *tx)
        .await?;

    // Check if user has sufficient balance
    if user.coin_balance < amount {
        return Err(ApiError::BadRequest("Insufficient balance".to_string()));
    }

    // Calculate new balance
    let new_balance = user.coin_balance - amount;

    // Update user balance
    let updated_user = sqlx::query_as::<_, User>(
        r#"
        UPDATE users
        SET coin_balance = $2, updated_at = NOW()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(new_balance)
    .fetch_one(&mut *tx)
    .await?;

    // Create transaction record (negative amount for deduction)
    let transaction = sqlx::query_as::<_, CoinTransaction>(
        r#"
        INSERT INTO coin_transactions (user_id, amount, type, description)
        VALUES ($1, $2, 'purchase', $3)
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(-amount) // Store as negative for deductions
    .bind(description)
    .fetch_one(&mut *tx)
    .await?;

    // Commit transaction
    tx.commit().await?;

    Ok((updated_user, transaction))
}

/// GET /api/user/balance - Get user coin balance
async fn get_balance_handler(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Extract and verify JWT token from Authorization header
    let token = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| ApiError::Unauthorized("Missing or invalid authorization header".to_string()))?;

    let claims = verify_token(token, &state.config.jwt_secret)
        .map_err(|_| ApiError::Unauthorized("Invalid or expired token".to_string()))?;

    // Get user from database
    let user = db::get_user_by_id(&state.db_pool, claims.sub)
        .await?
        .ok_or_else(|| ApiError::NotFound("User not found".to_string()))?;

    // Check if user is active
    if user.status == "disabled" {
        return Err(ApiError::Unauthorized("Account is disabled".to_string()));
    }

    // Get recent transactions (last 10)
    let transactions = sqlx::query_as::<_, CoinTransaction>(
        r#"
        SELECT * FROM coin_transactions
        WHERE user_id = $1
        ORDER BY created_at DESC
        LIMIT 10
        "#,
    )
    .bind(user.id)
    .fetch_all(&state.db_pool)
    .await?;

    Ok(Json(json!({
        "coin_balance": user.coin_balance,
        "recent_transactions": transactions,
    })))
}

// ============================================================================
// Package Management
// ============================================================================

/// Get user package data with caching
async fn get_user_package_with_cache(
    state: &AppState,
    user_id: i64,
) -> Result<Option<crate::cache::UserPackageCache>, ApiError> {
    // Try to get from cache first
    if let Ok(Some(cached)) = state.redis_cache.get_user_package(user_id).await {
        tracing::debug!("User package cache hit for user {}", user_id);
        return Ok(Some(cached));
    }

    tracing::debug!("User package cache miss for user {}", user_id);

    // Cache miss - get from database
    let _user = db::get_user_by_id(&state.db_pool, user_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("User not found".to_string()))?;

    // Get active user packages
    let user_packages = sqlx::query_as::<_, crate::models::UserPackage>(
        r#"
        SELECT * FROM user_packages
        WHERE user_id = $1 AND status = 'active'
        ORDER BY expires_at DESC
        LIMIT 1
        "#,
    )
    .bind(user_id)
    .fetch_optional(&state.db_pool)
    .await?;

    if let Some(pkg) = user_packages {
        let cache_data = crate::cache::UserPackageCache {
            traffic_quota: pkg.traffic_quota,
            traffic_used: pkg.traffic_used,
            expires_at: pkg.expires_at.to_rfc3339(),
            status: pkg.status,
        };

        // Cache the data
        if let Err(e) = state.redis_cache.cache_user_package(user_id, &cache_data).await {
            tracing::warn!("Failed to cache user package: {}", e);
            // Don't fail the request if caching fails
        }

        Ok(Some(cache_data))
    } else {
        Ok(None)
    }
}

/// GET /api/packages - Get list of available packages
async fn get_packages_handler(
    State(state): State<AppState>,
) -> Result<Json<Vec<crate::models::Package>>, ApiError> {
    // Get all active packages
    let packages = db::list_active_packages(&state.db_pool).await?;
    
    Ok(Json(packages))
}

/// Get active nodes with caching
async fn get_active_nodes_with_cache(
    state: &AppState,
) -> Result<Vec<crate::models::Node>, ApiError> {
    // Try to get from cache first
    if let Ok(Some(cached)) = state.redis_cache.get_active_nodes().await {
        tracing::debug!("Active nodes cache hit");
        return Ok(cached);
    }

    tracing::debug!("Active nodes cache miss");

    // Cache miss - get from database
    let nodes = db::list_nodes_by_status(&state.db_pool, "online").await?;

    // Cache the data
    if let Err(e) = state.redis_cache.cache_active_nodes(&nodes).await {
        tracing::warn!("Failed to cache active nodes: {}", e);
        // Don't fail the request if caching fails
    }

    Ok(nodes)
}

/// POST /api/packages/:id/purchase - Purchase a package
async fn purchase_package_handler(
    State(state): State<AppState>,
    Path(package_id): Path<i64>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Extract and verify JWT token
    let token = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| ApiError::Unauthorized("Missing or invalid authorization header".to_string()))?;

    let claims = verify_token(token, &state.config.jwt_secret)
        .map_err(|_| ApiError::Unauthorized("Invalid or expired token".to_string()))?;

    let user_id = claims.sub;

    // Get package details
    let package = db::get_package_by_id(&state.db_pool, package_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Package not found".to_string()))?;

    // Check if package is active
    if !package.is_active {
        return Err(ApiError::BadRequest("Package is not available".to_string()));
    }

    // Start a database transaction
    let mut tx = state.db_pool.begin().await
        .map_err(|e| ApiError::InternalServerError(format!("Transaction error: {}", e)))?;

    // Get user with row lock
    let user = sqlx::query_as::<_, crate::models::User>(
        "SELECT * FROM users WHERE id = $1 FOR UPDATE"
    )
    .bind(user_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| ApiError::NotFound("User not found".to_string()))?;

    // Check if user is active
    if user.status == "disabled" {
        return Err(ApiError::Unauthorized("Account is disabled".to_string()));
    }

    // Verify coin balance
    if user.coin_balance < package.price {
        return Err(ApiError::BadRequest("Insufficient balance".to_string()));
    }

    // Generate unique order number
    let order_no = format!("ORD-{}-{}", user_id, chrono::Utc::now().timestamp_millis());

    // Create order record
    let order = sqlx::query_as::<_, crate::models::Order>(
        r#"
        INSERT INTO orders (order_no, user_id, package_id, amount, status)
        VALUES ($1, $2, $3, $4, 'pending')
        RETURNING *
        "#,
    )
    .bind(&order_no)
    .bind(user_id)
    .bind(package_id)
    .bind(package.price)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| ApiError::InternalServerError(format!("Failed to create order: {}", e)))?;

    // Deduct coins from user balance
    let new_balance = user.coin_balance - package.price;
    sqlx::query(
        r#"
        UPDATE users
        SET coin_balance = $2, updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(user_id)
    .bind(new_balance)
    .execute(&mut *tx)
    .await
    .map_err(|e| ApiError::InternalServerError(format!("Failed to update balance: {}", e)))?;

    // Create coin transaction record (negative amount for deduction)
    sqlx::query(
        r#"
        INSERT INTO coin_transactions (user_id, amount, type, description)
        VALUES ($1, $2, 'purchase', $3)
        "#,
    )
    .bind(user_id)
    .bind(-package.price)
    .bind(format!("Purchase package: {}", package.name))
    .execute(&mut *tx)
    .await
    .map_err(|e| ApiError::InternalServerError(format!("Failed to create transaction: {}", e)))?;

    // Increase user traffic quota
    let new_traffic_quota = user.traffic_quota + package.traffic_amount;
    sqlx::query(
        r#"
        UPDATE users
        SET traffic_quota = $2, updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(user_id)
    .bind(new_traffic_quota)
    .execute(&mut *tx)
    .await
    .map_err(|e| ApiError::InternalServerError(format!("Failed to update traffic quota: {}", e)))?;

    // Calculate expiration date
    let expires_at = chrono::Utc::now() + chrono::Duration::days(package.duration_days as i64);

    // Create user_packages record
    let _user_package = sqlx::query_as::<_, crate::models::UserPackage>(
        r#"
        INSERT INTO user_packages (user_id, package_id, order_id, traffic_quota, expires_at)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(package_id)
    .bind(order.id)
    .bind(package.traffic_amount)
    .bind(expires_at)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| ApiError::InternalServerError(format!("Failed to create user package: {}", e)))?;

    // Update order status to completed
    sqlx::query(
        r#"
        UPDATE orders
        SET status = 'completed', completed_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(order.id)
    .execute(&mut *tx)
    .await
    .map_err(|e| ApiError::InternalServerError(format!("Failed to update order status: {}", e)))?;

    // Commit transaction
    tx.commit().await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to commit transaction: {}", e)))?;

    // Invalidate user package cache after successful purchase
    if let Err(e) = state.redis_cache.invalidate_user_package(user_id).await {
        tracing::warn!("Failed to invalidate user package cache: {}", e);
        // Don't fail the request if cache invalidation fails
    }

    // Process referral rebate if this is the user's first purchase
    // Default rebate is 10% of purchase amount
    let rebate_percentage = 0.10;
    if let Ok(Some(referrer)) = db::process_referral_rebate(
        &state.db_pool,
        user_id,
        package.price,
        rebate_percentage,
    ).await {
        tracing::info!(
            "Processed referral rebate: {} coins to user {} for referring user {}",
            (package.price as f64 * rebate_percentage) as i64,
            referrer.id,
            user_id
        );
    }

    Ok(Json(json!({
        "order_id": order.id,
        "order_no": order_no,
        "package_name": package.name,
        "traffic_added": package.traffic_amount,
        "new_balance": new_balance,
        "new_traffic_quota": new_traffic_quota,
        "expires_at": expires_at,
        "message": "Package purchased successfully"
    })))
}

// ============================================================================
// Order Management
// ============================================================================

/// GET /api/orders - Get user's order list
async fn get_orders_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Extract and verify JWT token
    let token = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| ApiError::Unauthorized("Missing or invalid authorization header".to_string()))?;

    let claims = verify_token(token, &state.config.jwt_secret)
        .map_err(|_| ApiError::Unauthorized("Invalid or expired token".to_string()))?;

    let user_id = claims.sub;

    // Get user's orders with pagination (default: 50 orders)
    let orders = db::list_orders_by_user(&state.db_pool, user_id, 50, 0).await?;

    // Get package details for each order
    let mut orders_with_details = Vec::new();
    for order in orders {
        let package = db::get_package_by_id(&state.db_pool, order.package_id).await?;
        
        orders_with_details.push(serde_json::json!({
            "id": order.id,
            "order_no": order.order_no,
            "package_id": order.package_id,
            "package_name": package.as_ref().map(|p| &p.name),
            "amount": order.amount,
            "status": order.status,
            "created_at": order.created_at,
            "completed_at": order.completed_at,
        }));
    }

    Ok(Json(serde_json::json!({
        "orders": orders_with_details,
        "total": orders_with_details.len(),
    })))
}

/// GET /api/orders/:id - Get order details by ID
async fn get_order_by_id_handler(
    State(state): State<AppState>,
    Path(order_id): Path<i64>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Extract and verify JWT token
    let token = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| ApiError::Unauthorized("Missing or invalid authorization header".to_string()))?;

    let claims = verify_token(token, &state.config.jwt_secret)
        .map_err(|_| ApiError::Unauthorized("Invalid or expired token".to_string()))?;

    let user_id = claims.sub;

    // Get order
    let order = db::get_order_by_id(&state.db_pool, order_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Order not found".to_string()))?;

    // Verify order belongs to user
    if order.user_id != user_id {
        return Err(ApiError::Unauthorized("Access denied".to_string()));
    }

    // Get package details
    let package = db::get_package_by_id(&state.db_pool, order.package_id).await?;

    // Get user package details if exists
    let user_package = sqlx::query_as::<_, crate::models::UserPackage>(
        r#"
        SELECT * FROM user_packages
        WHERE order_id = $1
        "#,
    )
    .bind(order_id)
    .fetch_optional(&state.db_pool)
    .await?;

    Ok(Json(serde_json::json!({
        "id": order.id,
        "order_no": order.order_no,
        "user_id": order.user_id,
        "package_id": order.package_id,
        "package_name": package.as_ref().map(|p| &p.name),
        "package_traffic": package.as_ref().map(|p| p.traffic_amount),
        "package_duration_days": package.as_ref().map(|p| p.duration_days),
        "amount": order.amount,
        "status": order.status,
        "created_at": order.created_at,
        "completed_at": order.completed_at,
        "user_package": user_package,
    })))
}

// ============================================================================
// Referral System
// ============================================================================

/// GET /api/user/referral - Get user's referral link and code
async fn get_referral_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Extract and verify JWT token
    let token = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| ApiError::Unauthorized("Missing or invalid authorization header".to_string()))?;

    let claims = verify_token(token, &state.config.jwt_secret)
        .map_err(|_| ApiError::Unauthorized("Invalid or expired token".to_string()))?;

    let user_id = claims.sub;

    // Get user from database
    let user = db::get_user_by_id(&state.db_pool, user_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("User not found".to_string()))?;

    // Check if user is active
    if user.status == "disabled" {
        return Err(ApiError::Unauthorized("Account is disabled".to_string()));
    }

    // Get referral code (should always exist after registration)
    let referral_code = user.referral_code
        .ok_or_else(|| ApiError::InternalServerError("Referral code not found".to_string()))?;

    // Construct referral link (assuming frontend base URL from config or environment)
    let base_url = std::env::var("FRONTEND_URL")
        .unwrap_or_else(|_| "http://localhost:3000".to_string());
    let referral_link = format!("{}/register?ref={}", base_url, referral_code);

    Ok(Json(json!({
        "referral_code": referral_code,
        "referral_link": referral_link,
    })))
}

/// GET /api/user/referral/stats - Get user's referral statistics
async fn get_referral_stats_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Extract and verify JWT token
    let token = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| ApiError::Unauthorized("Missing or invalid authorization header".to_string()))?;

    let claims = verify_token(token, &state.config.jwt_secret)
        .map_err(|_| ApiError::Unauthorized("Invalid or expired token".to_string()))?;

    let user_id = claims.sub;

    // Get user from database
    let user = db::get_user_by_id(&state.db_pool, user_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("User not found".to_string()))?;

    // Check if user is active
    if user.status == "disabled" {
        return Err(ApiError::Unauthorized("Account is disabled".to_string()));
    }

    // Get referral statistics
    let (referral_count, total_rebate) = db::get_referral_stats(&state.db_pool, user_id)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to get referral stats: {}", e)))?;

    // Get referral code (should always exist after registration)
    let referral_code = user.referral_code
        .ok_or_else(|| ApiError::InternalServerError("Referral code not found".to_string()))?;

    // Construct referral link
    let base_url = std::env::var("FRONTEND_URL")
        .unwrap_or_else(|_| "http://localhost:3000".to_string());
    let referral_link = format!("{}/register?ref={}", base_url, referral_code);

    Ok(Json(json!({
        "referral_count": referral_count,
        "total_commission": total_rebate,
        "referral_link": referral_link,
    })))
}

/// GET /api/user/traffic - Get user's traffic usage statistics
async fn get_user_traffic_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Extract and verify JWT token
    let token = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| ApiError::Unauthorized("Missing or invalid authorization header".to_string()))?;

    let claims = verify_token(token, &state.config.jwt_secret)
        .map_err(|_| ApiError::Unauthorized("Invalid or expired token".to_string()))?;

    let user_id = claims.sub;

    // Get user from database
    let user = db::get_user_by_id(&state.db_pool, user_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("User not found".to_string()))?;

    // Check if user is active
    if user.status == "disabled" {
        return Err(ApiError::Unauthorized("Account is disabled".to_string()));
    }

    // Get traffic statistics
    let stats = traffic::get_user_traffic_stats(&state.db_pool, user_id)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to get traffic stats: {}", e)))?;

    match stats {
        Some((quota, used, remaining)) => {
            // Calculate percentage used
            let percentage_used = if quota > 0 {
                (used as f64 / quota as f64 * 100.0).min(100.0)
            } else {
                0.0
            };

            Ok(Json(json!({
                "traffic_quota": quota,
                "traffic_used": used,
                "traffic_remaining": remaining,
                "percentage_used": percentage_used,
                "has_traffic": remaining > 0,
            })))
        }
        None => Err(ApiError::NotFound("User not found".to_string())),
    }
}

// ============================================================================
// Subscription Management
// ============================================================================

/// GET /api/subscription/link - Get user's subscription link
async fn get_subscription_link_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Extract and verify JWT token
    let token = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| ApiError::Unauthorized("Missing or invalid authorization header".to_string()))?;

    let claims = verify_token(token, &state.config.jwt_secret)
        .map_err(|_| ApiError::Unauthorized("Invalid or expired token".to_string()))?;

    let user_id = claims.sub;

    // Get user from database
    let user = db::get_user_by_id(&state.db_pool, user_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("User not found".to_string()))?;

    // Check if user is active
    if user.status == "disabled" {
        return Err(ApiError::Unauthorized("Account is disabled".to_string()));
    }

    // Check if subscription already exists
    let existing_subscription = sqlx::query_as::<_, crate::models::Subscription>(
        r#"
        SELECT * FROM subscriptions WHERE user_id = $1
        "#,
    )
    .bind(user_id)
    .fetch_optional(&state.db_pool)
    .await?;

    let subscription = if let Some(sub) = existing_subscription {
        sub
    } else {
        // Generate unique subscription token
        let mut token = crate::utils::generate_subscription_token();
        
        // Ensure uniqueness (retry if collision)
        while db::get_subscription_by_token(&state.db_pool, &token)
            .await?
            .is_some()
        {
            token = crate::utils::generate_subscription_token();
        }

        // Create subscription record
        db::create_subscription(&state.db_pool, user_id, &token).await?
    };

    // Construct subscription URL
    let base_url = std::env::var("API_BASE_URL")
        .unwrap_or_else(|_| "http://localhost:8080".to_string());
    let subscription_url = format!("{}/sub/{}", base_url, subscription.token);

    Ok(Json(json!({
        "token": subscription.token,
        "subscription_url": subscription_url,
        "created_at": subscription.created_at,
        "last_accessed": subscription.last_accessed,
    })))
}

/// GET /sub/:token - Get Clash subscription configuration (public endpoint)
async fn get_subscription_config_handler(
    State(state): State<AppState>,
    Path(token): Path<String>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    // Extract IP address from headers
    let ip_address = extract_client_ip(&headers)
        .unwrap_or_else(|| "unknown".to_string());
    
    // Extract User-Agent from headers
    let user_agent = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    
    // Try to get from cache first
    if let Ok(Some(cached_config)) = state.redis_cache.get_subscription_config(&token).await {
        tracing::debug!("Subscription config cache hit for token {}", token);
        
        // We need to get user_id for logging even with cache hit
        if let Ok(Some(subscription)) = db::get_subscription_by_token(&state.db_pool, &token).await {
            log_access_async(&state, subscription.user_id, &token, &ip_address, user_agent.as_deref(), "success").await;
        }
        
        return Ok((
            StatusCode::OK,
            [(axum::http::header::CONTENT_TYPE, "text/yaml; charset=utf-8")],
            cached_config,
        ));
    }

    tracing::debug!("Subscription config cache miss for token {}", token);

    // Get subscription from database
    let subscription = match db::get_subscription_by_token(&state.db_pool, &token).await {
        Ok(Some(sub)) => sub,
        Ok(None) => {
            // Log failed access attempt (subscription not found)
            // We don't have user_id, so we can't log this properly
            // This is a limitation - we'll skip logging for invalid tokens
            return Err(ApiError::NotFound("Subscription not found".to_string()));
        }
        Err(e) => return Err(e.into()),
    };

    let user_id = subscription.user_id;

    // Get user
    let user = match db::get_user_by_id(&state.db_pool, user_id).await {
        Ok(Some(u)) => u,
        Ok(None) => {
            log_access_async(&state, user_id, &token, &ip_address, user_agent.as_deref(), "failed").await;
            return Err(ApiError::NotFound("User not found".to_string()));
        }
        Err(e) => {
            log_access_async(&state, user_id, &token, &ip_address, user_agent.as_deref(), "failed").await;
            return Err(e.into());
        }
    };

    // Check if user is active
    if user.status == "disabled" {
        log_access_async(&state, user_id, &token, &ip_address, user_agent.as_deref(), "disabled").await;
        return Err(ApiError::Unauthorized("Account is disabled".to_string()));
    }

    // Check if user has exceeded traffic quota
    let has_traffic = traffic::check_traffic_quota(&state.db_pool, user.id)
        .await
        .unwrap_or(false);

    if !has_traffic {
        tracing::warn!("User {} has exceeded traffic quota", user.id);
        log_access_async(&state, user_id, &token, &ip_address, user_agent.as_deref(), "quota_exceeded").await;
        let empty_config = "proxies: []\nproxy-groups: []\nrules: []\n";
        return Ok((
            StatusCode::OK,
            [(axum::http::header::CONTENT_TYPE, "text/yaml; charset=utf-8")],
            empty_config.to_string(),
        ));
    }

    // Check if user has valid package
    let user_packages = sqlx::query_as::<_, crate::models::UserPackage>(
        r#"
        SELECT * FROM user_packages
        WHERE user_id = $1 AND status = 'active' AND expires_at > NOW()
        ORDER BY expires_at DESC
        LIMIT 1
        "#,
    )
    .bind(user.id)
    .fetch_optional(&state.db_pool)
    .await?;

    // If no valid package, return empty config
    if user_packages.is_none() {
        log_access_async(&state, user_id, &token, &ip_address, user_agent.as_deref(), "expired").await;
        let empty_config = "proxies: []\nproxy-groups: []\nrules: []\n";
        return Ok((
            StatusCode::OK,
            [(axum::http::header::CONTENT_TYPE, "text/yaml; charset=utf-8")],
            empty_config.to_string(),
        ));
    }

    let user_package = user_packages.unwrap();

    // Check if package traffic is exhausted
    if user_package.traffic_used >= user_package.traffic_quota {
        tracing::warn!("User {} package traffic exhausted", user.id);
        log_access_async(&state, user_id, &token, &ip_address, user_agent.as_deref(), "quota_exceeded").await;
        let empty_config = "proxies: []\nproxy-groups: []\nrules: []\n";
        return Ok((
            StatusCode::OK,
            [(axum::http::header::CONTENT_TYPE, "text/yaml; charset=utf-8")],
            empty_config.to_string(),
        ));
    }

    // Get active nodes
    let nodes = db::list_nodes_by_status(&state.db_pool, "online").await?;

    // Try to get Clash configuration from database first
    let proxies = db::list_clash_proxies(&state.db_pool, true).await.ok();
    let proxy_groups = db::list_clash_proxy_groups(&state.db_pool, true).await.ok();
    let rules = db::list_clash_rules(&state.db_pool, true).await.ok();

    // Generate Clash configuration
    let clash_config = if let (Some(p), Some(pg), Some(r)) = (proxies, proxy_groups, rules) {
        // Use database configuration if available
        if !p.is_empty() && !pg.is_empty() && !r.is_empty() {
            tracing::info!("Using database Clash configuration for user {}", user.id);
            crate::clash::generate_clash_config_from_db(&p, &pg, &r)
                .map_err(|e| ApiError::InternalServerError(format!("Failed to generate config: {}", e)))?
        } else {
            // Fall back to node-based configuration
            tracing::info!("Using node-based Clash configuration for user {}", user.id);
            crate::clash::generate_clash_config(&nodes)
                .map_err(|e| ApiError::InternalServerError(format!("Failed to generate config: {}", e)))?
        }
    } else {
        // Fall back to node-based configuration
        tracing::info!("Using node-based Clash configuration for user {}", user.id);
        crate::clash::generate_clash_config(&nodes)
            .map_err(|e| ApiError::InternalServerError(format!("Failed to generate config: {}", e)))?
    };

    // Cache the configuration
    if let Err(e) = state.redis_cache.cache_subscription_config(&token, &clash_config).await {
        tracing::warn!("Failed to cache subscription config: {}", e);
        // Don't fail the request if caching fails
    }

    // Update last_accessed timestamp
    let _ = sqlx::query(
        r#"
        UPDATE subscriptions
        SET last_accessed = NOW()
        WHERE token = $1
        "#,
    )
    .bind(&token)
    .execute(&state.db_pool)
    .await;

    // Log successful access
    log_access_async(&state, user_id, &token, &ip_address, user_agent.as_deref(), "success").await;

    Ok((
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "text/yaml; charset=utf-8")],
        clash_config,
    ))
}

// ============================================================================
// Admin Node Management Handlers
// ============================================================================

/// GET /api/admin/nodes - Get list of all nodes (admin only)
async fn admin_list_nodes_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<crate::models::Node>>, ApiError> {
    // Extract and verify JWT token
    let token = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| ApiError::Unauthorized("Missing or invalid authorization header".to_string()))?;

    let claims = verify_token(token, &state.config.jwt_secret)
        .map_err(|_| ApiError::Unauthorized("Invalid or expired token".to_string()))?;

    // Check if user is admin
    if !claims.is_admin {
        return Err(ApiError::Unauthorized("Admin access required".to_string()));
    }

    // Get all nodes
    let nodes = db::list_all_nodes(&state.db_pool).await?;

    Ok(Json(nodes))
}

/// POST /api/admin/nodes - Create a new node (admin only)
async fn admin_create_node_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<crate::models::CreateNodeRequest>,
) -> Result<Json<crate::models::Node>, ApiError> {
    // Extract and verify JWT token
    let token = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| ApiError::Unauthorized("Missing or invalid authorization header".to_string()))?;

    let claims = verify_token(token, &state.config.jwt_secret)
        .map_err(|_| ApiError::Unauthorized("Invalid or expired token".to_string()))?;

    // Check if user is admin
    if !claims.is_admin {
        return Err(ApiError::Unauthorized("Admin access required".to_string()));
    }

    // Validate port range
    if payload.port < 1 || payload.port > 65535 {
        return Err(ApiError::BadRequest("Port must be between 1 and 65535".to_string()));
    }

    // Validate protocol
    let valid_protocols = ["shadowsocks", "vmess", "trojan", "hysteria2", "vless"];
    if !valid_protocols.contains(&payload.protocol.as_str()) {
        return Err(ApiError::BadRequest(format!(
            "Invalid protocol. Must be one of: {}",
            valid_protocols.join(", ")
        )));
    }

    // Generate a secure secret for the node
    let secret = crate::utils::generate_node_secret();

    // Create node in database
    let node = db::create_node(
        &state.db_pool,
        &payload.name,
        &payload.host,
        payload.port,
        &payload.protocol,
        &secret,
        payload.config,
    )
    .await?;

    // Log admin action
    let _ = db::create_admin_log(
        &state.db_pool,
        claims.sub,
        "create_node",
        Some("node"),
        Some(node.id),
        Some(json!({
            "node_name": &payload.name,
            "host": &payload.host,
            "port": payload.port,
            "protocol": &payload.protocol,
        })),
    )
    .await;

    // Invalidate active nodes cache
    if let Err(e) = state.redis_cache.invalidate_active_nodes().await {
        tracing::warn!("Failed to invalidate active nodes cache: {}", e);
    }

    Ok(Json(node))
}

/// PUT /api/admin/nodes/:id - Update a node (admin only)
async fn admin_update_node_handler(
    State(state): State<AppState>,
    Path(node_id): Path<i64>,
    headers: HeaderMap,
    Json(payload): Json<crate::models::UpdateNodeRequest>,
) -> Result<Json<crate::models::Node>, ApiError> {
    // Extract and verify JWT token
    let token = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| ApiError::Unauthorized("Missing or invalid authorization header".to_string()))?;

    let claims = verify_token(token, &state.config.jwt_secret)
        .map_err(|_| ApiError::Unauthorized("Invalid or expired token".to_string()))?;

    // Check if user is admin
    if !claims.is_admin {
        return Err(ApiError::Unauthorized("Admin access required".to_string()));
    }

    // Check if node exists
    let _existing_node = db::get_node_by_id(&state.db_pool, node_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Node not found".to_string()))?;

    // Validate port if provided
    if let Some(port) = payload.port {
        if port < 1 || port > 65535 {
            return Err(ApiError::BadRequest("Port must be between 1 and 65535".to_string()));
        }
    }

    // Validate protocol if provided
    if let Some(ref protocol) = payload.protocol {
        let valid_protocols = ["shadowsocks", "vmess", "trojan", "hysteria2", "vless"];
        if !valid_protocols.contains(&protocol.as_str()) {
            return Err(ApiError::BadRequest(format!(
                "Invalid protocol. Must be one of: {}",
                valid_protocols.join(", ")
            )));
        }
    }

    // Validate status if provided
    if let Some(ref status) = payload.status {
        let valid_statuses = ["online", "offline", "maintenance"];
        if !valid_statuses.contains(&status.as_str()) {
            return Err(ApiError::BadRequest(format!(
                "Invalid status. Must be one of: {}",
                valid_statuses.join(", ")
            )));
        }
    }

    // Validate sort_order if provided (must be non-negative)
    if let Some(sort_order) = payload.sort_order {
        if sort_order < 0 {
            return Err(ApiError::BadRequest("sort_order must be a non-negative integer".to_string()));
        }
    }

    // Update node in database
    let updated_node = db::update_node(
        &state.db_pool,
        node_id,
        payload.name.as_deref(),
        payload.host.as_deref(),
        payload.port,
        payload.protocol.as_deref(),
        payload.config,
        payload.status.as_deref(),
        payload.include_in_clash,
        payload.sort_order,
    )
    .await?;

    // Log admin action
    let _ = db::create_admin_log(
        &state.db_pool,
        claims.sub,
        "update_node",
        Some("node"),
        Some(node_id),
        Some(json!({
            "node_id": node_id,
            "name": payload.name.clone(),
            "host": payload.host.clone(),
            "port": payload.port,
            "protocol": payload.protocol.clone(),
            "status": payload.status.clone(),
            "include_in_clash": payload.include_in_clash,
            "sort_order": payload.sort_order,
        })),
    )
    .await;

    // Invalidate active nodes cache
    if let Err(e) = state.redis_cache.invalidate_active_nodes().await {
        tracing::warn!("Failed to invalidate active nodes cache: {}", e);
    }

    // Notify node agent of configuration update via Redis Pub/Sub
    if let Err(e) = state.redis_cache.publish_node_config_update(node_id).await {
        tracing::warn!("Failed to publish node config update: {}", e);
    }

    Ok(Json(updated_node))
}

/// DELETE /api/admin/nodes/:id - Delete a node (admin only)
async fn admin_delete_node_handler(
    State(state): State<AppState>,
    Path(node_id): Path<i64>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Extract and verify JWT token
    let token = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| ApiError::Unauthorized("Missing or invalid authorization header".to_string()))?;

    let claims = verify_token(token, &state.config.jwt_secret)
        .map_err(|_| ApiError::Unauthorized("Invalid or expired token".to_string()))?;

    // Check if user is admin
    if !claims.is_admin {
        return Err(ApiError::Unauthorized("Admin access required".to_string()));
    }

    // Check if node exists
    let node = db::get_node_by_id(&state.db_pool, node_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Node not found".to_string()))?;

    // Delete node from database
    db::delete_node(&state.db_pool, node_id).await?;

    // Log admin action
    let _ = db::create_admin_log(
        &state.db_pool,
        claims.sub,
        "delete_node",
        Some("node"),
        Some(node_id),
        Some(json!({
            "node_id": node_id,
            "node_name": node.name,
        })),
    )
    .await;

    // Invalidate active nodes cache
    if let Err(e) = state.redis_cache.invalidate_active_nodes().await {
        tracing::warn!("Failed to invalidate active nodes cache: {}", e);
    }

    Ok(Json(json!({
        "message": "Node deleted successfully",
        "node_id": node_id,
    })))
}

// ============================================================================
// Node Agent Handlers
// ============================================================================

/// GET /api/node/config - Get node configuration (for Node Agent)
async fn node_get_config_handler(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Extract node_id and secret from query parameters
    let node_id = params
        .get("node_id")
        .and_then(|s| s.parse::<i64>().ok())
        .ok_or_else(|| ApiError::BadRequest("node_id is required".to_string()))?;

    let secret = params
        .get("secret")
        .ok_or_else(|| ApiError::BadRequest("secret is required".to_string()))?;

    // Authenticate node using ID and secret
    let node = db::get_node_by_id_and_secret(&state.db_pool, node_id, secret)
        .await?
        .ok_or_else(|| ApiError::Unauthorized("Invalid node credentials".to_string()))?;

    // Get active users (users with valid packages)
    let active_users = sqlx::query_as::<_, (i64, String)>(
        r#"
        SELECT DISTINCT u.id, u.email
        FROM users u
        INNER JOIN user_packages up ON u.id = up.user_id
        WHERE u.status = 'active'
          AND up.status = 'active'
          AND up.expires_at > NOW()
          AND up.traffic_used < up.traffic_quota
        "#,
    )
    .fetch_all(&state.db_pool)
    .await?;

    // Build user list for Xray configuration
    let users: Vec<serde_json::Value> = active_users
        .iter()
        .map(|(user_id, email)| {
            json!({
                "id": user_id,
                "email": email,
            })
        })
        .collect();

    // Return node configuration with user list
    Ok(Json(json!({
        "node_id": node.id,
        "name": node.name,
        "host": node.host,
        "port": node.port,
        "protocol": node.protocol,
        "config": node.config,
        "users": users,
        "max_users": node.max_users,
    })))
}

/// POST /api/node/heartbeat - Receive heartbeat from Node Agent
async fn node_heartbeat_handler(
    State(state): State<AppState>,
    Json(payload): Json<crate::models::HeartbeatRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Authenticate node using ID and secret
    let node = db::get_node_by_id_and_secret(&state.db_pool, payload.node_id, &payload.secret)
        .await?
        .ok_or_else(|| ApiError::Unauthorized("Invalid node credentials".to_string()))?;

    // Update node heartbeat and status
    let updated_node = db::update_node_heartbeat(
        &state.db_pool,
        payload.node_id,
        &payload.status,
        payload.active_connections,
    )
    .await?;

    // Invalidate active nodes cache if status changed
    if node.status != updated_node.status {
        if let Err(e) = state.redis_cache.invalidate_active_nodes().await {
            tracing::warn!("Failed to invalidate active nodes cache: {}", e);
        }
    }

    Ok(Json(json!({
        "message": "Heartbeat received",
        "node_id": payload.node_id,
        "status": updated_node.status,
        "last_heartbeat": updated_node.last_heartbeat,
    })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_check() {
        let response = health_check().await;
        assert_eq!(response, "OK");
    }

    #[test]
    fn test_api_error_into_response() {
        let error = ApiError::BadRequest("Test error".to_string());
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let error = ApiError::Unauthorized("Unauthorized".to_string());
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        let error = ApiError::NotFound("Not found".to_string());
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        let error = ApiError::Conflict("Conflict".to_string());
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::CONFLICT);

        let error = ApiError::InternalServerError("Server error".to_string());
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    // Integration tests would go here, but they require a running database
    // These would test the actual register, login, and refresh handlers

    #[test]
    fn test_extract_client_ip_x_forwarded_for_single() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "192.168.1.100".parse().unwrap());
        
        let result = extract_client_ip(&headers);
        assert_eq!(result, Some("192.168.1.100".to_string()));
    }

    #[test]
    fn test_extract_client_ip_x_forwarded_for_multiple() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "203.0.113.45, 198.51.100.178, 192.0.2.1".parse().unwrap());
        
        let result = extract_client_ip(&headers);
        // Should return the first IP in the chain
        assert_eq!(result, Some("203.0.113.45".to_string()));
    }

    #[test]
    fn test_extract_client_ip_x_forwarded_for_with_spaces() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "  10.0.0.1  , 10.0.0.2".parse().unwrap());
        
        let result = extract_client_ip(&headers);
        // Should trim whitespace
        assert_eq!(result, Some("10.0.0.1".to_string()));
    }

    #[test]
    fn test_extract_client_ip_x_real_ip() {
        let mut headers = HeaderMap::new();
        headers.insert("x-real-ip", "172.16.0.50".parse().unwrap());
        
        let result = extract_client_ip(&headers);
        assert_eq!(result, Some("172.16.0.50".to_string()));
    }

    #[test]
    fn test_extract_client_ip_x_forwarded_for_priority() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "203.0.113.45".parse().unwrap());
        headers.insert("x-real-ip", "172.16.0.50".parse().unwrap());
        
        let result = extract_client_ip(&headers);
        // X-Forwarded-For should take priority
        assert_eq!(result, Some("203.0.113.45".to_string()));
    }

    #[test]
    fn test_extract_client_ip_ipv6() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "2001:0db8:85a3:0000:0000:8a2e:0370:7334".parse().unwrap());
        
        let result = extract_client_ip(&headers);
        assert_eq!(result, Some("2001:0db8:85a3:0000:0000:8a2e:0370:7334".to_string()));
    }

    #[test]
    fn test_extract_client_ip_ipv6_compressed() {
        let mut headers = HeaderMap::new();
        headers.insert("x-real-ip", "2001:db8::1".parse().unwrap());
        
        let result = extract_client_ip(&headers);
        assert_eq!(result, Some("2001:db8::1".to_string()));
    }

    #[test]
    fn test_extract_client_ip_missing_headers() {
        let headers = HeaderMap::new();
        
        let result = extract_client_ip(&headers);
        assert_eq!(result, None);
    }

    #[test]
    fn test_extract_client_ip_empty_x_forwarded_for() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "".parse().unwrap());
        
        let result = extract_client_ip(&headers);
        // Should return None for empty header
        assert_eq!(result, None);
    }

    #[test]
    fn test_extract_client_ip_fallback_to_x_real_ip() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "".parse().unwrap());
        headers.insert("x-real-ip", "192.168.1.1".parse().unwrap());
        
        let result = extract_client_ip(&headers);
        // Should fall back to X-Real-IP when X-Forwarded-For is empty
        assert_eq!(result, Some("192.168.1.1".to_string()));
    }

    /// Test that log_access_async returns immediately without blocking
    /// 
    /// This test verifies that the logging function is non-blocking and returns
    /// quickly even though the actual database write happens asynchronously.
    #[tokio::test]
    async fn test_log_access_async_non_blocking() {
        // Create a mock state with a database pool
        // Note: This test doesn't actually connect to a database, it just verifies
        // that the function returns immediately
        
        let start = std::time::Instant::now();
        
        // Create a minimal AppState for testing
        // We'll use a connection string that won't actually connect
        let pool = sqlx::PgPool::connect_lazy("postgresql://test:test@localhost/test")
            .expect("Failed to create pool");
        
        let redis_conn = redis::Client::open("redis://127.0.0.1/")
            .expect("Failed to create redis client")
            .get_connection_manager()
            .await
            .expect("Failed to get connection manager");
        
        let redis_cache = RedisCache::new(redis_conn);
        
        let config = Config {
            database_url: "postgresql://test:test@localhost/test".to_string(),
            redis_url: "redis://127.0.0.1/".to_string(),
            jwt_secret: "test_secret".to_string(),
            jwt_expiration: 3600,
            host: "127.0.0.1".to_string(),
            port: 8080,
            cors_origins: vec!["*".to_string()],
        };
        
        let state = AppState {
            db_pool: pool,
            redis_cache,
            config: Arc::new(config),
        };
        
        // Call log_access_async
        log_access_async(
            &state,
            1,
            "test_token",
            "127.0.0.1",
            Some("test-agent"),
            "success",
        ).await;
        
        let elapsed = start.elapsed();
        
        // Verify that the function returned quickly (within 100ms)
        // The actual database write happens in the background
        assert!(
            elapsed < Duration::from_millis(100),
            "log_access_async should return immediately, took {:?}",
            elapsed
        );
        
        // Give the spawned task a moment to attempt the database write
        // (it will fail since we're not connected to a real database, but that's okay)
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    /// Test that logging failures don't crash the handler
    /// 
    /// This test verifies that even if the database write fails, the function
    /// completes successfully without panicking.
    #[tokio::test]
    async fn test_log_access_async_resilience() {
        // Create a state with an invalid database connection
        let pool = sqlx::PgPool::connect_lazy("postgresql://invalid:invalid@localhost:9999/invalid")
            .expect("Failed to create pool");
        
        let redis_conn = redis::Client::open("redis://127.0.0.1/")
            .expect("Failed to create redis client")
            .get_connection_manager()
            .await
            .expect("Failed to get connection manager");
        
        let redis_cache = RedisCache::new(redis_conn);
        
        let config = Config {
            database_url: "postgresql://invalid:invalid@localhost:9999/invalid".to_string(),
            redis_url: "redis://127.0.0.1/".to_string(),
            jwt_secret: "test_secret".to_string(),
            jwt_expiration: 3600,
            host: "127.0.0.1".to_string(),
            port: 8080,
            cors_origins: vec!["*".to_string()],
        };
        
        let state = AppState {
            db_pool: pool,
            redis_cache,
            config: Arc::new(config),
        };
        
        // This should not panic even though the database connection is invalid
        log_access_async(
            &state,
            1,
            "test_token",
            "192.168.1.1",
            Some("Mozilla/5.0"),
            "failed",
        ).await;
        
        // Give the spawned task time to fail
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // If we reach here without panicking, the test passes
        // The error will be logged but won't crash the application
    }
}

// ============================================================================
// Admin User Management Handlers
// ============================================================================

/// GET /api/admin/users - Get list of all users (admin only)
async fn admin_list_users_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Extract and verify JWT token
    let token = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| ApiError::Unauthorized("Missing or invalid authorization header".to_string()))?;

    let claims = verify_token(token, &state.config.jwt_secret)
        .map_err(|_| ApiError::Unauthorized("Invalid or expired token".to_string()))?;

    // Check if user is admin
    if !claims.is_admin {
        return Err(ApiError::Unauthorized("Admin access required".to_string()));
    }

    // Parse pagination parameters
    let limit = params
        .get("limit")
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(50);
    let offset = params
        .get("offset")
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(0);

    // Get users from database
    let users = db::list_users(&state.db_pool, limit, offset).await?;
    let total = db::count_users(&state.db_pool).await?;

    // Convert to response format (without password hash)
    let user_responses: Vec<crate::models::UserResponse> = users
        .into_iter()
        .map(|u| u.into())
        .collect();

    Ok(Json(json!({
        "users": user_responses,
        "total": total,
        "limit": limit,
        "offset": offset,
    })))
}

/// GET /api/admin/users/:id - Get user details (admin only)
async fn admin_get_user_handler(
    State(state): State<AppState>,
    Path(user_id): Path<i64>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Extract and verify JWT token
    let token = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| ApiError::Unauthorized("Missing or invalid authorization header".to_string()))?;

    let claims = verify_token(token, &state.config.jwt_secret)
        .map_err(|_| ApiError::Unauthorized("Invalid or expired token".to_string()))?;

    // Check if user is admin
    if !claims.is_admin {
        return Err(ApiError::Unauthorized("Admin access required".to_string()));
    }

    // Get user from database
    let user = db::get_user_by_id(&state.db_pool, user_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("User not found".to_string()))?;

    // Get user's active packages
    let user_packages = sqlx::query_as::<_, crate::models::UserPackage>(
        r#"
        SELECT * FROM user_packages
        WHERE user_id = $1
        ORDER BY created_at DESC
        LIMIT 10
        "#,
    )
    .bind(user_id)
    .fetch_all(&state.db_pool)
    .await?;

    // Get user's recent orders
    let orders = db::list_orders_by_user(&state.db_pool, user_id, 10, 0).await?;

    // Get referral stats
    let (referral_count, total_rebate) = db::get_referral_stats(&state.db_pool, user_id).await?;

    Ok(Json(json!({
        "user": crate::models::UserResponse::from(user),
        "packages": user_packages,
        "recent_orders": orders,
        "referral_stats": {
            "referral_count": referral_count,
            "total_rebate": total_rebate,
        },
    })))
}

/// PUT /api/admin/users/:id/status - Update user status (admin only)
async fn admin_update_user_status_handler(
    State(state): State<AppState>,
    Path(user_id): Path<i64>,
    headers: HeaderMap,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Extract and verify JWT token
    let token = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| ApiError::Unauthorized("Missing or invalid authorization header".to_string()))?;

    let claims = verify_token(token, &state.config.jwt_secret)
        .map_err(|_| ApiError::Unauthorized("Invalid or expired token".to_string()))?;

    // Check if user is admin
    if !claims.is_admin {
        return Err(ApiError::Unauthorized("Admin access required".to_string()));
    }

    // Extract status from payload
    let status = payload
        .get("status")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ApiError::BadRequest("status is required".to_string()))?;

    // Validate status
    let valid_statuses = ["active", "disabled"];
    if !valid_statuses.contains(&status) {
        return Err(ApiError::BadRequest(format!(
            "Invalid status. Must be one of: {}",
            valid_statuses.join(", ")
        )));
    }

    // Check if user exists
    let _existing_user = db::get_user_by_id(&state.db_pool, user_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("User not found".to_string()))?;

    // Update user status
    let updated_user = db::update_user_status(&state.db_pool, user_id, status).await?;

    // Log admin action
    let _ = db::create_admin_log(
        &state.db_pool,
        claims.sub,
        "update_user_status",
        Some("user"),
        Some(user_id),
        Some(json!({
            "user_id": user_id,
            "new_status": status,
        })),
    )
    .await;

    // Invalidate user package cache
    if let Err(e) = state.redis_cache.invalidate_user_package(user_id).await {
        tracing::warn!("Failed to invalidate user package cache: {}", e);
    }

    Ok(Json(json!({
        "message": "User status updated successfully",
        "user": crate::models::UserResponse::from(updated_user),
    })))
}

/// PUT /api/admin/users/:id/balance - Update user coin balance (admin only)
async fn admin_update_user_balance_handler(
    State(state): State<AppState>,
    Path(user_id): Path<i64>,
    headers: HeaderMap,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Extract and verify JWT token
    let token = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| ApiError::Unauthorized("Missing or invalid authorization header".to_string()))?;

    let claims = verify_token(token, &state.config.jwt_secret)
        .map_err(|_| ApiError::Unauthorized("Invalid or expired token".to_string()))?;

    // Check if user is admin
    if !claims.is_admin {
        return Err(ApiError::Unauthorized("Admin access required".to_string()));
    }

    // Extract amount from payload
    let amount = payload
        .get("amount")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| ApiError::BadRequest("amount is required".to_string()))?;

    let reason = payload
        .get("reason")
        .and_then(|v| v.as_str())
        .unwrap_or("Admin adjustment");

    // Check if user exists
    let user = db::get_user_by_id(&state.db_pool, user_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("User not found".to_string()))?;

    // Calculate new balance
    let new_balance = user.coin_balance + amount;

    // Ensure balance doesn't go negative
    if new_balance < 0 {
        return Err(ApiError::BadRequest("Balance cannot be negative".to_string()));
    }

    // Update user balance
    let updated_user = db::update_user_coin_balance(&state.db_pool, user_id, new_balance).await?;

    // Create coin transaction record
    let transaction_type = if amount >= 0 { "recharge" } else { "purchase" };
    let _ = db::create_coin_transaction(
        &state.db_pool,
        user_id,
        amount,
        transaction_type,
        Some(reason),
    )
    .await;

    // Log admin action
    let _ = db::create_admin_log(
        &state.db_pool,
        claims.sub,
        "update_user_balance",
        Some("user"),
        Some(user_id),
        Some(json!({
            "user_id": user_id,
            "amount": amount,
            "old_balance": user.coin_balance,
            "new_balance": new_balance,
            "reason": reason,
        })),
    )
    .await;

    Ok(Json(json!({
        "message": "User balance updated successfully",
        "user": crate::models::UserResponse::from(updated_user),
        "old_balance": user.coin_balance,
        "new_balance": new_balance,
        "amount": amount,
    })))
}

/// PUT /api/admin/users/:id/traffic - Update user traffic quota (admin only)
async fn admin_update_user_traffic_handler(
    State(state): State<AppState>,
    Path(user_id): Path<i64>,
    headers: HeaderMap,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Extract and verify JWT token
    let token = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| ApiError::Unauthorized("Missing or invalid authorization header".to_string()))?;

    let claims = verify_token(token, &state.config.jwt_secret)
        .map_err(|_| ApiError::Unauthorized("Invalid or expired token".to_string()))?;

    // Check if user is admin
    if !claims.is_admin {
        return Err(ApiError::Unauthorized("Admin access required".to_string()));
    }

    // Extract traffic_quota from payload (optional)
    let traffic_quota = payload
        .get("traffic_quota")
        .and_then(|v| v.as_i64());

    // Extract traffic_used from payload (optional)
    let traffic_used = payload
        .get("traffic_used")
        .and_then(|v| v.as_i64());

    let reason = payload
        .get("reason")
        .and_then(|v| v.as_str())
        .unwrap_or("Admin adjustment");

    // Check if user exists
    let user = db::get_user_by_id(&state.db_pool, user_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("User not found".to_string()))?;

    // Use existing values if not provided
    let new_quota = traffic_quota.unwrap_or(user.traffic_quota);
    let new_used = traffic_used.unwrap_or(user.traffic_used);

    // Validate values
    if new_quota < 0 || new_used < 0 {
        return Err(ApiError::BadRequest("Traffic values cannot be negative".to_string()));
    }

    // Update user traffic
    let updated_user = db::update_user_traffic(&state.db_pool, user_id, new_quota, new_used).await?;

    // Log admin action
    let _ = db::create_admin_log(
        &state.db_pool,
        claims.sub,
        "update_user_traffic",
        Some("user"),
        Some(user_id),
        Some(json!({
            "user_id": user_id,
            "old_traffic_quota": user.traffic_quota,
            "new_traffic_quota": new_quota,
            "old_traffic_used": user.traffic_used,
            "new_traffic_used": new_used,
            "reason": reason,
        })),
    )
    .await;

    // Invalidate user package cache
    if let Err(e) = state.redis_cache.invalidate_user_package(user_id).await {
        tracing::warn!("Failed to invalidate user package cache: {}", e);
    }

    Ok(Json(json!({
        "message": "User traffic updated successfully",
        "user": crate::models::UserResponse::from(updated_user),
        "old_traffic_quota": user.traffic_quota,
        "new_traffic_quota": new_quota,
        "old_traffic_used": user.traffic_used,
        "new_traffic_used": new_used,
    })))
}

// ============================================================================
// Admin Order Management Handlers
// ============================================================================

/// GET /api/admin/orders - Get list of all orders (admin only)
async fn admin_list_orders_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Extract and verify JWT token
    let token = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| ApiError::Unauthorized("Missing or invalid authorization header".to_string()))?;

    let claims = verify_token(token, &state.config.jwt_secret)
        .map_err(|_| ApiError::Unauthorized("Invalid or expired token".to_string()))?;

    // Check if user is admin
    if !claims.is_admin {
        return Err(ApiError::Unauthorized("Admin access required".to_string()));
    }

    // Parse pagination parameters
    let limit = params
        .get("limit")
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(50);
    let offset = params
        .get("offset")
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(0);

    // Parse filter parameters
    let status_filter = params.get("status").map(|s| s.as_str());
    let user_id_filter = params.get("user_id").and_then(|s| s.parse::<i64>().ok());

    // Get orders from database
    let orders = if let Some(user_id) = user_id_filter {
        db::list_orders_by_user(&state.db_pool, user_id, limit, offset).await?
    } else {
        db::list_all_orders(&state.db_pool, limit, offset).await?
    };

    // Filter by status if provided
    let filtered_orders: Vec<_> = if let Some(status) = status_filter {
        orders.into_iter().filter(|o| o.status == status).collect()
    } else {
        orders
    };

    // Enrich orders with user and package information
    let mut orders_with_details = Vec::new();
    for order in filtered_orders {
        let user = db::get_user_by_id(&state.db_pool, order.user_id).await?;
        let package = db::get_package_by_id(&state.db_pool, order.package_id).await?;
        
        orders_with_details.push(json!({
            "id": order.id,
            "order_no": order.order_no,
            "user_id": order.user_id,
            "user_email": user.as_ref().map(|u| &u.email),
            "package_id": order.package_id,
            "package_name": package.as_ref().map(|p| &p.name),
            "amount": order.amount,
            "status": order.status,
            "created_at": order.created_at,
            "completed_at": order.completed_at,
        }));
    }

    Ok(Json(json!({
        "orders": orders_with_details,
        "total": orders_with_details.len(),
        "limit": limit,
        "offset": offset,
    })))
}

/// GET /api/admin/orders/:id - Get order details (admin only)
async fn admin_get_order_handler(
    State(state): State<AppState>,
    Path(order_id): Path<i64>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Extract and verify JWT token
    let token = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| ApiError::Unauthorized("Missing or invalid authorization header".to_string()))?;

    let claims = verify_token(token, &state.config.jwt_secret)
        .map_err(|_| ApiError::Unauthorized("Invalid or expired token".to_string()))?;

    // Check if user is admin
    if !claims.is_admin {
        return Err(ApiError::Unauthorized("Admin access required".to_string()));
    }

    // Get order from database
    let order = db::get_order_by_id(&state.db_pool, order_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Order not found".to_string()))?;

    // Get user details
    let user = db::get_user_by_id(&state.db_pool, order.user_id).await?;

    // Get package details
    let package = db::get_package_by_id(&state.db_pool, order.package_id).await?;

    // Get user package details if exists
    let user_package = sqlx::query_as::<_, crate::models::UserPackage>(
        r#"
        SELECT * FROM user_packages
        WHERE order_id = $1
        "#,
    )
    .bind(order_id)
    .fetch_optional(&state.db_pool)
    .await?;

    Ok(Json(json!({
        "order": order,
        "user": user.map(|u| crate::models::UserResponse::from(u)),
        "package": package,
        "user_package": user_package,
    })))
}

// ============================================================================
// Admin Statistics Handlers
// ============================================================================

/// GET /api/admin/stats/overview - Get overview statistics (admin only)
async fn admin_stats_overview_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<crate::models::StatsOverview>, ApiError> {
    // Extract and verify JWT token
    let token = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| ApiError::Unauthorized("Missing or invalid authorization header".to_string()))?;

    let claims = verify_token(token, &state.config.jwt_secret)
        .map_err(|_| ApiError::Unauthorized("Invalid or expired token".to_string()))?;

    // Check if user is admin
    if !claims.is_admin {
        return Err(ApiError::Unauthorized("Admin access required".to_string()));
    }

    // Get total users
    let total_users = db::count_users(&state.db_pool).await?;

    // Get active users (users with active packages)
    let active_users: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(DISTINCT user_id) FROM user_packages
        WHERE status = 'active' AND expires_at > NOW()
        "#,
    )
    .fetch_one(&state.db_pool)
    .await?;

    // Get total traffic (sum of all traffic used)
    let total_traffic: (Option<i64>,) = sqlx::query_as(
        r#"
        SELECT COALESCE(SUM(traffic_used), 0)::BIGINT FROM users
        "#,
    )
    .fetch_one(&state.db_pool)
    .await?;

    // Get total revenue
    let total_revenue = db::get_total_revenue(&state.db_pool).await?;

    // Get online nodes count
    let online_nodes = db::count_nodes_by_status(&state.db_pool, "online").await?;

    Ok(Json(crate::models::StatsOverview {
        total_users,
        active_users: active_users.0,
        total_traffic: total_traffic.0.unwrap_or(0),
        total_revenue,
        online_nodes,
    }))
}

/// GET /api/admin/stats/revenue - Get revenue statistics (admin only)
async fn admin_stats_revenue_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Extract and verify JWT token
    let token = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| ApiError::Unauthorized("Missing or invalid authorization header".to_string()))?;

    let claims = verify_token(token, &state.config.jwt_secret)
        .map_err(|_| ApiError::Unauthorized("Invalid or expired token".to_string()))?;

    // Check if user is admin
    if !claims.is_admin {
        return Err(ApiError::Unauthorized("Admin access required".to_string()));
    }

    // Parse time range parameters
    let start_date = params.get("start_date").map(|s| s.as_str());
    let end_date = params.get("end_date").map(|s| s.as_str());

    // Build query based on time range
    let revenue_query = if let (Some(start), Some(end)) = (start_date, end_date) {
        sqlx::query_as::<_, (Option<i64>, i64)>(
            r#"
            SELECT COALESCE(SUM(amount), 0)::BIGINT, COUNT(*)
            FROM orders
            WHERE status = 'completed'
              AND created_at >= $1::timestamp
              AND created_at <= $2::timestamp
            "#,
        )
        .bind(start)
        .bind(end)
        .fetch_one(&state.db_pool)
        .await?
    } else {
        sqlx::query_as::<_, (Option<i64>, i64)>(
            r#"
            SELECT COALESCE(SUM(amount), 0)::BIGINT, COUNT(*)
            FROM orders
            WHERE status = 'completed'
            "#,
        )
        .fetch_one(&state.db_pool)
        .await?
    };

    let total_revenue = revenue_query.0.unwrap_or(0);
    let order_count = revenue_query.1;

    // Get daily revenue for the last 30 days
    let daily_revenue: Vec<(String, i64)> = sqlx::query_as(
        r#"
        SELECT DATE(created_at) as date, COALESCE(SUM(amount), 0)::BIGINT as revenue
        FROM orders
        WHERE status = 'completed'
          AND created_at >= NOW() - INTERVAL '30 days'
        GROUP BY DATE(created_at)
        ORDER BY date DESC
        "#,
    )
    .fetch_all(&state.db_pool)
    .await?;

    Ok(Json(json!({
        "total_revenue": total_revenue,
        "order_count": order_count,
        "daily_revenue": daily_revenue,
        "start_date": start_date,
        "end_date": end_date,
    })))
}

/// GET /api/admin/stats/traffic - Get traffic statistics (admin only)
async fn admin_stats_traffic_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Extract and verify JWT token
    let token = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| ApiError::Unauthorized("Missing or invalid authorization header".to_string()))?;

    let claims = verify_token(token, &state.config.jwt_secret)
        .map_err(|_| ApiError::Unauthorized("Invalid or expired token".to_string()))?;

    // Check if user is admin
    if !claims.is_admin {
        return Err(ApiError::Unauthorized("Admin access required".to_string()));
    }

    // Parse time range parameters
    let start_date = params.get("start_date").map(|s| s.as_str());
    let end_date = params.get("end_date").map(|s| s.as_str());

    // Get total traffic by node
    let node_traffic: Vec<(i64, String, i64, i64)> = sqlx::query_as(
        r#"
        SELECT id, name, total_upload, total_download
        FROM nodes
        ORDER BY (total_upload + total_download) DESC
        "#,
    )
    .fetch_all(&state.db_pool)
    .await?;

    // Get daily traffic for the last 30 days
    let daily_traffic_query = if let (Some(start), Some(end)) = (start_date, end_date) {
        sqlx::query_as::<_, (String, i64, i64)>(
            r#"
            SELECT DATE(recorded_at) as date,
                   COALESCE(SUM(upload), 0)::BIGINT as upload,
                   COALESCE(SUM(download), 0)::BIGINT as download
            FROM traffic_logs
            WHERE recorded_at >= $1::timestamp
              AND recorded_at <= $2::timestamp
            GROUP BY DATE(recorded_at)
            ORDER BY date DESC
            "#,
        )
        .bind(start)
        .bind(end)
        .fetch_all(&state.db_pool)
        .await?
    } else {
        sqlx::query_as::<_, (String, i64, i64)>(
            r#"
            SELECT DATE(recorded_at) as date,
                   COALESCE(SUM(upload), 0)::BIGINT as upload,
                   COALESCE(SUM(download), 0)::BIGINT as download
            FROM traffic_logs
            WHERE recorded_at >= NOW() - INTERVAL '30 days'
            GROUP BY DATE(recorded_at)
            ORDER BY date DESC
            "#,
        )
        .fetch_all(&state.db_pool)
        .await?
    };

    // Format node traffic data
    let node_stats: Vec<serde_json::Value> = node_traffic
        .into_iter()
        .map(|(id, name, upload, download)| {
            json!({
                "node_id": id,
                "node_name": name,
                "total_upload": upload,
                "total_download": download,
                "total_traffic": upload + download,
            })
        })
        .collect();

    // Format daily traffic data
    let daily_stats: Vec<serde_json::Value> = daily_traffic_query
        .into_iter()
        .map(|(date, upload, download)| {
            json!({
                "date": date,
                "upload": upload,
                "download": download,
                "total": upload + download,
            })
        })
        .collect();

    Ok(Json(json!({
        "node_traffic": node_stats,
        "daily_traffic": daily_stats,
        "start_date": start_date,
        "end_date": end_date,
    })))
}


// ============================================================================
// Clash Configuration Management Handlers
// ============================================================================

// Note: Clash proxy management handlers have been removed as part of node-proxy unification.
// Proxies are now managed through the node management endpoints (/api/admin/nodes).
// The following handlers remain for proxy groups and rules management:

/// GET /api/admin/clash/proxy-groups - Get all Clash proxy groups (admin only)
async fn admin_list_clash_proxy_groups_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<Vec<crate::models::ClashProxyGroup>>, ApiError> {
    // Extract and verify JWT token
    let token = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| ApiError::Unauthorized("Missing or invalid authorization header".to_string()))?;

    let claims = verify_token(token, &state.config.jwt_secret)
        .map_err(|_| ApiError::Unauthorized("Invalid or expired token".to_string()))?;

    // Check if user is admin
    if !claims.is_admin {
        return Err(ApiError::Unauthorized("Admin access required".to_string()));
    }

    // Parse active_only parameter
    let active_only = params
        .get("active_only")
        .and_then(|s| s.parse::<bool>().ok())
        .unwrap_or(false);

    // Get proxy groups from database
    let groups = db::list_clash_proxy_groups(&state.db_pool, active_only).await?;

    Ok(Json(groups))
}

/// POST /api/admin/clash/proxy-groups - Create a new Clash proxy group (admin only)
async fn admin_create_clash_proxy_group_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<crate::models::ClashProxyGroupRequest>,
) -> Result<Json<crate::models::ClashProxyGroup>, ApiError> {
    // Extract and verify JWT token
    let token = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| ApiError::Unauthorized("Missing or invalid authorization header".to_string()))?;

    let claims = verify_token(token, &state.config.jwt_secret)
        .map_err(|_| ApiError::Unauthorized("Invalid or expired token".to_string()))?;

    // Check if user is admin
    if !claims.is_admin {
        return Err(ApiError::Unauthorized("Admin access required".to_string()));
    }

    // Validate group type
    let valid_types = ["select", "url-test", "fallback", "load-balance", "relay"];
    if !valid_types.contains(&payload.group_type.as_str()) {
        return Err(ApiError::BadRequest(format!(
            "Invalid group type. Must be one of: {}",
            valid_types.join(", ")
        )));
    }

    // Create proxy group in database
    let group = db::create_clash_proxy_group(
        &state.db_pool,
        &payload.name,
        &payload.group_type,
        &payload.proxies,
        payload.url.as_deref(),
        payload.interval,
        payload.tolerance,
        payload.is_active.unwrap_or(true),
        payload.sort_order.unwrap_or(0),
    )
    .await?;

    // Log admin action
    let _ = db::create_admin_log(
        &state.db_pool,
        claims.sub,
        "create_clash_proxy_group",
        Some("clash_proxy_group"),
        Some(group.id),
        Some(json!({
            "group_name": &payload.name,
            "group_type": &payload.group_type,
        })),
    )
    .await;

    Ok(Json(group))
}

/// PUT /api/admin/clash/proxy-groups/:id - Update a Clash proxy group (admin only)
async fn admin_update_clash_proxy_group_handler(
    State(state): State<AppState>,
    Path(group_id): Path<i64>,
    headers: HeaderMap,
    Json(payload): Json<crate::models::ClashProxyGroupRequest>,
) -> Result<Json<crate::models::ClashProxyGroup>, ApiError> {
    // Extract and verify JWT token
    let token = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| ApiError::Unauthorized("Missing or invalid authorization header".to_string()))?;

    let claims = verify_token(token, &state.config.jwt_secret)
        .map_err(|_| ApiError::Unauthorized("Invalid or expired token".to_string()))?;

    // Check if user is admin
    if !claims.is_admin {
        return Err(ApiError::Unauthorized("Admin access required".to_string()));
    }

    // Check if proxy group exists
    let _existing_group = db::get_clash_proxy_group_by_id(&state.db_pool, group_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Proxy group not found".to_string()))?;

    // Update proxy group in database
    let updated_group = db::update_clash_proxy_group(
        &state.db_pool,
        group_id,
        Some(&payload.name),
        Some(&payload.group_type),
        Some(&payload.proxies),
        Some(payload.url.as_deref()),
        Some(payload.interval),
        Some(payload.tolerance),
        payload.is_active,
        payload.sort_order,
    )
    .await?;

    // Log admin action
    let _ = db::create_admin_log(
        &state.db_pool,
        claims.sub,
        "update_clash_proxy_group",
        Some("clash_proxy_group"),
        Some(group_id),
        Some(json!({
            "group_id": group_id,
            "group_name": &payload.name,
        })),
    )
    .await;

    Ok(Json(updated_group))
}

/// DELETE /api/admin/clash/proxy-groups/:id - Delete a Clash proxy group (admin only)
async fn admin_delete_clash_proxy_group_handler(
    State(state): State<AppState>,
    Path(group_id): Path<i64>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Extract and verify JWT token
    let token = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| ApiError::Unauthorized("Missing or invalid authorization header".to_string()))?;

    let claims = verify_token(token, &state.config.jwt_secret)
        .map_err(|_| ApiError::Unauthorized("Invalid or expired token".to_string()))?;

    // Check if user is admin
    if !claims.is_admin {
        return Err(ApiError::Unauthorized("Admin access required".to_string()));
    }

    // Check if proxy group exists
    let group = db::get_clash_proxy_group_by_id(&state.db_pool, group_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Proxy group not found".to_string()))?;

    // Delete proxy group from database
    db::delete_clash_proxy_group(&state.db_pool, group_id).await?;

    // Log admin action
    let _ = db::create_admin_log(
        &state.db_pool,
        claims.sub,
        "delete_clash_proxy_group",
        Some("clash_proxy_group"),
        Some(group_id),
        Some(json!({
            "group_id": group_id,
            "group_name": group.name,
        })),
    )
    .await;

    Ok(Json(json!({
        "message": "Clash proxy group deleted successfully",
        "group_id": group_id,
    })))
}

/// GET /api/admin/clash/rules - Get all Clash rules (admin only)
async fn admin_list_clash_rules_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<Vec<crate::models::ClashRule>>, ApiError> {
    // Extract and verify JWT token
    let token = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| ApiError::Unauthorized("Missing or invalid authorization header".to_string()))?;

    let claims = verify_token(token, &state.config.jwt_secret)
        .map_err(|_| ApiError::Unauthorized("Invalid or expired token".to_string()))?;

    // Check if user is admin
    if !claims.is_admin {
        return Err(ApiError::Unauthorized("Admin access required".to_string()));
    }

    // Parse active_only parameter
    let active_only = params
        .get("active_only")
        .and_then(|s| s.parse::<bool>().ok())
        .unwrap_or(false);

    // Get rules from database
    let rules = db::list_clash_rules(&state.db_pool, active_only).await?;

    Ok(Json(rules))
}

/// POST /api/admin/clash/rules - Create a new Clash rule (admin only)
async fn admin_create_clash_rule_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<crate::models::ClashRuleRequest>,
) -> Result<Json<crate::models::ClashRule>, ApiError> {
    // Extract and verify JWT token
    let token = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| ApiError::Unauthorized("Missing or invalid authorization header".to_string()))?;

    let claims = verify_token(token, &state.config.jwt_secret)
        .map_err(|_| ApiError::Unauthorized("Invalid or expired token".to_string()))?;

    // Check if user is admin
    if !claims.is_admin {
        return Err(ApiError::Unauthorized("Admin access required".to_string()));
    }

    // Validate rule type
    let valid_types = [
        "DOMAIN", "DOMAIN-SUFFIX", "DOMAIN-KEYWORD",
        "IP-CIDR", "IP-CIDR6", "SRC-IP-CIDR",
        "GEOIP", "DST-PORT", "SRC-PORT",
        "PROCESS-NAME", "MATCH"
    ];
    if !valid_types.contains(&payload.rule_type.as_str()) {
        return Err(ApiError::BadRequest(format!(
            "Invalid rule type. Must be one of: {}",
            valid_types.join(", ")
        )));
    }

    // Create rule in database
    let rule = db::create_clash_rule(
        &state.db_pool,
        &payload.rule_type,
        payload.rule_value.as_deref(),
        &payload.proxy_group,
        payload.no_resolve.unwrap_or(false),
        payload.is_active.unwrap_or(true),
        payload.sort_order.unwrap_or(0),
        payload.description.as_deref(),
    )
    .await?;

    // Log admin action
    let _ = db::create_admin_log(
        &state.db_pool,
        claims.sub,
        "create_clash_rule",
        Some("clash_rule"),
        Some(rule.id),
        Some(json!({
            "rule_type": &payload.rule_type,
            "proxy_group": &payload.proxy_group,
        })),
    )
    .await;

    Ok(Json(rule))
}

/// PUT /api/admin/clash/rules/:id - Update a Clash rule (admin only)
async fn admin_update_clash_rule_handler(
    State(state): State<AppState>,
    Path(rule_id): Path<i64>,
    headers: HeaderMap,
    Json(payload): Json<crate::models::ClashRuleRequest>,
) -> Result<Json<crate::models::ClashRule>, ApiError> {
    // Extract and verify JWT token
    let token = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| ApiError::Unauthorized("Missing or invalid authorization header".to_string()))?;

    let claims = verify_token(token, &state.config.jwt_secret)
        .map_err(|_| ApiError::Unauthorized("Invalid or expired token".to_string()))?;

    // Check if user is admin
    if !claims.is_admin {
        return Err(ApiError::Unauthorized("Admin access required".to_string()));
    }

    // Check if rule exists
    let _existing_rule = db::get_clash_rule_by_id(&state.db_pool, rule_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Rule not found".to_string()))?;

    // Update rule in database
    let updated_rule = db::update_clash_rule(
        &state.db_pool,
        rule_id,
        Some(&payload.rule_type),
        Some(payload.rule_value.as_deref()),
        Some(&payload.proxy_group),
        payload.no_resolve,
        payload.is_active,
        payload.sort_order,
        Some(payload.description.as_deref()),
    )
    .await?;

    // Log admin action
    let _ = db::create_admin_log(
        &state.db_pool,
        claims.sub,
        "update_clash_rule",
        Some("clash_rule"),
        Some(rule_id),
        Some(json!({
            "rule_id": rule_id,
            "rule_type": &payload.rule_type,
        })),
    )
    .await;

    Ok(Json(updated_rule))
}

/// DELETE /api/admin/clash/rules/:id - Delete a Clash rule (admin only)
async fn admin_delete_clash_rule_handler(
    State(state): State<AppState>,
    Path(rule_id): Path<i64>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Extract and verify JWT token
    let token = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| ApiError::Unauthorized("Missing or invalid authorization header".to_string()))?;

    let claims = verify_token(token, &state.config.jwt_secret)
        .map_err(|_| ApiError::Unauthorized("Invalid or expired token".to_string()))?;

    // Check if user is admin
    if !claims.is_admin {
        return Err(ApiError::Unauthorized("Admin access required".to_string()));
    }

    // Check if rule exists
    let rule = db::get_clash_rule_by_id(&state.db_pool, rule_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Rule not found".to_string()))?;

    // Delete rule from database
    db::delete_clash_rule(&state.db_pool, rule_id).await?;

    // Log admin action
    let _ = db::create_admin_log(
        &state.db_pool,
        claims.sub,
        "delete_clash_rule",
        Some("clash_rule"),
        Some(rule_id),
        Some(json!({
            "rule_id": rule_id,
            "rule_type": rule.rule_type,
        })),
    )
    .await;

    Ok(Json(json!({
        "message": "Clash rule deleted successfully",
        "rule_id": rule_id,
    })))
}

/// GET /api/admin/clash/generate - Generate Clash YAML configuration (admin only)
async fn admin_generate_clash_config_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    // Extract and verify JWT token
    let token = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| ApiError::Unauthorized("Missing or invalid authorization header".to_string()))?;

    let claims = verify_token(token, &state.config.jwt_secret)
        .map_err(|_| ApiError::Unauthorized("Invalid or expired token".to_string()))?;

    // Check if user is admin
    if !claims.is_admin {
        return Err(ApiError::Unauthorized("Admin access required".to_string()));
    }

    // Generate Clash configuration from nodes
    let clash_config = crate::clash::generate_clash_config_from_nodes(&state.db_pool).await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to generate config: {}", e)))?;

    Ok((
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "text/yaml; charset=utf-8")],
        clash_config,
    ))
}

// ============================================================================
// Access Logs Management (Admin)
// ============================================================================

/// GET /api/admin/access-logs - Query access logs (admin only)
async fn admin_query_access_logs_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Query(params): axum::extract::Query<crate::models::AccessLogQueryRequest>,
) -> Result<Json<crate::models::AccessLogListResponse>, ApiError> {
    // Extract and verify JWT token from Authorization header
    let token = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| ApiError::Unauthorized("Missing authorization header".to_string()))?;

    let claims = verify_token(token, &state.config.jwt_secret)
        .map_err(|_| ApiError::Unauthorized("Invalid token".to_string()))?;

    // Verify user is admin using get_user_by_id
    let user = db::get_user_by_id(&state.db_pool, claims.sub)
        .await?
        .ok_or_else(|| ApiError::Unauthorized("User not found".to_string()))?;

    if !user.is_admin {
        return Err(ApiError::Unauthorized("Admin access required".to_string()));
    }

    // Set default pagination values (page=1, page_size=50)
    let page = params.page.unwrap_or(1).max(1);
    let page_size = params.page_size.unwrap_or(50).clamp(1, 100);

    // Call db::query_access_logs with filters
    let (logs, total) = db::query_access_logs(
        &state.db_pool,
        params.user_id,
        params.start_date,
        params.end_date,
        params.status.as_deref(),
        page,
        page_size,
    )
    .await?;

    // Calculate total_pages from total count
    let total_pages = (total + page_size - 1) / page_size;

    // Return Json<AccessLogListResponse>
    Ok(Json(crate::models::AccessLogListResponse {
        logs,
        total,
        page,
        page_size,
        total_pages,
    }))
}
