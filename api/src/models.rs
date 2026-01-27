use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// User model representing a platform user
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: i64,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub coin_balance: i64,
    pub traffic_quota: i64,
    pub traffic_used: i64,
    pub referral_code: Option<String>,
    pub referred_by: Option<i64>,
    pub status: String,
    pub is_admin: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Package model representing a traffic package
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Package {
    pub id: i64,
    pub name: String,
    pub traffic_amount: i64,
    pub price: i64,
    pub duration_days: i32,
    pub description: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Order model representing a purchase order
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Order {
    pub id: i64,
    pub order_no: String,
    pub user_id: i64,
    pub package_id: i64,
    pub amount: i64,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// UserPackage model representing a user's purchased package
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserPackage {
    pub id: i64,
    pub user_id: i64,
    pub package_id: i64,
    pub order_id: i64,
    pub traffic_quota: i64,
    pub traffic_used: i64,
    pub expires_at: DateTime<Utc>,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

/// Node model representing a VPN server node
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Node {
    pub id: i64,
    pub name: String,
    pub host: String,
    pub port: i32,
    pub protocol: String,
    #[serde(skip_serializing)]
    pub secret: String,
    pub config: serde_json::Value,
    pub status: String,
    pub max_users: i32,
    pub current_users: i32,
    pub total_upload: i64,
    pub total_download: i64,
    pub last_heartbeat: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// TrafficLog model representing traffic usage records
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TrafficLog {
    pub id: i64,
    pub user_id: i64,
    pub node_id: i64,
    pub upload: i64,
    pub download: i64,
    pub recorded_at: DateTime<Utc>,
}

/// Subscription model representing a user's subscription link
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Subscription {
    pub id: i64,
    pub user_id: i64,
    pub token: String,
    pub created_at: DateTime<Utc>,
    pub last_accessed: Option<DateTime<Utc>>,
}

/// CoinTransaction model representing coin balance changes
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CoinTransaction {
    pub id: i64,
    pub user_id: i64,
    pub amount: i64,
    #[serde(rename = "type")]
    #[sqlx(rename = "type")]
    pub transaction_type: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// AdminLog model representing admin operations
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AdminLog {
    pub id: i64,
    pub admin_id: i64,
    pub action: String,
    pub target_type: Option<String>,
    pub target_id: Option<i64>,
    pub details: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

// DTO (Data Transfer Object) models for API requests/responses

/// Request body for user registration
#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    pub referral_code: Option<String>,
}

/// Request body for user login
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

/// Response body for authentication (login/register)
#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: UserResponse,
}

/// User response (without sensitive data)
#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: i64,
    pub email: String,
    pub coin_balance: i64,
    pub traffic_quota: i64,
    pub traffic_used: i64,
    pub referral_code: Option<String>,
    pub status: String,
    pub is_admin: bool,
    pub created_at: DateTime<Utc>,
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            email: user.email,
            coin_balance: user.coin_balance,
            traffic_quota: user.traffic_quota,
            traffic_used: user.traffic_used,
            referral_code: user.referral_code,
            status: user.status,
            is_admin: user.is_admin,
            created_at: user.created_at,
        }
    }
}

/// Request body for package purchase
#[derive(Debug, Deserialize)]
pub struct PurchasePackageRequest {
    pub package_id: i64,
}

/// Request body for creating a node
#[derive(Debug, Deserialize)]
pub struct CreateNodeRequest {
    pub name: String,
    pub host: String,
    pub port: i32,
    pub protocol: String,
    pub config: serde_json::Value,
}

/// Request body for updating a node
#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateNodeRequest {
    pub name: Option<String>,
    pub host: Option<String>,
    pub port: Option<i32>,
    pub protocol: Option<String>,
    pub config: Option<serde_json::Value>,
    pub status: Option<String>,
}

