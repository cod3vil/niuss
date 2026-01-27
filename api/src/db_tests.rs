#[cfg(test)]
mod tests {
    use crate::db::*;
    use chrono::Utc;
    use sqlx::PgPool;

    // Helper function to get test database pool
    async fn get_test_pool() -> PgPool {
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://vpn_user:vpn_password@localhost/vpn_platform_test".to_string());
        
        create_pool(&database_url).await.expect("Failed to create test pool")
    }

    // Helper function to clean up test data
    async fn cleanup_test_data(pool: &PgPool) {
        // Delete in reverse order of dependencies
        let _ = sqlx::query("DELETE FROM admin_logs").execute(pool).await;
        let _ = sqlx::query("DELETE FROM coin_transactions").execute(pool).await;
        let _ = sqlx::query("DELETE FROM traffic_logs").execute(pool).await;
        let _ = sqlx::query("DELETE FROM subscriptions").execute(pool).await;
        let _ = sqlx::query("DELETE FROM user_packages").execute(pool).await;
        let _ = sqlx::query("DELETE FROM orders").execute(pool).await;
        let _ = sqlx::query("DELETE FROM nodes").execute(pool).await;
        let _ = sqlx::query("DELETE FROM packages WHERE name LIKE 'Test%'").execute(pool).await;
        let _ = sqlx::query("DELETE FROM users WHERE email LIKE 'test%'").execute(pool).await;
    }

    #[tokio::test]
    #[ignore] // Requires database to be running
    async fn test_user_crud_operations() {
        let pool = get_test_pool().await;
        cleanup_test_data(&pool).await;

        // Test create user
        let user = create_user(
            &pool,
            "test_user@example.com",
            "hashed_password",
            Some("TESTREF123"),
            None,
        )
        .await
        .expect("Failed to create user");

        assert_eq!(user.email, "test_user@example.com");
        assert_eq!(user.referral_code, Some("TESTREF123".to_string()));
        assert_eq!(user.coin_balance, 0);
        assert_eq!(user.status, "active");

        // Test get user by ID
        let fetched_user = get_user_by_id(&pool, user.id)
            .await
            .expect("Failed to get user by ID")
            .expect("User not found");
        assert_eq!(fetched_user.id, user.id);
        assert_eq!(fetched_user.email, user.email);

        // Test get user by email
        let fetched_user = get_user_by_email(&pool, "test_user@example.com")
            .await
            .expect("Failed to get user by email")
            .expect("User not found");
        assert_eq!(fetched_user.id, user.id);

        // Test get user by referral code
        let fetched_user = get_user_by_referral_code(&pool, "TESTREF123")
            .await
            .expect("Failed to get user by referral code")
            .expect("User not found");
        assert_eq!(fetched_user.id, user.id);

        // Test update user coin balance
        let updated_user = update_user_coin_balance(&pool, user.id, 1000)
            .await
            .expect("Failed to update coin balance");
        assert_eq!(updated_user.coin_balance, 1000);

        // Test update user traffic
        let updated_user = update_user_traffic(&pool, user.id, 10737418240, 1073741824)
            .await
            .expect("Failed to update traffic");
        assert_eq!(updated_user.traffic_quota, 10737418240);
        assert_eq!(updated_user.traffic_used, 1073741824);

        // Test update user status
        let updated_user = update_user_status(&pool, user.id, "disabled")
            .await
            .expect("Failed to update status");
        assert_eq!(updated_user.status, "disabled");

        // Test list users
        let users = list_users(&pool, 10, 0)
            .await
            .expect("Failed to list users");
        assert!(users.len() > 0);

        // Test count users
        let count = count_users(&pool)
            .await
            .expect("Failed to count users");
        assert!(count > 0);

        cleanup_test_data(&pool).await;
    }

