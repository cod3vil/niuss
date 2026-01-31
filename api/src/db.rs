use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::{postgres::PgPoolOptions, PgPool};

use crate::models::{
    AdminLog, CoinTransaction, Node, Order, Package, Subscription, TrafficLog, User, UserPackage,
};

/// Create a database connection pool
pub async fn create_pool(database_url: &str) -> Result<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await?;

    Ok(pool)
}

// ============================================================================
// User CRUD Operations
// ============================================================================

/// Create a new user
pub async fn create_user(
    pool: &PgPool,
    email: &str,
    password_hash: &str,
    referral_code: Option<&str>,
    referred_by: Option<i64>,
) -> Result<User> {
    let user = sqlx::query_as::<_, User>(
        r#"
        INSERT INTO users (email, password_hash, referral_code, referred_by)
        VALUES ($1, $2, $3, $4)
        RETURNING *
        "#,
    )
    .bind(email)
    .bind(password_hash)
    .bind(referral_code)
    .bind(referred_by)
    .fetch_one(pool)
    .await?;

    Ok(user)
}

/// Get user by ID
pub async fn get_user_by_id(pool: &PgPool, user_id: i64) -> Result<Option<User>> {
    let user = sqlx::query_as::<_, User>(
        r#"
        SELECT * FROM users WHERE id = $1
        "#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    Ok(user)
}

/// Get user by email
pub async fn get_user_by_email(pool: &PgPool, email: &str) -> Result<Option<User>> {
    let user = sqlx::query_as::<_, User>(
        r#"
        SELECT * FROM users WHERE email = $1
        "#,
    )
    .bind(email)
    .fetch_optional(pool)
    .await?;

    Ok(user)
}

/// Get user by referral code
pub async fn get_user_by_referral_code(
    pool: &PgPool,
    referral_code: &str,
) -> Result<Option<User>> {
    let user = sqlx::query_as::<_, User>(
        r#"
        SELECT * FROM users WHERE referral_code = $1
        "#,
    )
    .bind(referral_code)
    .fetch_optional(pool)
    .await?;

    Ok(user)
}

/// Update user coin balance
pub async fn update_user_coin_balance(
    pool: &PgPool,
    user_id: i64,
    new_balance: i64,
) -> Result<User> {
    let user = sqlx::query_as::<_, User>(
        r#"
        UPDATE users
        SET coin_balance = $2, updated_at = NOW()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(new_balance)
    .fetch_one(pool)
    .await?;

    Ok(user)
}

/// Update user traffic quota and used
pub async fn update_user_traffic(
    pool: &PgPool,
    user_id: i64,
    traffic_quota: i64,
    traffic_used: i64,
) -> Result<User> {
    let user = sqlx::query_as::<_, User>(
        r#"
        UPDATE users
        SET traffic_quota = $2, traffic_used = $3, updated_at = NOW()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(traffic_quota)
    .bind(traffic_used)
    .fetch_one(pool)
    .await?;

    Ok(user)
}

/// Update user status
pub async fn update_user_status(pool: &PgPool, user_id: i64, status: &str) -> Result<User> {
    let user = sqlx::query_as::<_, User>(
        r#"
        UPDATE users
        SET status = $2, updated_at = NOW()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(status)
    .fetch_one(pool)
    .await?;

    Ok(user)
}

/// List all users with pagination
pub async fn list_users(pool: &PgPool, limit: i64, offset: i64) -> Result<Vec<User>> {
    let users = sqlx::query_as::<_, User>(
        r#"
        SELECT * FROM users
        ORDER BY created_at DESC
        LIMIT $1 OFFSET $2
        "#,
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    Ok(users)
}

/// Count total users
pub async fn count_users(pool: &PgPool) -> Result<i64> {
    let count: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*) FROM users
        "#,
    )
    .fetch_one(pool)
    .await?;

    Ok(count.0)
}

/// Delete user (soft delete by setting status to disabled)
pub async fn delete_user(pool: &PgPool, user_id: i64) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE users
        SET status = 'disabled', updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(user_id)
    .execute(pool)
    .await?;

    Ok(())
}

// ============================================================================
// Package CRUD Operations
// ============================================================================

/// Create a new package
pub async fn create_package(
    pool: &PgPool,
    name: &str,
    traffic_amount: i64,
    price: i64,
    duration_days: i32,
    description: Option<&str>,
) -> Result<Package> {
    let package = sqlx::query_as::<_, Package>(
        r#"
        INSERT INTO packages (name, traffic_amount, price, duration_days, description)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING *
        "#,
    )
    .bind(name)
    .bind(traffic_amount)
    .bind(price)
    .bind(duration_days)
    .bind(description)
    .fetch_one(pool)
    .await?;

    Ok(package)
}

/// Get package by ID
pub async fn get_package_by_id(pool: &PgPool, package_id: i64) -> Result<Option<Package>> {
    let package = sqlx::query_as::<_, Package>(
        r#"
        SELECT * FROM packages WHERE id = $1
        "#,
    )
    .bind(package_id)
    .fetch_optional(pool)
    .await?;

    Ok(package)
}

/// List all active packages
pub async fn list_active_packages(pool: &PgPool) -> Result<Vec<Package>> {
    let packages = sqlx::query_as::<_, Package>(
        r#"
        SELECT * FROM packages
        WHERE is_active = true
        ORDER BY price ASC
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(packages)
}

/// List all packages (including inactive)
pub async fn list_all_packages(pool: &PgPool) -> Result<Vec<Package>> {
    let packages = sqlx::query_as::<_, Package>(
        r#"
        SELECT * FROM packages
        ORDER BY created_at DESC
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(packages)
}

/// Update package
pub async fn update_package(
    pool: &PgPool,
    package_id: i64,
    name: Option<&str>,
    traffic_amount: Option<i64>,
    price: Option<i64>,
    duration_days: Option<i32>,
    description: Option<&str>,
    is_active: Option<bool>,
) -> Result<Package> {
    // Build dynamic update query
    let mut query = String::from("UPDATE packages SET updated_at = NOW()");
    let mut bind_count = 1;

    if name.is_some() {
        query.push_str(&format!(", name = ${}", bind_count));
        bind_count += 1;
    }
    if traffic_amount.is_some() {
        query.push_str(&format!(", traffic_amount = ${}", bind_count));
        bind_count += 1;
    }
    if price.is_some() {
        query.push_str(&format!(", price = ${}", bind_count));
        bind_count += 1;
    }
    if duration_days.is_some() {
        query.push_str(&format!(", duration_days = ${}", bind_count));
        bind_count += 1;
    }
    if description.is_some() {
        query.push_str(&format!(", description = ${}", bind_count));
        bind_count += 1;
    }
    if is_active.is_some() {
        query.push_str(&format!(", is_active = ${}", bind_count));
        bind_count += 1;
    }

    query.push_str(&format!(" WHERE id = ${} RETURNING *", bind_count));

    let mut q = sqlx::query_as::<_, Package>(&query);

    if let Some(n) = name {
        q = q.bind(n);
    }
    if let Some(ta) = traffic_amount {
        q = q.bind(ta);
    }
    if let Some(p) = price {
        q = q.bind(p);
    }
    if let Some(dd) = duration_days {
        q = q.bind(dd);
    }
    if let Some(d) = description {
        q = q.bind(d);
    }
    if let Some(ia) = is_active {
        q = q.bind(ia);
    }

    q = q.bind(package_id);

    let package = q.fetch_one(pool).await?;

    Ok(package)
}

/// Delete package (soft delete by setting is_active to false)
pub async fn delete_package(pool: &PgPool, package_id: i64) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE packages
        SET is_active = false, updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(package_id)
    .execute(pool)
    .await?;

    Ok(())
}

// ============================================================================
// Order CRUD Operations
// ============================================================================

/// Create a new order
pub async fn create_order(
    pool: &PgPool,
    order_no: &str,
    user_id: i64,
    package_id: i64,
    amount: i64,
) -> Result<Order> {
    let order = sqlx::query_as::<_, Order>(
        r#"
        INSERT INTO orders (order_no, user_id, package_id, amount, status)
        VALUES ($1, $2, $3, $4, 'pending')
        RETURNING *
        "#,
    )
    .bind(order_no)
    .bind(user_id)
    .bind(package_id)
    .bind(amount)
    .fetch_one(pool)
    .await?;

    Ok(order)
}

/// Get order by ID
pub async fn get_order_by_id(pool: &PgPool, order_id: i64) -> Result<Option<Order>> {
    let order = sqlx::query_as::<_, Order>(
        r#"
        SELECT * FROM orders WHERE id = $1
        "#,
    )
    .bind(order_id)
    .fetch_optional(pool)
    .await?;

    Ok(order)
}

/// Get order by order number
pub async fn get_order_by_order_no(pool: &PgPool, order_no: &str) -> Result<Option<Order>> {
    let order = sqlx::query_as::<_, Order>(
        r#"
        SELECT * FROM orders WHERE order_no = $1
        "#,
    )
    .bind(order_no)
    .fetch_optional(pool)
    .await?;

    Ok(order)
}

/// List orders by user ID
pub async fn list_orders_by_user(
    pool: &PgPool,
    user_id: i64,
    limit: i64,
    offset: i64,
) -> Result<Vec<Order>> {
    let orders = sqlx::query_as::<_, Order>(
        r#"
        SELECT * FROM orders
        WHERE user_id = $1
        ORDER BY created_at DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(user_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    Ok(orders)
}

/// List all orders with pagination
pub async fn list_all_orders(pool: &PgPool, limit: i64, offset: i64) -> Result<Vec<Order>> {
    let orders = sqlx::query_as::<_, Order>(
        r#"
        SELECT * FROM orders
        ORDER BY created_at DESC
        LIMIT $1 OFFSET $2
        "#,
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    Ok(orders)
}

/// Update order status
pub async fn update_order_status(
    pool: &PgPool,
    order_id: i64,
    status: &str,
    completed_at: Option<DateTime<Utc>>,
) -> Result<Order> {
    let order = sqlx::query_as::<_, Order>(
        r#"
        UPDATE orders
        SET status = $2, completed_at = $3
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(order_id)
    .bind(status)
    .bind(completed_at)
    .fetch_one(pool)
    .await?;

    Ok(order)
}

/// Count orders by status
pub async fn count_orders_by_status(pool: &PgPool, status: &str) -> Result<i64> {
    let count: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*) FROM orders WHERE status = $1
        "#,
    )
    .bind(status)
    .fetch_one(pool)
    .await?;

    Ok(count.0)
}

/// Get total revenue (sum of completed orders)
pub async fn get_total_revenue(pool: &PgPool) -> Result<i64> {
    let revenue: (Option<i64>,) = sqlx::query_as(
        r#"
        SELECT COALESCE(SUM(amount), 0)::BIGINT FROM orders WHERE status = 'completed'
        "#,
    )
    .fetch_one(pool)
    .await?;

    Ok(revenue.0.unwrap_or(0))
}

// ============================================================================
// Node CRUD Operations
// ============================================================================

/// Create a new node
pub async fn create_node(
    pool: &PgPool,
    name: &str,
    host: &str,
    port: i32,
    protocol: &str,
    secret: &str,
    config: serde_json::Value,
) -> Result<Node> {
    let node = sqlx::query_as::<_, Node>(
        r#"
        INSERT INTO nodes (name, host, port, protocol, secret, config)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING *
        "#,
    )
    .bind(name)
    .bind(host)
    .bind(port)
    .bind(protocol)
    .bind(secret)
    .bind(config)
    .fetch_one(pool)
    .await?;

    Ok(node)
}

/// Get node by ID
pub async fn get_node_by_id(pool: &PgPool, node_id: i64) -> Result<Option<Node>> {
    let node = sqlx::query_as::<_, Node>(
        r#"
        SELECT * FROM nodes WHERE id = $1
        "#,
    )
    .bind(node_id)
    .fetch_optional(pool)
    .await?;

    Ok(node)
}

/// Get node by ID and secret (for authentication)
pub async fn get_node_by_id_and_secret(
    pool: &PgPool,
    node_id: i64,
    secret: &str,
) -> Result<Option<Node>> {
    let node = sqlx::query_as::<_, Node>(
        r#"
        SELECT * FROM nodes WHERE id = $1 AND secret = $2
        "#,
    )
    .bind(node_id)
    .bind(secret)
    .fetch_optional(pool)
    .await?;

    Ok(node)
}

/// List all nodes
pub async fn list_all_nodes(pool: &PgPool) -> Result<Vec<Node>> {
    let nodes = sqlx::query_as::<_, Node>(
        r#"
        SELECT * FROM nodes
        ORDER BY created_at DESC
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(nodes)
}

/// List nodes by status
pub async fn list_nodes_by_status(pool: &PgPool, status: &str) -> Result<Vec<Node>> {
    let nodes = sqlx::query_as::<_, Node>(
        r#"
        SELECT * FROM nodes
        WHERE status = $1
        ORDER BY created_at DESC
        "#,
    )
    .bind(status)
    .fetch_all(pool)
    .await?;

    Ok(nodes)
}

/// List nodes that should be included in Clash configuration
/// Filters by include_in_clash=true and orders by sort_order, then name
pub async fn list_clash_nodes(pool: &PgPool) -> Result<Vec<Node>> {
    let nodes = sqlx::query_as::<_, Node>(
        r#"
        SELECT * FROM nodes
        WHERE include_in_clash = true
        ORDER BY sort_order ASC, name ASC
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(nodes)
}

/// Update node
pub async fn update_node(
    pool: &PgPool,
    node_id: i64,
    name: Option<&str>,
    host: Option<&str>,
    port: Option<i32>,
    protocol: Option<&str>,
    config: Option<serde_json::Value>,
    status: Option<&str>,
    include_in_clash: Option<bool>,
    sort_order: Option<i32>,
) -> Result<Node> {
    // Build dynamic update query
    let mut query = String::from("UPDATE nodes SET updated_at = NOW()");
    let mut bind_count = 1;

    if name.is_some() {
        query.push_str(&format!(", name = ${}", bind_count));
        bind_count += 1;
    }
    if host.is_some() {
        query.push_str(&format!(", host = ${}", bind_count));
        bind_count += 1;
    }
    if port.is_some() {
        query.push_str(&format!(", port = ${}", bind_count));
        bind_count += 1;
    }
    if protocol.is_some() {
        query.push_str(&format!(", protocol = ${}", bind_count));
        bind_count += 1;
    }
    if config.is_some() {
        query.push_str(&format!(", config = ${}", bind_count));
        bind_count += 1;
    }
    if status.is_some() {
        query.push_str(&format!(", status = ${}", bind_count));
        bind_count += 1;
    }
    if include_in_clash.is_some() {
        query.push_str(&format!(", include_in_clash = ${}", bind_count));
        bind_count += 1;
    }
    if sort_order.is_some() {
        query.push_str(&format!(", sort_order = ${}", bind_count));
        bind_count += 1;
    }

    query.push_str(&format!(" WHERE id = ${} RETURNING *", bind_count));

    let mut q = sqlx::query_as::<_, Node>(&query);

    if let Some(n) = name {
        q = q.bind(n);
    }
    if let Some(h) = host {
        q = q.bind(h);
    }
    if let Some(p) = port {
        q = q.bind(p);
    }
    if let Some(pr) = protocol {
        q = q.bind(pr);
    }
    if let Some(c) = config {
        q = q.bind(c);
    }
    if let Some(s) = status {
        q = q.bind(s);
    }
    if let Some(ic) = include_in_clash {
        q = q.bind(ic);
    }
    if let Some(so) = sort_order {
        q = q.bind(so);
    }

    q = q.bind(node_id);

    let node = q.fetch_one(pool).await?;

    Ok(node)
}

/// Update node heartbeat
pub async fn update_node_heartbeat(
    pool: &PgPool,
    node_id: i64,
    status: &str,
    current_users: Option<i32>,
) -> Result<Node> {
    let node = sqlx::query_as::<_, Node>(
        r#"
        UPDATE nodes
        SET status = $2, current_users = COALESCE($3, current_users), 
            last_heartbeat = NOW(), updated_at = NOW()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(node_id)
    .bind(status)
    .bind(current_users)
    .fetch_one(pool)
    .await?;

    Ok(node)
}

/// Update node traffic statistics
pub async fn update_node_traffic(
    pool: &PgPool,
    node_id: i64,
    upload_delta: i64,
    download_delta: i64,
) -> Result<Node> {
    let node = sqlx::query_as::<_, Node>(
        r#"
        UPDATE nodes
        SET total_upload = total_upload + $2,
            total_download = total_download + $3,
            updated_at = NOW()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(node_id)
    .bind(upload_delta)
    .bind(download_delta)
    .fetch_one(pool)
    .await?;

    Ok(node)
}

/// Delete node
pub async fn delete_node(pool: &PgPool, node_id: i64) -> Result<()> {
    sqlx::query(
        r#"
        DELETE FROM nodes WHERE id = $1
        "#,
    )
    .bind(node_id)
    .execute(pool)
    .await?;

    Ok(())
}

/// Count nodes by status
pub async fn count_nodes_by_status(pool: &PgPool, status: &str) -> Result<i64> {
    let count: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*) FROM nodes WHERE status = $1
        "#,
    )
    .bind(status)
    .fetch_one(pool)
    .await?;

    Ok(count.0)
}

// ============================================================================
// Additional helper functions
// ============================================================================

/// Create a user package
pub async fn create_user_package(
    pool: &PgPool,
    user_id: i64,
    package_id: i64,
    order_id: i64,
    traffic_quota: i64,
    expires_at: DateTime<Utc>,
) -> Result<UserPackage> {
    let user_package = sqlx::query_as::<_, UserPackage>(
        r#"
        INSERT INTO user_packages (user_id, package_id, order_id, traffic_quota, expires_at)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(package_id)
    .bind(order_id)
    .bind(traffic_quota)
    .bind(expires_at)
    .fetch_one(pool)
    .await?;

    Ok(user_package)
}

/// Create a subscription
pub async fn create_subscription(pool: &PgPool, user_id: i64, token: &str) -> Result<Subscription> {
    let subscription = sqlx::query_as::<_, Subscription>(
        r#"
        INSERT INTO subscriptions (user_id, token)
        VALUES ($1, $2)
        ON CONFLICT (user_id) DO UPDATE SET token = $2
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(token)
    .fetch_one(pool)
    .await?;

    Ok(subscription)
}

/// Get subscription by token
pub async fn get_subscription_by_token(
    pool: &PgPool,
    token: &str,
) -> Result<Option<Subscription>> {
    let subscription = sqlx::query_as::<_, Subscription>(
        r#"
        SELECT * FROM subscriptions WHERE token = $1
        "#,
    )
    .bind(token)
    .fetch_optional(pool)
    .await?;

    Ok(subscription)
}

/// Create a coin transaction
pub async fn create_coin_transaction(
    pool: &PgPool,
    user_id: i64,
    amount: i64,
    transaction_type: &str,
    description: Option<&str>,
) -> Result<CoinTransaction> {
    let transaction = sqlx::query_as::<_, CoinTransaction>(
        r#"
        INSERT INTO coin_transactions (user_id, amount, type, description)
        VALUES ($1, $2, $3, $4)
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(amount)
    .bind(transaction_type)
    .bind(description)
    .fetch_one(pool)
    .await?;

    Ok(transaction)
}

/// Create an admin log
pub async fn create_admin_log(
    pool: &PgPool,
    admin_id: i64,
    action: &str,
    target_type: Option<&str>,
    target_id: Option<i64>,
    details: Option<serde_json::Value>,
) -> Result<AdminLog> {
    let log = sqlx::query_as::<_, AdminLog>(
        r#"
        INSERT INTO admin_logs (admin_id, action, target_type, target_id, details)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING *
        "#,
    )
    .bind(admin_id)
    .bind(action)
    .bind(target_type)
    .bind(target_id)
    .bind(details)
    .fetch_one(pool)
    .await?;

    Ok(log)
}

/// Create a traffic log
pub async fn create_traffic_log(
    pool: &PgPool,
    user_id: i64,
    node_id: i64,
    upload: i64,
    download: i64,
) -> Result<TrafficLog> {
    let log = sqlx::query_as::<_, TrafficLog>(
        r#"
        INSERT INTO traffic_logs (user_id, node_id, upload, download)
        VALUES ($1, $2, $3, $4)
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(node_id)
    .bind(upload)
    .bind(download)
    .fetch_one(pool)
    .await?;

    Ok(log)
}

/// Check if user has made any completed purchases
pub async fn has_user_made_purchase(pool: &PgPool, user_id: i64) -> Result<bool> {
    let count: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*) FROM orders
        WHERE user_id = $1 AND status = 'completed'
        "#,
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    Ok(count.0 > 0)
}

/// Process referral rebate for first purchase
/// Returns the referrer user if rebate was processed, None otherwise
pub async fn process_referral_rebate(
    pool: &PgPool,
    user_id: i64,
    purchase_amount: i64,
    rebate_percentage: f64,
) -> Result<Option<User>> {
    // Start a transaction
    let mut tx = pool.begin().await?;

    // Get the user to check if they were referred
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(&mut *tx)
        .await?;

    // Check if user was referred by someone
    let referrer_id = match user.referred_by {
        Some(id) => id,
        None => {
            // No referrer, nothing to do
            tx.rollback().await?;
            return Ok(None);
        }
    };

    // Prevent self-referral (should not happen, but double-check)
    if referrer_id == user_id {
        tx.rollback().await?;
        return Ok(None);
    }

    // Check if this is the user's first completed purchase
    let previous_purchases: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*) FROM orders
        WHERE user_id = $1 AND status = 'completed'
        "#,
    )
    .bind(user_id)
    .fetch_one(&mut *tx)
    .await?;

    if previous_purchases.0 > 0 {
        // Not the first purchase, no rebate
        tx.rollback().await?;
        return Ok(None);
    }

    // Calculate rebate amount (default 10% if not specified)
    let rebate_amount = (purchase_amount as f64 * rebate_percentage) as i64;

    if rebate_amount <= 0 {
        tx.rollback().await?;
        return Ok(None);
    }

    // Get referrer with row lock
    let referrer = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1 FOR UPDATE")
        .bind(referrer_id)
        .fetch_one(&mut *tx)
        .await?;

    // Add rebate to referrer's balance
    let new_balance = referrer.coin_balance + rebate_amount;
    let updated_referrer = sqlx::query_as::<_, User>(
        r#"
        UPDATE users
        SET coin_balance = $2, updated_at = NOW()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(referrer_id)
    .bind(new_balance)
    .fetch_one(&mut *tx)
    .await?;

    // Create coin transaction record for the rebate
    sqlx::query(
        r#"
        INSERT INTO coin_transactions (user_id, amount, type, description)
        VALUES ($1, $2, 'referral', $3)
        "#,
    )
    .bind(referrer_id)
    .bind(rebate_amount)
    .bind(format!("Referral rebate from user {}", user_id))
    .execute(&mut *tx)
    .await?;

    // Commit transaction
    tx.commit().await?;

    Ok(Some(updated_referrer))
}

/// Get referral statistics for a user
/// Returns (number of referrals, total rebate amount)
pub async fn get_referral_stats(pool: &PgPool, user_id: i64) -> Result<(i64, i64)> {
    // Count number of users referred by this user
    let referral_count: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*) FROM users
        WHERE referred_by = $1
        "#,
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    // Sum total rebate amount from coin transactions
    let total_rebate: (Option<i64>,) = sqlx::query_as(
        r#"
        SELECT COALESCE(SUM(amount), 0)::BIGINT FROM coin_transactions
        WHERE user_id = $1 AND type = 'referral'
        "#,
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    Ok((referral_count.0, total_rebate.0.unwrap_or(0)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires database to be running
    async fn test_create_pool() {
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://vpn_user:vpn_password@localhost/vpn_platform".to_string());

        let result = create_pool(&database_url).await;
        assert!(result.is_ok());
    }
}

// Include comprehensive database tests
#[cfg(test)]
#[path = "db_tests.rs"]
mod db_tests;

// ============================================================================
// Clash Configuration Management
// ============================================================================

/// Create a new Clash proxy
pub async fn create_clash_proxy(
    pool: &PgPool,
    name: &str,
    proxy_type: &str,
    server: &str,
    port: i32,
    config: &serde_json::Value,
    is_active: bool,
    sort_order: i32,
) -> Result<crate::models::ClashProxy> {
    let proxy = sqlx::query_as::<_, crate::models::ClashProxy>(
        r#"
        INSERT INTO clash_proxies (name, type, server, port, config, is_active, sort_order)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING *
        "#,
    )
    .bind(name)
    .bind(proxy_type)
    .bind(server)
    .bind(port)
    .bind(config)
    .bind(is_active)
    .bind(sort_order)
    .fetch_one(pool)
    .await?;

    Ok(proxy)
}

/// Get Clash proxy by ID
pub async fn get_clash_proxy_by_id(pool: &PgPool, proxy_id: i64) -> Result<Option<crate::models::ClashProxy>> {
    let proxy = sqlx::query_as::<_, crate::models::ClashProxy>(
        r#"
        SELECT * FROM clash_proxies WHERE id = $1
        "#,
    )
    .bind(proxy_id)
    .fetch_optional(pool)
    .await?;

    Ok(proxy)
}

/// List all Clash proxies
pub async fn list_clash_proxies(pool: &PgPool, active_only: bool) -> Result<Vec<crate::models::ClashProxy>> {
    let query = if active_only {
        "SELECT * FROM clash_proxies WHERE is_active = true ORDER BY sort_order, id"
    } else {
        "SELECT * FROM clash_proxies ORDER BY sort_order, id"
    };

    let proxies = sqlx::query_as::<_, crate::models::ClashProxy>(query)
        .fetch_all(pool)
        .await?;

    Ok(proxies)
}

/// Update Clash proxy
pub async fn update_clash_proxy(
    pool: &PgPool,
    proxy_id: i64,
    name: Option<&str>,
    proxy_type: Option<&str>,
    server: Option<&str>,
    port: Option<i32>,
    config: Option<&serde_json::Value>,
    is_active: Option<bool>,
    sort_order: Option<i32>,
) -> Result<crate::models::ClashProxy> {
    let mut query = String::from("UPDATE clash_proxies SET ");
    let mut updates = Vec::new();
    let mut param_count = 1;

    if name.is_some() {
        updates.push(format!("name = ${}", param_count));
        param_count += 1;
    }
    if proxy_type.is_some() {
        updates.push(format!("type = ${}", param_count));
        param_count += 1;
    }
    if server.is_some() {
        updates.push(format!("server = ${}", param_count));
        param_count += 1;
    }
    if port.is_some() {
        updates.push(format!("port = ${}", param_count));
        param_count += 1;
    }
    if config.is_some() {
        updates.push(format!("config = ${}", param_count));
        param_count += 1;
    }
    if is_active.is_some() {
        updates.push(format!("is_active = ${}", param_count));
        param_count += 1;
    }
    if sort_order.is_some() {
        updates.push(format!("sort_order = ${}", param_count));
        param_count += 1;
    }

    if updates.is_empty() {
        return get_clash_proxy_by_id(pool, proxy_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Proxy not found"));
    }

    query.push_str(&updates.join(", "));
    query.push_str(&format!(", updated_at = NOW() WHERE id = ${} RETURNING *", param_count));

    let mut q = sqlx::query_as::<_, crate::models::ClashProxy>(&query);

    if let Some(v) = name {
        q = q.bind(v);
    }
    if let Some(v) = proxy_type {
        q = q.bind(v);
    }
    if let Some(v) = server {
        q = q.bind(v);
    }
    if let Some(v) = port {
        q = q.bind(v);
    }
    if let Some(v) = config {
        q = q.bind(v);
    }
    if let Some(v) = is_active {
        q = q.bind(v);
    }
    if let Some(v) = sort_order {
        q = q.bind(v);
    }
    q = q.bind(proxy_id);

    let proxy = q.fetch_one(pool).await?;
    Ok(proxy)
}

/// Delete Clash proxy
pub async fn delete_clash_proxy(pool: &PgPool, proxy_id: i64) -> Result<()> {
    sqlx::query("DELETE FROM clash_proxies WHERE id = $1")
        .bind(proxy_id)
        .execute(pool)
        .await?;

    Ok(())
}

/// Create a new Clash proxy group
pub async fn create_clash_proxy_group(
    pool: &PgPool,
    name: &str,
    group_type: &str,
    proxies: &[String],
    url: Option<&str>,
    interval: Option<i32>,
    tolerance: Option<i32>,
    is_active: bool,
    sort_order: i32,
) -> Result<crate::models::ClashProxyGroup> {
    let group = sqlx::query_as::<_, crate::models::ClashProxyGroup>(
        r#"
        INSERT INTO clash_proxy_groups (name, type, proxies, url, interval, tolerance, is_active, sort_order)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING *
        "#,
    )
    .bind(name)
    .bind(group_type)
    .bind(proxies)
    .bind(url)
    .bind(interval)
    .bind(tolerance)
    .bind(is_active)
    .bind(sort_order)
    .fetch_one(pool)
    .await?;

    Ok(group)
}

/// Get Clash proxy group by ID
pub async fn get_clash_proxy_group_by_id(pool: &PgPool, group_id: i64) -> Result<Option<crate::models::ClashProxyGroup>> {
    let group = sqlx::query_as::<_, crate::models::ClashProxyGroup>(
        r#"
        SELECT * FROM clash_proxy_groups WHERE id = $1
        "#,
    )
    .bind(group_id)
    .fetch_optional(pool)
    .await?;

    Ok(group)
}

/// List all Clash proxy groups
pub async fn list_clash_proxy_groups(pool: &PgPool, active_only: bool) -> Result<Vec<crate::models::ClashProxyGroup>> {
    let query = if active_only {
        "SELECT * FROM clash_proxy_groups WHERE is_active = true ORDER BY sort_order, id"
    } else {
        "SELECT * FROM clash_proxy_groups ORDER BY sort_order, id"
    };

    let groups = sqlx::query_as::<_, crate::models::ClashProxyGroup>(query)
        .fetch_all(pool)
        .await?;

    Ok(groups)
}

/// Update Clash proxy group
pub async fn update_clash_proxy_group(
    pool: &PgPool,
    group_id: i64,
    name: Option<&str>,
    group_type: Option<&str>,
    proxies: Option<&[String]>,
    url: Option<Option<&str>>,
    interval: Option<Option<i32>>,
    tolerance: Option<Option<i32>>,
    is_active: Option<bool>,
    sort_order: Option<i32>,
) -> Result<crate::models::ClashProxyGroup> {
    let mut query = String::from("UPDATE clash_proxy_groups SET ");
    let mut updates = Vec::new();
    let mut param_count = 1;

    if name.is_some() {
        updates.push(format!("name = ${}", param_count));
        param_count += 1;
    }
    if group_type.is_some() {
        updates.push(format!("type = ${}", param_count));
        param_count += 1;
    }
    if proxies.is_some() {
        updates.push(format!("proxies = ${}", param_count));
        param_count += 1;
    }
    if url.is_some() {
        updates.push(format!("url = ${}", param_count));
        param_count += 1;
    }
    if interval.is_some() {
        updates.push(format!("interval = ${}", param_count));
        param_count += 1;
    }
    if tolerance.is_some() {
        updates.push(format!("tolerance = ${}", param_count));
        param_count += 1;
    }
    if is_active.is_some() {
        updates.push(format!("is_active = ${}", param_count));
        param_count += 1;
    }
    if sort_order.is_some() {
        updates.push(format!("sort_order = ${}", param_count));
        param_count += 1;
    }

    if updates.is_empty() {
        return get_clash_proxy_group_by_id(pool, group_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Proxy group not found"));
    }

    query.push_str(&updates.join(", "));
    query.push_str(&format!(", updated_at = NOW() WHERE id = ${} RETURNING *", param_count));

    let mut q = sqlx::query_as::<_, crate::models::ClashProxyGroup>(&query);

    if let Some(v) = name {
        q = q.bind(v);
    }
    if let Some(v) = group_type {
        q = q.bind(v);
    }
    if let Some(v) = proxies {
        q = q.bind(v);
    }
    if let Some(v) = url {
        q = q.bind(v);
    }
    if let Some(v) = interval {
        q = q.bind(v);
    }
    if let Some(v) = tolerance {
        q = q.bind(v);
    }
    if let Some(v) = is_active {
        q = q.bind(v);
    }
    if let Some(v) = sort_order {
        q = q.bind(v);
    }
    q = q.bind(group_id);

    let group = q.fetch_one(pool).await?;
    Ok(group)
}

/// Delete Clash proxy group
pub async fn delete_clash_proxy_group(pool: &PgPool, group_id: i64) -> Result<()> {
    sqlx::query("DELETE FROM clash_proxy_groups WHERE id = $1")
        .bind(group_id)
        .execute(pool)
        .await?;

    Ok(())
}

/// Create a new Clash rule
pub async fn create_clash_rule(
    pool: &PgPool,
    rule_type: &str,
    rule_value: Option<&str>,
    proxy_group: &str,
    no_resolve: bool,
    is_active: bool,
    sort_order: i32,
    description: Option<&str>,
) -> Result<crate::models::ClashRule> {
    let rule = sqlx::query_as::<_, crate::models::ClashRule>(
        r#"
        INSERT INTO clash_rules (rule_type, rule_value, proxy_group, no_resolve, is_active, sort_order, description)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING *
        "#,
    )
    .bind(rule_type)
    .bind(rule_value)
    .bind(proxy_group)
    .bind(no_resolve)
    .bind(is_active)
    .bind(sort_order)
    .bind(description)
    .fetch_one(pool)
    .await?;

    Ok(rule)
}

/// Get Clash rule by ID
pub async fn get_clash_rule_by_id(pool: &PgPool, rule_id: i64) -> Result<Option<crate::models::ClashRule>> {
    let rule = sqlx::query_as::<_, crate::models::ClashRule>(
        r#"
        SELECT * FROM clash_rules WHERE id = $1
        "#,
    )
    .bind(rule_id)
    .fetch_optional(pool)
    .await?;

    Ok(rule)
}

/// List all Clash rules
pub async fn list_clash_rules(pool: &PgPool, active_only: bool) -> Result<Vec<crate::models::ClashRule>> {
    let query = if active_only {
        "SELECT * FROM clash_rules WHERE is_active = true ORDER BY sort_order, id"
    } else {
        "SELECT * FROM clash_rules ORDER BY sort_order, id"
    };

    let rules = sqlx::query_as::<_, crate::models::ClashRule>(query)
        .fetch_all(pool)
        .await?;

    Ok(rules)
}

/// Update Clash rule
pub async fn update_clash_rule(
    pool: &PgPool,
    rule_id: i64,
    rule_type: Option<&str>,
    rule_value: Option<Option<&str>>,
    proxy_group: Option<&str>,
    no_resolve: Option<bool>,
    is_active: Option<bool>,
    sort_order: Option<i32>,
    description: Option<Option<&str>>,
) -> Result<crate::models::ClashRule> {
    let mut query = String::from("UPDATE clash_rules SET ");
    let mut updates = Vec::new();
    let mut param_count = 1;

    if rule_type.is_some() {
        updates.push(format!("rule_type = ${}", param_count));
        param_count += 1;
    }
    if rule_value.is_some() {
        updates.push(format!("rule_value = ${}", param_count));
        param_count += 1;
    }
    if proxy_group.is_some() {
        updates.push(format!("proxy_group = ${}", param_count));
        param_count += 1;
    }
    if no_resolve.is_some() {
        updates.push(format!("no_resolve = ${}", param_count));
        param_count += 1;
    }
    if is_active.is_some() {
        updates.push(format!("is_active = ${}", param_count));
        param_count += 1;
    }
    if sort_order.is_some() {
        updates.push(format!("sort_order = ${}", param_count));
        param_count += 1;
    }
    if description.is_some() {
        updates.push(format!("description = ${}", param_count));
        param_count += 1;
    }

    if updates.is_empty() {
        return get_clash_rule_by_id(pool, rule_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Rule not found"));
    }

    query.push_str(&updates.join(", "));
    query.push_str(&format!(", updated_at = NOW() WHERE id = ${} RETURNING *", param_count));

    let mut q = sqlx::query_as::<_, crate::models::ClashRule>(&query);

    if let Some(v) = rule_type {
        q = q.bind(v);
    }
    if let Some(v) = rule_value {
        q = q.bind(v);
    }
    if let Some(v) = proxy_group {
        q = q.bind(v);
    }
    if let Some(v) = no_resolve {
        q = q.bind(v);
    }
    if let Some(v) = is_active {
        q = q.bind(v);
    }
    if let Some(v) = sort_order {
        q = q.bind(v);
    }
    if let Some(v) = description {
        q = q.bind(v);
    }
    q = q.bind(rule_id);

    let rule = q.fetch_one(pool).await?;
    Ok(rule)
}

/// Delete Clash rule
pub async fn delete_clash_rule(pool: &PgPool, rule_id: i64) -> Result<()> {
    sqlx::query("DELETE FROM clash_rules WHERE id = $1")
        .bind(rule_id)
        .execute(pool)
        .await?;

    Ok(())
}

// ============================================================================
// Clash Access Logs
// ============================================================================

/// Create a new access log entry
pub async fn create_access_log(
    pool: &PgPool,
    user_id: i64,
    subscription_token: &str,
    ip_address: &str,
    user_agent: Option<&str>,
    response_status: &str,
) -> Result<crate::models::ClashAccessLog> {
    let log = sqlx::query_as::<_, crate::models::ClashAccessLog>(
        r#"
        INSERT INTO clash_access_logs 
        (user_id, subscription_token, access_timestamp, ip_address, user_agent, response_status)
        VALUES ($1, $2, NOW(), $3, $4, $5)
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(subscription_token)
    .bind(ip_address)
    .bind(user_agent)
    .bind(response_status)
    .fetch_one(pool)
    .await?;

    Ok(log)
}

/// Query access logs with filters and pagination
pub async fn query_access_logs(
    pool: &PgPool,
    user_id: Option<i64>,
    start_date: Option<DateTime<Utc>>,
    end_date: Option<DateTime<Utc>>,
    status: Option<&str>,
    page: i64,
    page_size: i64,
) -> Result<(Vec<crate::models::AccessLogResponse>, i64)> {
    let offset = (page - 1) * page_size;
    
    // Build dynamic query based on filters
    let mut query = String::from(
        r#"
        SELECT 
            cal.id,
            cal.user_id,
            u.email as user_email,
            cal.subscription_token,
            cal.access_timestamp,
            cal.ip_address,
            cal.user_agent,
            cal.response_status
        FROM clash_access_logs cal
        INNER JOIN users u ON cal.user_id = u.id
        WHERE 1=1
        "#
    );
    
    let mut count_query = String::from(
        r#"
        SELECT COUNT(*) as count
        FROM clash_access_logs cal
        WHERE 1=1
        "#
    );
    
    let mut bind_params: Vec<String> = Vec::new();
    let mut param_count = 1;
    
    // Add filters dynamically
    if user_id.is_some() {
        query.push_str(&format!(" AND cal.user_id = ${}", param_count));
        count_query.push_str(&format!(" AND cal.user_id = ${}", param_count));
        bind_params.push("user_id".to_string());
        param_count += 1;
    }
    
    if start_date.is_some() {
        query.push_str(&format!(" AND cal.access_timestamp >= ${}", param_count));
        count_query.push_str(&format!(" AND cal.access_timestamp >= ${}", param_count));
        bind_params.push("start_date".to_string());
        param_count += 1;
    }
    
    if end_date.is_some() {
        query.push_str(&format!(" AND cal.access_timestamp <= ${}", param_count));
        count_query.push_str(&format!(" AND cal.access_timestamp <= ${}", param_count));
        bind_params.push("end_date".to_string());
        param_count += 1;
    }
    
    if status.is_some() {
        query.push_str(&format!(" AND cal.response_status = ${}", param_count));
        count_query.push_str(&format!(" AND cal.response_status = ${}", param_count));
        bind_params.push("status".to_string());
        param_count += 1;
    }
    
    query.push_str(&format!(" ORDER BY cal.access_timestamp DESC LIMIT ${} OFFSET ${}", param_count, param_count + 1));
    
    // Build queries with proper parameter binding
    let mut logs_query = sqlx::query_as::<_, crate::models::AccessLogResponse>(&query);
    let mut count_query_exec = sqlx::query_scalar::<_, i64>(&count_query);
    
    // Bind parameters in order
    for param in &bind_params {
        match param.as_str() {
            "user_id" => {
                if let Some(uid) = user_id {
                    logs_query = logs_query.bind(uid);
                    count_query_exec = count_query_exec.bind(uid);
                }
            }
            "start_date" => {
                if let Some(sd) = start_date {
                    logs_query = logs_query.bind(sd);
                    count_query_exec = count_query_exec.bind(sd);
                }
            }
            "end_date" => {
                if let Some(ed) = end_date {
                    logs_query = logs_query.bind(ed);
                    count_query_exec = count_query_exec.bind(ed);
                }
            }
            "status" => {
                if let Some(s) = status {
                    logs_query = logs_query.bind(s);
                    count_query_exec = count_query_exec.bind(s);
                }
            }
            _ => {}
        }
    }
    
    // Bind pagination parameters
    logs_query = logs_query.bind(page_size).bind(offset);
    
    // Execute queries
    let logs = logs_query.fetch_all(pool).await?;
    let total = count_query_exec.fetch_one(pool).await?;
    
    Ok((logs, total))
}
