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

    Ok(Json(json!({
        "referral_count": referral_count,
        "total_rebate": total_rebate,
        "referral_code": user.referral_code,
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
) -> Result<impl IntoResponse, ApiError> {
    // Try to get from cache first
    if let Ok(Some(cached_config)) = state.redis_cache.get_subscription_config(&token).await {
        tracing::debug!("Subscription config cache hit for token {}", token);
        return Ok((
            StatusCode::OK,
            [(axum::http::header::CONTENT_TYPE, "text/yaml; charset=utf-8")],
            cached_config,
        ));
    }

    tracing::debug!("Subscription config cache miss for token {}", token);

    // Get subscription from database
    let subscription = db::get_subscription_by_token(&state.db_pool, &token)
        .await?
        .ok_or_else(|| ApiError::NotFound("Subscription not found".to_string()))?;

    // Get user
    let user = db::get_user_by_id(&state.db_pool, subscription.user_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("User not found".to_string()))?;

    // Check if user is active
    if user.status == "disabled" {
        return Err(ApiError::Unauthorized("Account is disabled".to_string()));
    }

    // Check if user has exceeded traffic quota
    let has_traffic = traffic::check_traffic_quota(&state.db_pool, user.id)
        .await
        .unwrap_or(false);

    if !has_traffic {
        tracing::warn!("User {} has exceeded traffic quota", user.id);
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
        let empty_config = "proxies: []\nproxy-groups: []\nrules: []\n";
        return Ok((
            StatusCode::OK,
            [(axum::http::header::CONTENT_TYPE, "text/yaml; charset=utf-8")],
            empty_config.to_string(),
        ));
    }

    // Get active nodes
    let nodes = db::list_nodes_by_status(&state.db_pool, "online").await?;

    // Generate Clash configuration
    let clash_config = crate::clash::generate_clash_config(&nodes)
        .map_err(|e| ApiError::InternalServerError(format!("Failed to generate config: {}", e)))?;

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