    #[tokio::test]
    #[ignore] // Requires database to be running
    async fn test_package_crud_operations() {
        let pool = get_test_pool().await;
        cleanup_test_data(&pool).await;

        // Test create package
        let package = create_package(
            &pool,
            "Test Package",
            10737418240, // 10GB
            500,
            30,
            Some("Test package description"),
        )
        .await
        .expect("Failed to create package");

        assert_eq!(package.name, "Test Package");
        assert_eq!(package.traffic_amount, 10737418240);
        assert_eq!(package.price, 500);
        assert_eq!(package.duration_days, 30);
        assert!(package.is_active);

        // Test get package by ID
        let fetched_package = get_package_by_id(&pool, package.id)
            .await
            .expect("Failed to get package by ID")
            .expect("Package not found");
        assert_eq!(fetched_package.id, package.id);

        // Test list active packages
        let packages = list_active_packages(&pool)
            .await
            .expect("Failed to list active packages");
        assert!(packages.len() > 0);

        // Test update package
        let updated_package = update_package(
            &pool,
            package.id,
            Some("Updated Test Package"),
            Some(21474836480), // 20GB
            Some(900),
            None,
            None,
            Some(false),
        )
        .await
        .expect("Failed to update package");
        assert_eq!(updated_package.name, "Updated Test Package");
        assert_eq!(updated_package.traffic_amount, 21474836480);
        assert_eq!(updated_package.price, 900);
        assert!(!updated_package.is_active);

        // Test list all packages (including inactive)
        let all_packages = list_all_packages(&pool)
            .await
            .expect("Failed to list all packages");
        assert!(all_packages.len() > 0);

        cleanup_test_data(&pool).await;
    }

    #[tokio::test]
    #[ignore] // Requires database to be running
    async fn test_order_crud_operations() {
        let pool = get_test_pool().await;
        cleanup_test_data(&pool).await;

        // Create test user and package first
        let user = create_user(&pool, "test_order@example.com", "hash", None, None)
            .await
            .expect("Failed to create user");

        let package = create_package(&pool, "Test Order Package", 10737418240, 500, 30, None)
            .await
            .expect("Failed to create package");

        // Test create order
        let order = create_order(&pool, "ORDER123456", user.id, package.id, 500)
            .await
            .expect("Failed to create order");

        assert_eq!(order.order_no, "ORDER123456");
        assert_eq!(order.user_id, user.id);
        assert_eq!(order.package_id, package.id);
        assert_eq!(order.amount, 500);
        assert_eq!(order.status, "pending");

        // Test get order by ID
        let fetched_order = get_order_by_id(&pool, order.id)
            .await
            .expect("Failed to get order by ID")
            .expect("Order not found");
        assert_eq!(fetched_order.id, order.id);

        // Test get order by order number
        let fetched_order = get_order_by_order_no(&pool, "ORDER123456")
            .await
            .expect("Failed to get order by order_no")
            .expect("Order not found");
        assert_eq!(fetched_order.id, order.id);

        // Test list orders by user
        let orders = list_orders_by_user(&pool, user.id, 10, 0)
            .await
            .expect("Failed to list orders by user");
        assert_eq!(orders.len(), 1);

        // Test update order status
        let updated_order = update_order_status(&pool, order.id, "completed", Some(Utc::now()))
            .await
            .expect("Failed to update order status");
        assert_eq!(updated_order.status, "completed");
        assert!(updated_order.completed_at.is_some());

        // Test count orders by status
        let count = count_orders_by_status(&pool, "completed")
            .await
            .expect("Failed to count orders");
        assert!(count > 0);

        // Test get total revenue
        let revenue = get_total_revenue(&pool)
            .await
            .expect("Failed to get total revenue");
        assert!(revenue >= 500);

        cleanup_test_data(&pool).await;
    }

