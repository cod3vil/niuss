use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use tracing::{error, info, warn};

use crate::config::Config;
use crate::sync::UserConfig;

/// Active user information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveUser {
    pub user_id: String,
    pub email: String,
    pub traffic_quota: u64,
    pub traffic_used: u64,
}

/// Response from API service for active users
#[derive(Debug, Deserialize)]
pub struct ActiveUsersResponse {
    pub users: Vec<ActiveUser>,
}

/// User manager that syncs active users and manages Xray-core user configuration
pub struct UserManager {
    config: Arc<Config>,
    http_client: reqwest::Client,
    active_users: Arc<RwLock<HashSet<String>>>,
}

impl UserManager {
    pub fn new(config: Arc<Config>) -> Self {
        Self {
            config,
            http_client: reqwest::Client::new(),
            active_users: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    /// Start the user management loop
    pub async fn start(&self) -> Result<()> {
        let config = Arc::clone(&self.config);
        let http_client = self.http_client.clone();
        let active_users = Arc::clone(&self.active_users);

        tokio::spawn(async move {
            // Sync every 60 seconds
            let mut ticker = interval(Duration::from_secs(60));

            loop {
                ticker.tick().await;

                match Self::sync_active_users(&config, &http_client, &active_users).await {
                    Ok(count) => {
                        info!("Successfully synced {} active users", count);
                    }
                    Err(e) => {
                        error!("Failed to sync active users: {}", e);
                    }
                }
            }
        });

        info!("User manager started");

        Ok(())
    }

    /// Sync active users from API service
    async fn sync_active_users(
        config: &Config,
        http_client: &reqwest::Client,
        active_users: &Arc<RwLock<HashSet<String>>>,
    ) -> Result<usize> {
        // Fetch active users from API
        let url = format!(
            "{}/api/node/users?node_id={}&secret={}",
            config.api_url, config.node_id, config.node_secret
        );

        let response = http_client
            .get(&url)
            .send()
            .await
            .context("Failed to fetch active users from API")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!(
                "Failed to fetch active users: status={}, body={}",
                status,
                body
            );
        }

        let users_response: ActiveUsersResponse = response
            .json()
            .await
            .context("Failed to parse active users response")?;

        // Update active users set
        let mut users_set = active_users.write().await;
        users_set.clear();

        for user in &users_response.users {
            // Check if user has exceeded traffic quota
            if user.traffic_used >= user.traffic_quota {
                warn!(
                    "User {} has exceeded traffic quota, skipping",
                    user.email
                );
                continue;
            }

            users_set.insert(user.email.clone());
        }

        let count = users_set.len();
        drop(users_set);

        info!("Active users updated: {} users", count);

        Ok(count)
    }

    /// Get list of active user emails
    pub async fn get_active_users(&self) -> Vec<String> {
        let users = self.active_users.read().await;
        users.iter().cloned().collect()
    }

    /// Check if a user is active
    pub async fn is_user_active(&self, email: &str) -> bool {
        let users = self.active_users.read().await;
        users.contains(email)
    }

    /// Convert active users to Xray UserConfig format
    pub async fn get_xray_user_configs(&self) -> Vec<UserConfig> {
        let users = self.active_users.read().await;
        users
            .iter()
            .map(|email| {
                // Generate UUID from email (simplified - in production use proper UUID)
                let uuid = Self::email_to_uuid(email);
                UserConfig {
                    id: uuid,
                    email: email.clone(),
                    flow: Some("xtls-rprx-vision".to_string()),
                }
            })
            .collect()
    }

    /// Convert email to UUID (simplified implementation)
    fn email_to_uuid(email: &str) -> String {
        // In production, this should use a proper UUID generation or lookup
        // For now, we'll use a simple hash-based approach
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        email.hash(&mut hasher);
        let hash = hasher.finish();

        // Format as UUID-like string
        format!(
            "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
            (hash >> 32) as u32,
            ((hash >> 16) & 0xFFFF) as u16,
            (hash & 0xFFFF) as u16,
            ((hash >> 48) & 0xFFFF) as u16,
            hash & 0xFFFFFFFFFFFF
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_active_user_serialization() {
        let user = ActiveUser {
            user_id: "123".to_string(),
            email: "user@example.com".to_string(),
            traffic_quota: 10737418240, // 10GB
            traffic_used: 5368709120,   // 5GB
        };

        let json = serde_json::to_string(&user).unwrap();
        let deserialized: ActiveUser = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.user_id, "123");
        assert_eq!(deserialized.email, "user@example.com");
        assert_eq!(deserialized.traffic_quota, 10737418240);
        assert_eq!(deserialized.traffic_used, 5368709120);
    }

    #[test]
    fn test_email_to_uuid() {
        let email1 = "user1@example.com";
        let email2 = "user2@example.com";

        let uuid1 = UserManager::email_to_uuid(email1);
        let uuid2 = UserManager::email_to_uuid(email2);

        // UUIDs should be different for different emails
        assert_ne!(uuid1, uuid2);

        // UUID should be consistent for the same email
        let uuid1_again = UserManager::email_to_uuid(email1);
        assert_eq!(uuid1, uuid1_again);

        // UUID should be in the correct format
        assert!(uuid1.contains('-'));
        assert_eq!(uuid1.len(), 36);
    }

    #[tokio::test]
    async fn test_user_manager_active_users() {
        let config = Arc::new(Config {
            api_url: "http://localhost:8080".to_string(),
            node_id: "test-node".to_string(),
            node_secret: "secret".to_string(),
            xray_api_port: 10085,
            traffic_report_interval: 30,
            heartbeat_interval: 60,
        });

        let manager = UserManager::new(config);

        // Initially no active users
        let users = manager.get_active_users().await;
        assert_eq!(users.len(), 0);

        // Add some users manually for testing
        {
            let mut active_users = manager.active_users.write().await;
            active_users.insert("user1@example.com".to_string());
            active_users.insert("user2@example.com".to_string());
        }

        // Check active users
        let users = manager.get_active_users().await;
        assert_eq!(users.len(), 2);

        assert!(manager.is_user_active("user1@example.com").await);
        assert!(manager.is_user_active("user2@example.com").await);
        assert!(!manager.is_user_active("user3@example.com").await);

        // Get Xray user configs
        let configs = manager.get_xray_user_configs().await;
        assert_eq!(configs.len(), 2);
    }
}