/// Request body for node heartbeat
#[derive(Debug, Deserialize)]
pub struct HeartbeatRequest {
    pub node_id: i64,
    pub secret: String,
    pub status: String,
    pub cpu_usage: Option<f64>,
    pub memory_usage: Option<f64>,
    pub active_connections: Option<i32>,
}

/// Request body for traffic reporting
#[derive(Debug, Deserialize)]
pub struct TrafficReportRequest {
    pub node_id: i64,
    pub user_id: i64,
    pub upload: i64,
    pub download: i64,
}

/// Response for statistics overview
#[derive(Debug, Serialize)]
pub struct StatsOverview {
    pub total_users: i64,
    pub active_users: i64,
    pub total_traffic: i64,
    pub total_revenue: i64,
    pub online_nodes: i64,
}

// ============================================================================
// Clash Configuration Models
// ============================================================================

/// ClashProxy model representing a Clash proxy configuration
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ClashProxy {
    pub id: i64,
    pub name: String,
    #[serde(rename = "type")]
    #[sqlx(rename = "type")]
    pub proxy_type: String,
    pub server: String,
    pub port: i32,
    pub config: serde_json::Value,
    pub is_active: bool,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// ClashProxyGroup model representing a Clash proxy group
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ClashProxyGroup {
    pub id: i64,
    pub name: String,
    #[serde(rename = "type")]
    #[sqlx(rename = "type")]
    pub group_type: String,
    pub proxies: Vec<String>,
    pub url: Option<String>,
    pub interval: Option<i32>,
    pub tolerance: Option<i32>,
    pub is_active: bool,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// ClashRule model representing a Clash routing rule
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ClashRule {
    pub id: i64,
    pub rule_type: String,
    pub rule_value: Option<String>,
    pub proxy_group: String,
    pub no_resolve: bool,
    pub is_active: bool,
    pub sort_order: i32,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Request body for creating/updating a Clash proxy
#[derive(Debug, Deserialize)]
pub struct ClashProxyRequest {
    pub name: String,
    #[serde(rename = "type")]
    pub proxy_type: String,
    pub server: String,
    pub port: i32,
    pub config: serde_json::Value,
    pub is_active: Option<bool>,
    pub sort_order: Option<i32>,
}

/// Request body for creating/updating a Clash proxy group
#[derive(Debug, Deserialize)]
pub struct ClashProxyGroupRequest {
    pub name: String,
    #[serde(rename = "type")]
    pub group_type: String,
    pub proxies: Vec<String>,
    pub url: Option<String>,
    pub interval: Option<i32>,
    pub tolerance: Option<i32>,
    pub is_active: Option<bool>,
    pub sort_order: Option<i32>,
}

/// Request body for creating/updating a Clash rule
#[derive(Debug, Deserialize)]
pub struct ClashRuleRequest {
    pub rule_type: String,
    pub rule_value: Option<String>,
    pub proxy_group: String,
    pub no_resolve: Option<bool>,
    pub is_active: Option<bool>,
    pub sort_order: Option<i32>,
    pub description: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn test_user_response_from_user() {
        let user = User {
            id: 1,
            email: "test@example.com".to_string(),
            password_hash: "hashed_password".to_string(),
            coin_balance: 1000,
            traffic_quota: 10737418240,
            traffic_used: 1073741824,
            referral_code: Some("ABC123".to_string()),
            referred_by: None,
            status: "active".to_string(),
            is_admin: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let response: UserResponse = user.clone().into();
        
        assert_eq!(response.id, user.id);
        assert_eq!(response.email, user.email);
        assert_eq!(response.coin_balance, user.coin_balance);
        assert_eq!(response.traffic_quota, user.traffic_quota);
        assert_eq!(response.traffic_used, user.traffic_used);
        assert_eq!(response.referral_code, user.referral_code);
        assert_eq!(response.status, user.status);
        assert_eq!(response.is_admin, user.is_admin);
    }

    #[test]
    fn test_user_serialization_skips_password() {
        let user = User {
            id: 1,
            email: "test@example.com".to_string(),
            password_hash: "hashed_password".to_string(),
            coin_balance: 1000,
            traffic_quota: 10737418240,
            traffic_used: 1073741824,
            referral_code: Some("ABC123".to_string()),
            referred_by: None,
            status: "active".to_string(),
            is_admin: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let json = serde_json::to_string(&user).unwrap();
        assert!(!json.contains("password_hash"));
        assert!(!json.contains("hashed_password"));
    }

    #[test]
    fn test_node_serialization_skips_secret() {
        let node = Node {
            id: 1,
            name: "Test Node".to_string(),
            host: "example.com".to_string(),
            port: 443,
            protocol: "vless".to_string(),
            secret: "secret_key".to_string(),
            config: serde_json::json!({"key": "value"}),
            status: "online".to_string(),
            max_users: 1000,
            current_users: 50,
            total_upload: 1073741824,
            total_download: 2147483648,
            last_heartbeat: Some(Utc::now()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let json = serde_json::to_string(&node).unwrap();
        assert!(!json.contains("secret"));
        assert!(!json.contains("secret_key"));
    }

    // Property-based tests

    // Feature: vpn-subscription-platform, Property 10: 节点配置往返一致性
    // **Validates: Requirements 6.1**
    // For any node configuration, saving to database and reading back should yield the same configuration content (JSON serialization round-trip)
    proptest! {
        #[test]
        fn test_node_config_roundtrip_consistency(
            protocol in prop::sample::select(vec!["shadowsocks", "vmess", "trojan", "hysteria2", "vless"]),
            port in 1..=65535i32,
            max_users in 1..=10000i32,
        ) {
            // Create a node configuration with various protocol-specific settings
            let config = match protocol.as_ref() {
                "shadowsocks" => serde_json::json!({
                    "method": "aes-256-gcm",
                    "password": "test_password"
                }),
                "vmess" => serde_json::json!({
                    "alter_id": 0,
                    "security": "auto"
                }),
                "trojan" => serde_json::json!({
                    "password": "test_password"
                }),
                "hysteria2" => serde_json::json!({
                    "password": "test_password",
                    "obfs": "salamander"
                }),
                "vless" => serde_json::json!({
                    "flow": "xtls-rprx-vision",
                    "encryption": "none",
                    "reality": {
                        "dest": "www.microsoft.com:443",
                        "serverNames": ["www.microsoft.com"],
                        "publicKey": "test_public_key",
                        "privateKey": "test_private_key",
                        "shortIds": [""]
                    }
                }),
                _ => serde_json::json!({}),
            };

            let node = Node {
                id: 1,
                name: format!("Test {} Node", protocol),
                host: "example.com".to_string(),
                port,
                protocol: protocol.to_string(),
                secret: "test_secret".to_string(),
                config: config.clone(),
                status: "online".to_string(),
                max_users,
                current_users: 0,
                total_upload: 0,
                total_download: 0,
                last_heartbeat: Some(Utc::now()),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            };

            // Serialize to JSON (simulating database storage)
            let serialized = serde_json::to_string(&node.config).unwrap();
            
            // Deserialize back (simulating database retrieval)
            let deserialized: serde_json::Value = serde_json::from_str(&serialized).unwrap();
            
            // Verify round-trip consistency
            prop_assert_eq!(&node.config, &deserialized);
            
            // Verify protocol-specific fields are preserved
            match protocol.as_ref() {
                "shadowsocks" => {
                    prop_assert_eq!(&deserialized["method"], "aes-256-gcm");
                    prop_assert_eq!(&deserialized["password"], "test_password");
                },
                "vmess" => {
                    prop_assert_eq!(&deserialized["alter_id"], &serde_json::json!(0));
                    prop_assert_eq!(&deserialized["security"], "auto");
                },
                "trojan" => {
                    prop_assert_eq!(&deserialized["password"], "test_password");
                },
                "hysteria2" => {
                    prop_assert_eq!(&deserialized["password"], "test_password");
                    prop_assert_eq!(&deserialized["obfs"], "salamander");
                },
                "vless" => {
                    prop_assert_eq!(&deserialized["flow"], "xtls-rprx-vision");
                    prop_assert_eq!(&deserialized["encryption"], "none");
                    prop_assert!(deserialized["reality"].is_object());
                    prop_assert_eq!(&deserialized["reality"]["dest"], "www.microsoft.com:443");
                },
                _ => {},
            }
        }
    }

    // Additional property test: Node configuration with nested JSON structures
    proptest! {
        #[test]
        fn test_complex_node_config_roundtrip(
            num_server_names in 1..=5usize,
            num_short_ids in 0..=3usize,
        ) {
            // Create a complex VLESS-Reality configuration with arrays
            let server_names: Vec<String> = (0..num_server_names)
                .map(|i| format!("server{}.example.com", i))
                .collect();
            
            let short_ids: Vec<String> = (0..num_short_ids)
                .map(|i| format!("id{}", i))
                .collect();

            let config = serde_json::json!({
                "flow": "xtls-rprx-vision",
                "encryption": "none",
                "reality": {
                    "dest": "www.example.com:443",
                    "serverNames": server_names,
                    "publicKey": "test_public_key_12345",
                    "privateKey": "test_private_key_67890",
                    "shortIds": short_ids,
                    "xver": 0,
                    "show": false
                }
            });

            // Serialize and deserialize
            let serialized = serde_json::to_string(&config).unwrap();
            let deserialized: serde_json::Value = serde_json::from_str(&serialized).unwrap();
            
            // Verify round-trip consistency
            prop_assert_eq!(&config, &deserialized);
            
            // Verify array lengths are preserved
            prop_assert_eq!(
                deserialized["reality"]["serverNames"].as_array().unwrap().len(),
                num_server_names
            );
            prop_assert_eq!(
                deserialized["reality"]["shortIds"].as_array().unwrap().len(),
                num_short_ids
            );
        }
    }

    // Property test: Verify all node fields survive serialization
    proptest! {
        #[test]
        fn test_node_full_roundtrip(
            id in 1..=1000000i64,
            port in 1..=65535i32,
            max_users in 1..=10000i32,
            current_users in 0..=10000i32,
            total_upload in 0..=1000000000000i64,
            total_download in 0..=1000000000000i64,
        ) {
            let config = serde_json::json!({
                "test_field": "test_value",
                "nested": {
                    "field": 123
                }
            });

            let node = Node {
                id,
                name: "Test Node".to_string(),
                host: "example.com".to_string(),
                port,
                protocol: "vless".to_string(),
                secret: "secret".to_string(),
                config: config.clone(),
                status: "online".to_string(),
                max_users,
                current_users,
                total_upload,
                total_download,
                last_heartbeat: Some(Utc::now()),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            };

            // Serialize the entire node
            let serialized = serde_json::to_string(&node).unwrap();
            let deserialized: serde_json::Value = serde_json::from_str(&serialized).unwrap();
            
            // Verify key fields are preserved (excluding secret which is skipped)
            prop_assert_eq!(&deserialized["id"], &serde_json::json!(id));
            prop_assert_eq!(&deserialized["port"], &serde_json::json!(port));
            prop_assert_eq!(&deserialized["max_users"], &serde_json::json!(max_users));
            prop_assert_eq!(&deserialized["current_users"], &serde_json::json!(current_users));
            prop_assert_eq!(&deserialized["total_upload"], &serde_json::json!(total_upload));
            prop_assert_eq!(&deserialized["total_download"], &serde_json::json!(total_download));
            
            // Verify config is preserved
            prop_assert_eq!(&deserialized["config"], &config);
            
            // Verify secret is NOT in serialized output
            prop_assert!(deserialized.get("secret").is_none());
        }
    }
}