    #[tokio::test]
    #[ignore] // Requires database to be running
    async fn test_node_crud_operations() {
        let pool = get_test_pool().await;
        cleanup_test_data(&pool).await;

        // Test create node
        let config = serde_json::json!({
            "flow": "xtls-rprx-vision",
            "encryption": "none",
            "reality": {
                "dest": "www.microsoft.com:443",
                "serverNames": ["www.microsoft.com"],
                "publicKey": "test_public_key",
                "privateKey": "test_private_key",
                "shortIds": [""]
            }
        });

        let node = create_node(
            &pool,
            "Test Node",
            "example.com",
            443,
            "vless",
            "secret_key_123",
            config.clone(),
        )
        .await
        .expect("Failed to create node");

        assert_eq!(node.name, "Test Node");
        assert_eq!(node.host, "example.com");
        assert_eq!(node.port, 443);
        assert_eq!(node.protocol, "vless");
        assert_eq!(node.status, "offline");

        // Test get node by ID
        let fetched_node = get_node_by_id(&pool, node.id)
            .await
            .expect("Failed to get node by ID")
            .expect("Node not found");
        assert_eq!(fetched_node.id, node.id);

        // Test get node by ID and secret
        let fetched_node = get_node_by_id_and_secret(&pool, node.id, "secret_key_123")
            .await
            .expect("Failed to get node by ID and secret")
            .expect("Node not found");
        assert_eq!(fetched_node.id, node.id);

        // Test invalid secret
        let result = get_node_by_id_and_secret(&pool, node.id, "wrong_secret")
            .await
            .expect("Failed to query node");
        assert!(result.is_none());

        // Test list all nodes
        let nodes = list_all_nodes(&pool)
            .await
            .expect("Failed to list all nodes");
        assert!(nodes.len() > 0);

        // Test update node
        let updated_node = update_node(
            &pool,
            node.id,
            Some("Updated Test Node"),
            None,
            Some(8443),
            None,
            None,
            Some("online"),
        )
        .await
        .expect("Failed to update node");
        assert_eq!(updated_node.name, "Updated Test Node");
        assert_eq!(updated_node.port, 8443);
        assert_eq!(updated_node.status, "online");

        // Test list nodes by status
        let online_nodes = list_nodes_by_status(&pool, "online")
            .await
            .expect("Failed to list nodes by status");
        assert!(online_nodes.len() > 0);

        // Test update node heartbeat
        let updated_node = update_node_heartbeat(&pool, node.id, "online", Some(50))
            .await
            .expect("Failed to update node heartbeat");
        assert_eq!(updated_node.status, "online");
        assert_eq!(updated_node.current_users, 50);
        assert!(updated_node.last_heartbeat.is_some());

        // Test update node traffic
        let updated_node = update_node_traffic(&pool, node.id, 1073741824, 2147483648)
            .await
            .expect("Failed to update node traffic");
        assert_eq!(updated_node.total_upload, 1073741824);
        assert_eq!(updated_node.total_download, 2147483648);

        // Test count nodes by status
        let count = count_nodes_by_status(&pool, "online")
            .await
            .expect("Failed to count nodes");
        assert!(count > 0);

        // Test delete node
        delete_node(&pool, node.id)
            .await
            .expect("Failed to delete node");

        let result = get_node_by_id(&pool, node.id)
            .await
            .expect("Failed to query node");
        assert!(result.is_none());

        cleanup_test_data(&pool).await;
    }

    #[tokio::test]
    #[ignore] // Requires database to be running
    async fn test_transaction_rollback() {
        let pool = get_test_pool().await;
        cleanup_test_data(&pool).await;

        // Create a user
        let user = create_user(&pool, "test_transaction@example.com", "hash", None, None)
            .await
            .expect("Failed to create user");

        // Start a transaction
        let mut tx = pool.begin().await.expect("Failed to begin transaction");

        // Update user balance within transaction
        let _ = sqlx::query(
            "UPDATE users SET coin_balance = 1000 WHERE id = $1"
        )
        .bind(user.id)
        .execute(&mut *tx)
        .await
        .expect("Failed to update balance");

        // Rollback the transaction
        tx.rollback().await.expect("Failed to rollback");

        // Verify balance was not changed
        let fetched_user = get_user_by_id(&pool, user.id)
            .await
            .expect("Failed to get user")
            .expect("User not found");
        assert_eq!(fetched_user.coin_balance, 0);

        cleanup_test_data(&pool).await;
    }

