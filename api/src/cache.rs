use anyhow::{Context, Result};
use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};

use crate::models::Node;

/// User package cache data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPackageCache {
    pub traffic_quota: i64,
    pub traffic_used: i64,
    pub expires_at: String,
    pub status: String,
}

/// Redis cache utility functions
#[derive(Clone)]
pub struct RedisCache {
    conn: ConnectionManager,
}

impl RedisCache {
    /// Create a new RedisCache instance
    pub fn new(conn: ConnectionManager) -> Self {
        Self { conn }
    }

    /// Get the Redis connection manager
    pub fn connection(&self) -> ConnectionManager {
        self.conn.clone()
    }

    // ========================================================================
    // User Package Cache Operations
    // ========================================================================

    /// Cache user package data
    /// TTL: 300 seconds (5 minutes)
    pub async fn cache_user_package(
        &self,
        user_id: i64,
        package_data: &UserPackageCache,
    ) -> Result<()> {
        let key = format!("user:package:{}", user_id);
        let json = serde_json::to_string(package_data)
            .context("Failed to serialize user package data")?;

        let mut conn = self.conn.clone();
        conn.set_ex(&key, json, 300)
            .await
            .context("Failed to cache user package data")?;

        Ok(())
    }

    /// Get cached user package data
    pub async fn get_user_package(&self, user_id: i64) -> Result<Option<UserPackageCache>> {
        let key = format!("user:package:{}", user_id);
        let mut conn = self.conn.clone();

        let json: Option<String> = conn
            .get(&key)
            .await
            .context("Failed to get user package from cache")?;

        match json {
            Some(data) => {
                let package = serde_json::from_str(&data)
                    .context("Failed to deserialize user package data")?;
                Ok(Some(package))
            }
            None => Ok(None),
        }
    }

    /// Invalidate user package cache
    pub async fn invalidate_user_package(&self, user_id: i64) -> Result<()> {
        let key = format!("user:package:{}", user_id);
        let mut conn = self.conn.clone();

        conn.del(&key)
            .await
            .context("Failed to invalidate user package cache")?;

        Ok(())
    }

    // ========================================================================
    // Node List Cache Operations
    // ========================================================================

    /// Cache active nodes list
    /// TTL: 60 seconds (1 minute)
    pub async fn cache_active_nodes(&self, nodes: &[Node]) -> Result<()> {
        let key = "nodes:active";
        let json = serde_json::to_string(nodes)
            .context("Failed to serialize nodes list")?;

        let mut conn = self.conn.clone();
        conn.set_ex(key, json, 60)
            .await
            .context("Failed to cache active nodes")?;

        Ok(())
    }

    /// Get cached active nodes list
    pub async fn get_active_nodes(&self) -> Result<Option<Vec<Node>>> {
        let key = "nodes:active";
        let mut conn = self.conn.clone();

        let json: Option<String> = conn
            .get(key)
            .await
            .context("Failed to get active nodes from cache")?;

        match json {
            Some(data) => {
                let nodes = serde_json::from_str(&data)
                    .context("Failed to deserialize nodes list")?;
                Ok(Some(nodes))
            }
            None => Ok(None),
        }
    }

    /// Invalidate active nodes cache
    pub async fn invalidate_active_nodes(&self) -> Result<()> {
        let key = "nodes:active";
        let mut conn = self.conn.clone();

        conn.del(key)
            .await
            .context("Failed to invalidate active nodes cache")?;

        Ok(())
    }

    // ========================================================================
    // Subscription Configuration Cache Operations
    // ========================================================================

    /// Cache subscription configuration (Clash YAML)
    /// TTL: 300 seconds (5 minutes)
    pub async fn cache_subscription_config(&self, token: &str, config: &str) -> Result<()> {
        let key = format!("subscription:{}", token);
        let mut conn = self.conn.clone();

        conn.set_ex(&key, config, 300)
            .await
            .context("Failed to cache subscription config")?;

        Ok(())
    }

