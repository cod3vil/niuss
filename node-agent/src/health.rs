use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::process::Command;
use std::sync::Arc;
use tokio::time::{interval, Duration};
use tracing::{error, info, warn};

use crate::config::Config;

/// Health status of the node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeHealth {
    pub node_id: String,
    pub status: String,
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub active_connections: u32,
    pub xray_status: String,
}

/// Heartbeat data sent to API service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatData {
    pub node_id: String,
    pub secret: String,
    pub status: String,
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub active_connections: u32,
}

/// Health checker that monitors Xray-core and sends heartbeats
pub struct HealthChecker {
    config: Arc<Config>,
    http_client: reqwest::Client,
}

impl HealthChecker {
    pub fn new(config: Arc<Config>) -> Self {
        Self {
            config,
            http_client: reqwest::Client::new(),
        }
    }

    /// Start the health check and heartbeat loop
    pub async fn start(&self) -> Result<()> {
        let config = Arc::clone(&self.config);
        let http_client = self.http_client.clone();

        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(config.heartbeat_interval));

            loop {
                ticker.tick().await;

                match Self::check_and_send_heartbeat(&config, &http_client).await {
                    Ok(_) => {
                        info!("Heartbeat sent successfully");
                    }
                    Err(e) => {
                        error!("Failed to send heartbeat: {}", e);
                    }
                }
            }
        });

        info!(
            "Health checker started with interval: {} seconds",
            self.config.heartbeat_interval
        );

        Ok(())
    }

    /// Check node health and send heartbeat to API service
    async fn check_and_send_heartbeat(
        config: &Config,
        http_client: &reqwest::Client,
    ) -> Result<()> {
        // Check Xray-core status
        let xray_status = Self::check_xray_status(config).await;

        // If Xray is not running, try to restart it
        if xray_status != "running" {
            warn!("Xray-core is not running, attempting to restart");
            if let Err(e) = Self::restart_xray().await {
                error!("Failed to restart Xray-core: {}", e);
            }
        }

        // Collect system metrics
        let cpu_usage = Self::get_cpu_usage().await.unwrap_or(0.0);
        let memory_usage = Self::get_memory_usage().await.unwrap_or(0.0);
        let active_connections = Self::get_active_connections(config).await.unwrap_or(0);

        // Prepare heartbeat data
        let heartbeat = HeartbeatData {
            node_id: config.node_id.clone(),
            secret: config.node_secret.clone(),
            status: if xray_status == "running" {
                "online".to_string()
            } else {
                "offline".to_string()
            },
            cpu_usage,
            memory_usage,
            active_connections,
        };

        // Send heartbeat to API service
        let url = format!("{}/api/node/heartbeat", config.api_url);

        let response = http_client
            .post(&url)
            .json(&heartbeat)
            .send()
            .await
            .context("Failed to send heartbeat request")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!(
                "Heartbeat request failed: status={}, body={}",
                status,
                body
            );
        }

        Ok(())
    }

    /// Check if Xray-core is running
    async fn check_xray_status(config: &Config) -> String {
        // Try to connect to Xray API
        let api_url = format!("http://127.0.0.1:{}/stats/query", config.xray_api_port);

        match reqwest::Client::new()
            .post(&api_url)
            .json(&serde_json::json!({
                "pattern": "",
                "reset": false
            }))
            .timeout(Duration::from_secs(5))
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => "running".to_string(),
            _ => "stopped".to_string(),
        }
    }

    /// Restart Xray-core service
    async fn restart_xray() -> Result<()> {
        info!("Restarting Xray-core service");

        // Try systemctl restart
        let output = Command::new("systemctl")
            .args(&["restart", "xray"])
            .output()
            .context("Failed to execute systemctl restart")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to restart Xray-core: {}", stderr);
        }

        info!("Xray-core service restarted successfully");
        Ok(())
    }

    /// Get CPU usage percentage
    async fn get_cpu_usage() -> Result<f64> {
        // This is a simplified implementation
        // In production, you might want to use a proper system monitoring library
        let output = Command::new("sh")
            .args(&[
                "-c",
                "top -bn1 | grep 'Cpu(s)' | sed 's/.*, *\\([0-9.]*\\)%* id.*/\\1/' | awk '{print 100 - $1}'",
            ])
            .output()
            .context("Failed to get CPU usage")?;

        let cpu_str = String::from_utf8_lossy(&output.stdout);
        let cpu_usage = cpu_str.trim().parse::<f64>().unwrap_or(0.0);

        Ok(cpu_usage)
    }

    /// Get memory usage percentage
    async fn get_memory_usage() -> Result<f64> {
        // This is a simplified implementation
        let output = Command::new("sh")
            .args(&[
                "-c",
                "free | grep Mem | awk '{print ($3/$2) * 100.0}'",
            ])
            .output()
            .context("Failed to get memory usage")?;

        let mem_str = String::from_utf8_lossy(&output.stdout);
        let mem_usage = mem_str.trim().parse::<f64>().unwrap_or(0.0);

        Ok(mem_usage)
    }

    /// Get number of active connections
    async fn get_active_connections(config: &Config) -> Result<u32> {
        // Query Xray API for connection count
        let api_url = format!("http://127.0.0.1:{}/stats/query", config.xray_api_port);

        let response = reqwest::Client::new()
            .post(&api_url)
            .json(&serde_json::json!({
                "pattern": "inbound>>>",
                "reset": false
            }))
            .timeout(Duration::from_secs(5))
            .send()
            .await;

        match response {
            Ok(resp) if resp.status().is_success() => {
                // Parse response and count connections
                // This is a simplified implementation
                Ok(0)
            }
            _ => Ok(0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heartbeat_data_serialization() {
        let heartbeat = HeartbeatData {
            node_id: "node-001".to_string(),
            secret: "secret-key".to_string(),
            status: "online".to_string(),
            cpu_usage: 45.2,
            memory_usage: 60.5,
            active_connections: 123,
        };

        let json = serde_json::to_string(&heartbeat).unwrap();
        let deserialized: HeartbeatData = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.node_id, "node-001");
        assert_eq!(deserialized.secret, "secret-key");
        assert_eq!(deserialized.status, "online");
        assert_eq!(deserialized.cpu_usage, 45.2);
        assert_eq!(deserialized.memory_usage, 60.5);
        assert_eq!(deserialized.active_connections, 123);
    }

    #[test]
    fn test_node_health_serialization() {
        let health = NodeHealth {
            node_id: "node-001".to_string(),
            status: "online".to_string(),
            cpu_usage: 45.2,
            memory_usage: 60.5,
            active_connections: 123,
            xray_status: "running".to_string(),
        };

        let json = serde_json::to_string(&health).unwrap();
        let deserialized: NodeHealth = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.node_id, "node-001");
        assert_eq!(deserialized.status, "online");
        assert_eq!(deserialized.xray_status, "running");
    }
}