    #[tokio::test]
    #[ignore] // Requires database to be running
    async fn test_constraint_validation() {
        let pool = get_test_pool().await;
        cleanup_test_data(&pool).await;

        // Test duplicate email constraint
        let _ = create_user(&pool, "test_constraint@example.com", "hash", None, None)
            .await
            .expect("Failed to create first user");

        let result = create_user(&pool, "test_constraint@example.com", "hash2", None, None)
            .await;
        assert!(result.is_err(), "Should fail on duplicate email");

        // Test duplicate referral code constraint
        let _ = create_user(&pool, "test_ref1@example.com", "hash", Some("DUPREF"), None)
            .await
            .expect("Failed to create user with referral code");

        let result = create_user(&pool, "test_ref2@example.com", "hash", Some("DUPREF"), None)
            .await;
        assert!(result.is_err(), "Should fail on duplicate referral code");

        // Test duplicate order number constraint
        let user = create_user(&pool, "test_order_dup@example.com", "hash", None, None)
            .await
            .expect("Failed to create user");

        let package = create_package(&pool, "Test Package", 10737418240, 500, 30, None)
            .await
            .expect("Failed to create package");

        let _ = create_order(&pool, "DUPORDER123", user.id, package.id, 500)
            .await
            .expect("Failed to create first order");

        let result = create_order(&pool, "DUPORDER123", user.id, package.id, 500)
            .await;
        assert!(result.is_err(), "Should fail on duplicate order number");

        cleanup_test_data(&pool).await;
    }

    #[tokio::test]
    #[ignore] // Requires database to be running
    async fn test_helper_functions() {
        let pool = get_test_pool().await;
        cleanup_test_data(&pool).await;

        // Create test data
        let user = create_user(&pool, "test_helper@example.com", "hash", None, None)
            .await
            .expect("Failed to create user");

        let package = create_package(&pool, "Test Helper Package", 10737418240, 500, 30, None)
            .await
            .expect("Failed to create package");

        let order = create_order(&pool, "HELPER123", user.id, package.id, 500)
            .await
            .expect("Failed to create order");

        let node = create_node(
            &pool,
            "Test Helper Node",
            "example.com",
            443,
            "vless",
            "secret",
            serde_json::json!({}),
        )
        .await
        .expect("Failed to create node");

        // Test create user package
        let expires_at = Utc::now() + chrono::Duration::days(30);
        let user_package = create_user_package(
            &pool,
            user.id,
            package.id,
            order.id,
            10737418240,
            expires_at,
        )
        .await
        .expect("Failed to create user package");
        assert_eq!(user_package.user_id, user.id);
        assert_eq!(user_package.traffic_quota, 10737418240);

        // Test create subscription
        let subscription = create_subscription(&pool, user.id, "test_token_123")
            .await
            .expect("Failed to create subscription");
        assert_eq!(subscription.user_id, user.id);
        assert_eq!(subscription.token, "test_token_123");

        // Test get subscription by token
        let fetched_sub = get_subscription_by_token(&pool, "test_token_123")
            .await
            .expect("Failed to get subscription")
            .expect("Subscription not found");
        assert_eq!(fetched_sub.user_id, user.id);

        // Test create coin transaction
        let coin_tx = create_coin_transaction(
            &pool,
            user.id,
            1000,
            "recharge",
            Some("Test recharge"),
        )
        .await
        .expect("Failed to create coin transaction");
        assert_eq!(coin_tx.user_id, user.id);
        assert_eq!(coin_tx.amount, 1000);
        assert_eq!(coin_tx.transaction_type, "recharge");

        // Test create admin log
        let admin_log = create_admin_log(
            &pool,
            user.id,
            "test_action",
            Some("user"),
            Some(user.id),
            Some(serde_json::json!({"key": "value"})),
        )
        .await
        .expect("Failed to create admin log");
        assert_eq!(admin_log.admin_id, user.id);
        assert_eq!(admin_log.action, "test_action");

        // Test create traffic log
        let traffic_log = create_traffic_log(&pool, user.id, node.id, 1073741824, 2147483648)
            .await
            .expect("Failed to create traffic log");
        assert_eq!(traffic_log.user_id, user.id);
        assert_eq!(traffic_log.node_id, node.id);
        assert_eq!(traffic_log.upload, 1073741824);
        assert_eq!(traffic_log.download, 2147483648);

        cleanup_test_data(&pool).await;
    }
}