    /// Get cached subscription configuration
    pub async fn get_subscription_config(&self, token: &str) -> Result<Option<String>> {
        let key = format!("subscription:{}", token);
        let mut conn = self.conn.clone();

        let config: Option<String> = conn
            .get(&key)
            .await
            .context("Failed to get subscription config from cache")?;

        Ok(config)
    }

    /// Invalidate subscription configuration cache
    pub async fn invalidate_subscription_config(&self, token: &str) -> Result<()> {
        let key = format!("subscription:{}", token);
        let mut conn = self.conn.clone();

        conn.del(&key)
            .await
            .context("Failed to invalidate subscription config cache")?;

        Ok(())
    }

    // ========================================================================
    // Node Configuration Update Notification (Redis Pub/Sub)
    // ========================================================================

    /// Publish node configuration update notification
    pub async fn publish_node_config_update(&self, node_id: i64) -> Result<()> {
        let channel = "node:config:update";
        let message = serde_json::json!({
            "node_id": node_id,
            "action": "reload",
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });

        let mut conn = self.conn.clone();
        conn.publish(channel, message.to_string())
            .await
            .context("Failed to publish node config update")?;

        Ok(())
    }

    // ========================================================================
    // Generic Cache Operations
    // ========================================================================

    /// Set a key-value pair with TTL
    pub async fn set_with_ttl(&self, key: &str, value: &str, ttl_seconds: u64) -> Result<()> {
        let mut conn = self.conn.clone();
        conn.set_ex(key, value, ttl_seconds)
            .await
            .context("Failed to set cache value")?;

        Ok(())
    }

    /// Get a value by key
    pub async fn get(&self, key: &str) -> Result<Option<String>> {
        let mut conn = self.conn.clone();
        let value: Option<String> = conn
            .get(key)
            .await
            .context("Failed to get cache value")?;

        Ok(value)
    }

    /// Delete a key
    pub async fn delete(&self, key: &str) -> Result<()> {
        let mut conn = self.conn.clone();
        conn.del(key)
            .await
            .context("Failed to delete cache key")?;

        Ok(())
    }

    /// Check if a key exists
    pub async fn exists(&self, key: &str) -> Result<bool> {
        let mut conn = self.conn.clone();
        let exists: bool = conn
            .exists(key)
            .await
            .context("Failed to check key existence")?;

        Ok(exists)
    }

    /// Set TTL for an existing key
    pub async fn expire(&self, key: &str, ttl_seconds: u64) -> Result<()> {
        let mut conn = self.conn.clone();
        conn.expire(key, ttl_seconds as i64)
            .await
            .context("Failed to set key expiration")?;

        Ok(())
    }
}

