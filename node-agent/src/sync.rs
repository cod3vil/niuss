use anyhow::{Context, Result};
use futures_util::stream::StreamExt;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

use crate::config::Config;

/// Node configuration received from API service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    pub protocol: String,
    pub port: u16,
    pub users: Vec<UserConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reality_config: Option<RealityConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shadowsocks_config: Option<ShadowsocksConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vmess_config: Option<VMessConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trojan_config: Option<TrojanConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hysteria2_config: Option<Hysteria2Config>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserConfig {
    pub id: String,
    pub email: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flow: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealityConfig {
    pub show: bool,
    pub dest: String,
    pub xver: u8,
    pub server_names: Vec<String>,
    pub private_key: String,
    pub short_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShadowsocksConfig {
    pub method: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VMessConfig {
    pub alter_id: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrojanConfig {
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hysteria2Config {
    pub password: String,
    pub obfs: Option<String>,
}

/// Configuration synchronization manager
pub struct ConfigSync {
    config: Arc<Config>,
    http_client: reqwest::Client,
    redis_url: Option<String>,
    current_config: Arc<RwLock<Option<NodeConfig>>>,
}

impl ConfigSync {
    pub fn new(config: Arc<Config>, redis_url: Option<String>) -> Self {
        Self {
            config,
            http_client: reqwest::Client::new(),
            redis_url,
            current_config: Arc::new(RwLock::new(None)),
        }
    }

    /// Register with API service and fetch initial configuration
    pub async fn register_and_fetch_config(&self) -> Result<NodeConfig> {
        info!("Registering with API service and fetching initial configuration");

        let url = format!(
            "{}/api/node/config?node_id={}&secret={}",
            self.config.api_url, self.config.node_id, self.config.node_secret
        );

        let response = self
            .http_client
            .get(&url)
            .send()
            .await
            .context("Failed to send config request to API")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!(
                "Failed to fetch config from API: status={}, body={}",
                status,
                body
            );
        }

        let node_config: NodeConfig = response
            .json()
            .await
            .context("Failed to parse config response")?;

        info!(
            "Successfully fetched initial configuration: protocol={}, port={}",
            node_config.protocol, node_config.port
        );

        // Store the configuration
        let mut current = self.current_config.write().await;
        *current = Some(node_config.clone());

        Ok(node_config)
    }

    /// Subscribe to configuration updates via Redis Pub/Sub
    pub async fn subscribe_to_updates(&self) -> Result<()> {
        let redis_url = match &self.redis_url {
            Some(url) => url.clone(),
            None => {
                warn!("Redis URL not available, skipping config update subscription");
                return Ok(());
            }
        };

        let channel = format!("node:config:update:{}", self.config.node_id);
        info!("Subscribing to config updates on channel: {}", channel);

        // Create a new Redis client for pub/sub
        let client = redis::Client::open(redis_url.as_str())
            .context("Failed to create Redis client for pub/sub")?;
        
        let conn = client
            .get_async_connection()
            .await
            .context("Failed to get Redis connection")?;
        
        let mut pubsub = conn.into_pubsub();
        
        pubsub
            .subscribe(&channel)
            .await
            .context("Failed to subscribe to config update channel")?;

        let config_sync = self.clone_for_updates();

        tokio::spawn(async move {
            let mut stream = pubsub.on_message();
            loop {
                match stream.next().await {
                    Some(msg) => {
                        let payload: String = match msg.get_payload() {
                            Ok(p) => p,
                            Err(e) => {
                                error!("Failed to get message payload: {}", e);
                                continue;
                            }
                        };

                        info!("Received config update notification: {}", payload);

                        // Fetch the new configuration
                        match config_sync.register_and_fetch_config().await {
                            Ok(new_config) => {
                                info!("Successfully fetched updated configuration");
                                // Apply the new configuration to Xray-core
                                if let Err(e) = config_sync.apply_config(&new_config).await {
                                    error!("Failed to apply new configuration: {}", e);
                                }
                            }
                            Err(e) => {
                                error!("Failed to fetch updated configuration: {}", e);
                            }
                        }
                    }
                    None => {
                        warn!("Config update subscription ended, reconnecting...");
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    /// Apply configuration to Xray-core
    pub async fn apply_config(&self, config: &NodeConfig) -> Result<()> {
        info!("Applying configuration to Xray-core");

        // Generate Xray-core configuration JSON
        let xray_config = self.generate_xray_config(config)?;

        // Write configuration to file
        let config_path = "/etc/xray/config.json";
        tokio::fs::write(config_path, xray_config)
            .await
            .context("Failed to write Xray config file")?;

        info!("Xray configuration written to {}", config_path);

        // Reload Xray-core (send SIGHUP or restart service)
        // This is a placeholder - actual implementation would use systemctl or similar
        info!("Reloading Xray-core service");

        Ok(())
    }

    /// Generate Xray-core configuration JSON from NodeConfig
    fn generate_xray_config(&self, config: &NodeConfig) -> Result<String> {
        let mut xray_config = serde_json::json!({
            "log": {
                "loglevel": "warning"
            },
            "api": {
                "tag": "api",
                "services": ["StatsService"]
            },
            "stats": {},
            "policy": {
                "levels": {
                    "0": {
                        "statsUserUplink": true,
                        "statsUserDownlink": true
                    }
                },
                "system": {
                    "statsInboundUplink": true,
                    "statsInboundDownlink": true
                }
            },
            "inbounds": [],
            "outbounds": [
                {
                    "protocol": "freedom",
                    "tag": "direct"
                }
            ],
            "routing": {
                "rules": [
                    {
                        "type": "field",
                        "inboundTag": ["api"],
                        "outboundTag": "api"
                    }
                ]
            }
        });

        // Add API inbound
        xray_config["inbounds"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({
                "tag": "api",
                "listen": "127.0.0.1",
                "port": self.config.xray_api_port,
                "protocol": "dokodemo-door",
                "settings": {
                    "address": "127.0.0.1"
                }
            }));

        // Add main inbound based on protocol
        let main_inbound = match config.protocol.as_str() {
            "vless" => self.generate_vless_inbound(config)?,
            "vmess" => self.generate_vmess_inbound(config)?,
            "trojan" => self.generate_trojan_inbound(config)?,
            "shadowsocks" => self.generate_shadowsocks_inbound(config)?,
            "hysteria2" => self.generate_hysteria2_inbound(config)?,
            _ => anyhow::bail!("Unsupported protocol: {}", config.protocol),
        };

        xray_config["inbounds"]
            .as_array_mut()
            .unwrap()
            .push(main_inbound);

        serde_json::to_string_pretty(&xray_config).context("Failed to serialize Xray config")
    }

    fn generate_vless_inbound(&self, config: &NodeConfig) -> Result<serde_json::Value> {
        let clients: Vec<serde_json::Value> = config
            .users
            .iter()
            .map(|user| {
                serde_json::json!({
                    "id": user.id,
                    "email": user.email,
                    "flow": user.flow.as_ref().unwrap_or(&"xtls-rprx-vision".to_string())
                })
            })
            .collect();

        let mut inbound = serde_json::json!({
            "port": config.port,
            "protocol": "vless",
            "settings": {
                "clients": clients,
                "decryption": "none"
            },
            "streamSettings": {
                "network": "tcp",
                "security": "reality"
            }
        });

        // Add Reality configuration if present
        if let Some(reality) = &config.reality_config {
            inbound["streamSettings"]["realitySettings"] = serde_json::json!({
                "show": reality.show,
                "dest": reality.dest,
                "xver": reality.xver,
                "serverNames": reality.server_names,
                "privateKey": reality.private_key,
                "shortIds": reality.short_ids
            });
        }

        Ok(inbound)
    }

    fn generate_vmess_inbound(&self, config: &NodeConfig) -> Result<serde_json::Value> {
        let clients: Vec<serde_json::Value> = config
            .users
            .iter()
            .map(|user| {
                serde_json::json!({
                    "id": user.id,
                    "email": user.email,
                    "alterId": config.vmess_config.as_ref().map(|c| c.alter_id).unwrap_or(0)
                })
            })
            .collect();

        Ok(serde_json::json!({
            "port": config.port,
            "protocol": "vmess",
            "settings": {
                "clients": clients
            },
            "streamSettings": {
                "network": "tcp"
            }
        }))
    }

    fn generate_trojan_inbound(&self, config: &NodeConfig) -> Result<serde_json::Value> {
        let clients: Vec<serde_json::Value> = config
            .users
            .iter()
            .map(|user| {
                serde_json::json!({
                    "password": config.trojan_config.as_ref()
                        .map(|c| c.password.clone())
                        .unwrap_or_else(|| user.id.clone()),
                    "email": user.email
                })
            })
            .collect();

        Ok(serde_json::json!({
            "port": config.port,
            "protocol": "trojan",
            "settings": {
                "clients": clients
            },
            "streamSettings": {
                "network": "tcp",
                "security": "tls"
            }
        }))
    }

    fn generate_shadowsocks_inbound(&self, config: &NodeConfig) -> Result<serde_json::Value> {
        let ss_config = config
            .shadowsocks_config
            .as_ref()
            .context("Shadowsocks config is required")?;

        Ok(serde_json::json!({
            "port": config.port,
            "protocol": "shadowsocks",
            "settings": {
                "method": ss_config.method,
                "password": ss_config.password,
                "network": "tcp,udp"
            }
        }))
    }

    fn generate_hysteria2_inbound(&self, config: &NodeConfig) -> Result<serde_json::Value> {
        let h2_config = config
            .hysteria2_config
            .as_ref()
            .context("Hysteria2 config is required")?;

        Ok(serde_json::json!({
            "port": config.port,
            "protocol": "hysteria2",
            "settings": {
                "password": h2_config.password,
                "obfs": h2_config.obfs
            }
        }))
    }

    /// Clone for use in async tasks
    fn clone_for_updates(&self) -> Self {
        Self {
            config: Arc::clone(&self.config),
            http_client: self.http_client.clone(),
            redis_url: self.redis_url.clone(),
            current_config: Arc::clone(&self.current_config),
        }
    }

    /// Get current configuration
    pub async fn get_current_config(&self) -> Option<NodeConfig> {
        self.current_config.read().await.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_config_serialization() {
        let config = NodeConfig {
            protocol: "vless".to_string(),
            port: 443,
            users: vec![UserConfig {
                id: "uuid-123".to_string(),
                email: "user@example.com".to_string(),
                flow: Some("xtls-rprx-vision".to_string()),
            }],
            reality_config: Some(RealityConfig {
                show: false,
                dest: "www.microsoft.com:443".to_string(),
                xver: 0,
                server_names: vec!["www.microsoft.com".to_string()],
                private_key: "private-key".to_string(),
                short_ids: vec!["".to_string()],
            }),
            shadowsocks_config: None,
            vmess_config: None,
            trojan_config: None,
            hysteria2_config: None,
        };

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: NodeConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.protocol, "vless");
        assert_eq!(deserialized.port, 443);
        assert_eq!(deserialized.users.len(), 1);
        assert!(deserialized.reality_config.is_some());
    }

    #[tokio::test]
    async fn test_generate_vless_config() {
        let config = Arc::new(Config {
            api_url: "http://localhost:8080".to_string(),
            node_id: "test-node".to_string(),
            node_secret: "secret".to_string(),
            xray_api_port: 10085,
            traffic_report_interval: 30,
            heartbeat_interval: 60,
        });

        let sync = ConfigSync::new(config, None);

        let node_config = NodeConfig {
            protocol: "vless".to_string(),
            port: 443,
            users: vec![UserConfig {
                id: "uuid-123".to_string(),
                email: "user@example.com".to_string(),
                flow: Some("xtls-rprx-vision".to_string()),
            }],
            reality_config: Some(RealityConfig {
                show: false,
                dest: "www.microsoft.com:443".to_string(),
                xver: 0,
                server_names: vec!["www.microsoft.com".to_string()],
                private_key: "private-key".to_string(),
                short_ids: vec!["".to_string()],
            }),
            shadowsocks_config: None,
            vmess_config: None,
            trojan_config: None,
            hysteria2_config: None,
        };

        let xray_config = sync.generate_xray_config(&node_config).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&xray_config).unwrap();

        assert!(parsed["inbounds"].is_array());
        assert!(parsed["outbounds"].is_array());
        assert!(parsed["api"].is_object());
    }
}