/// Create a Redis connection manager
pub async fn create_redis_connection(redis_url: &str) -> Result<ConnectionManager> {
    let client = redis::Client::open(redis_url)
        .context("Failed to create Redis client")?;
    
    let conn = client
        .get_connection_manager()
        .await
        .context("Failed to get Redis connection manager")?;

    Ok(conn)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper function to create a test Redis connection
    async fn create_test_redis() -> Result<RedisCache> {
        let redis_url = std::env::var("REDIS_URL")
            .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
        
        let conn = create_redis_connection(&redis_url).await?;
        Ok(RedisCache::new(conn))
    }

    #[tokio::test]
    #[ignore] // Requires Redis to be running
    async fn test_redis_connection() {
        let result = create_test_redis().await;
        assert!(result.is_ok());
    }

    // ========================================================================
    // Generic Cache Operations Tests
    // ========================================================================

    #[tokio::test]
    #[ignore] // Requires Redis to be running
    async fn test_set_and_get() {
        let cache = create_test_redis().await.unwrap();
        
        cache.set_with_ttl("test_key", "test_value", 60).await.unwrap();
        let value = cache.get("test_key").await.unwrap();
        
        assert_eq!(value, Some("test_value".to_string()));
        
        // Cleanup
        cache.delete("test_key").await.unwrap();
    }

    #[tokio::test]
    #[ignore] // Requires Redis to be running
    async fn test_exists() {
        let cache = create_test_redis().await.unwrap();
        
        cache.set_with_ttl("test_exists", "value", 60).await.unwrap();
        
        let exists = cache.exists("test_exists").await.unwrap();
        assert!(exists);
        
        let not_exists = cache.exists("nonexistent_key").await.unwrap();
        assert!(!not_exists);
        
        // Cleanup
        cache.delete("test_exists").await.unwrap();
    }

    #[tokio::test]
    #[ignore] // Requires Redis to be running
    async fn test_delete() {
        let cache = create_test_redis().await.unwrap();
        
        cache.set_with_ttl("test_delete", "value", 60).await.unwrap();
        cache.delete("test_delete").await.unwrap();
        
        let value = cache.get("test_delete").await.unwrap();
        assert_eq!(value, None);
    }

    #[tokio::test]
    #[ignore] // Requires Redis to be running
    async fn test_ttl_expiration() {
        let cache = create_test_redis().await.unwrap();
        let key = "test_ttl_key";
        
        // Set with 2 second TTL
        cache.set_with_ttl(key, "value", 2).await.unwrap();
        
        // Should exist immediately
        let value = cache.get(key).await.unwrap();
        assert!(value.is_some());
        
        // Wait for expiration
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
        
        // Should be gone
        let value = cache.get(key).await.unwrap();
        assert!(value.is_none());
    }

    // ========================================================================
    // User Package Cache Tests
    // ========================================================================

    #[tokio::test]
    #[ignore] // Requires Redis to be running
    async fn test_user_package_cache_hit() {
        let cache = create_test_redis().await.unwrap();
        let user_id = 12345;

        let package_data = UserPackageCache {
            traffic_quota: 10737418240,
            traffic_used: 1073741824,
            expires_at: chrono::Utc::now().to_rfc3339(),
            status: "active".to_string(),
        };

        // Cache the data
        cache.cache_user_package(user_id, &package_data).await.unwrap();

        // Retrieve from cache (should hit)
        let cached = cache.get_user_package(user_id).await.unwrap();

        assert!(cached.is_some());
        let cached = cached.unwrap();
        assert_eq!(cached.traffic_quota, package_data.traffic_quota);
        assert_eq!(cached.traffic_used, package_data.traffic_used);
        assert_eq!(cached.status, package_data.status);

        // Cleanup
        cache.invalidate_user_package(user_id).await.unwrap();
    }

    #[tokio::test]
    #[ignore] // Requires Redis to be running
    async fn test_user_package_cache_miss() {
        let cache = create_test_redis().await.unwrap();
        let user_id = 99999;

        // Try to get non-existent cache entry
        let cached = cache.get_user_package(user_id).await.unwrap();
        assert!(cached.is_none());
    }

    #[tokio::test]
    #[ignore] // Requires Redis to be running
    async fn test_user_package_cache_invalidation() {
        let cache = create_test_redis().await.unwrap();
        let user_id = 54321;

        let package_data = UserPackageCache {
            traffic_quota: 5368709120,
            traffic_used: 0,
            expires_at: chrono::Utc::now().to_rfc3339(),
            status: "active".to_string(),
        };

        cache.cache_user_package(user_id, &package_data).await.unwrap();

        // Verify it's cached
        let cached = cache.get_user_package(user_id).await.unwrap();
        assert!(cached.is_some());

        // Invalidate cache
        cache.invalidate_user_package(user_id).await.unwrap();

        // Verify it's gone
        let cached = cache.get_user_package(user_id).await.unwrap();
        assert!(cached.is_none());
    }

    // ========================================================================
    // Node List Cache Tests
    // ========================================================================

    #[tokio::test]
    #[ignore] // Requires Redis to be running
    async fn test_active_nodes_cache_hit() {
        let cache = create_test_redis().await.unwrap();

        let nodes = vec![
            Node {
                id: 1,
                name: "Test Node 1".to_string(),
                host: "node1.example.com".to_string(),
                port: 443,
                protocol: "vless".to_string(),
                secret: "secret1".to_string(),
                config: serde_json::json!({"test": "config1"}),
                status: "online".to_string(),
                max_users: 1000,
                current_users: 50,
                total_upload: 0,
                total_download: 0,
                last_heartbeat: Some(chrono::Utc::now()),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            },
        ];

        // Cache the nodes
        cache.cache_active_nodes(&nodes).await.unwrap();

        // Retrieve from cache (should hit)
        let cached = cache.get_active_nodes().await.unwrap();

        assert!(cached.is_some());
        let cached = cached.unwrap();
        assert_eq!(cached.len(), 1);
        assert_eq!(cached[0].name, "Test Node 1");

        // Cleanup
        cache.invalidate_active_nodes().await.unwrap();
    }

    #[tokio::test]
    #[ignore] // Requires Redis to be running
    async fn test_active_nodes_cache_miss() {
        let cache = create_test_redis().await.unwrap();

        // Ensure cache is empty
        let _ = cache.invalidate_active_nodes().await;

        // Try to get non-existent cache entry
        let cached = cache.get_active_nodes().await.unwrap();
        assert!(cached.is_none());
    }

    #[tokio::test]
    #[ignore] // Requires Redis to be running
    async fn test_active_nodes_cache_invalidation() {
        let cache = create_test_redis().await.unwrap();

        let nodes = vec![
            Node {
                id: 3,
                name: "Test Node 3".to_string(),
                host: "node3.example.com".to_string(),
                port: 443,
                protocol: "trojan".to_string(),
                secret: "secret3".to_string(),
                config: serde_json::json!({"test": "config3"}),
                status: "online".to_string(),
                max_users: 1000,
                current_users: 100,
                total_upload: 0,
                total_download: 0,
                last_heartbeat: Some(chrono::Utc::now()),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            },
        ];

        cache.cache_active_nodes(&nodes).await.unwrap();

        // Verify it's cached
        let cached = cache.get_active_nodes().await.unwrap();
        assert!(cached.is_some());

        // Invalidate cache
        cache.invalidate_active_nodes().await.unwrap();

        // Verify it's gone
        let cached = cache.get_active_nodes().await.unwrap();
        assert!(cached.is_none());
    }

    // ========================================================================
    // Subscription Config Cache Tests
    // ========================================================================

    #[tokio::test]
    #[ignore] // Requires Redis to be running
    async fn test_subscription_config_cache() {
        let cache = create_test_redis().await.unwrap();
        let token = "test-subscription-token-123";
        let config = "proxies:\n  - name: Test Node\n    type: vless";

        // Cache the config
        cache.cache_subscription_config(token, config).await.unwrap();

        // Retrieve from cache
        let cached = cache.get_subscription_config(token).await.unwrap();

        assert!(cached.is_some());
        assert_eq!(cached.unwrap(), config);

        // Cleanup
        cache.invalidate_subscription_config(token).await.unwrap();
    }

    // ========================================================================
    // Cache Fallback Tests
    // ========================================================================

    #[tokio::test]
    #[ignore] // Requires Redis to be running
    async fn test_cache_miss_returns_none() {
        let cache = create_test_redis().await.unwrap();
        
        // Test user package cache miss
        let user_pkg = cache.get_user_package(88888).await.unwrap();
        assert!(user_pkg.is_none());
        
        // Test nodes cache miss
        let _ = cache.invalidate_active_nodes().await;
        let nodes = cache.get_active_nodes().await.unwrap();
        assert!(nodes.is_none());
        
        // Test subscription cache miss
        let sub = cache.get_subscription_config("nonexistent").await.unwrap();
        assert!(sub.is_none());
    }

    #[tokio::test]
    #[ignore] // Requires Redis to be running
    async fn test_cache_operations_are_idempotent() {
        let cache = create_test_redis().await.unwrap();
        
        // Multiple invalidations should not fail
        cache.invalidate_user_package(12345).await.unwrap();
        cache.invalidate_user_package(12345).await.unwrap();
        
        // Multiple deletes should not fail
        cache.delete("nonexistent_key").await.unwrap();
        cache.delete("nonexistent_key").await.unwrap();
    }
}
