use anyhow::{Context, Result};
use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{interval, Duration};
use tracing::{error, info, warn};

use crate::config::Config;

/// Traffic statistics for a user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserTraffic {
    pub user_email: String,
    pub upload: u64,
    pub download: u64,
}

/// Traffic data to be reported to Redis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrafficReport {
    pub node_id: String,
    pub user_id: String,
    pub upload: u64,
    pub download: u64,
    pub timestamp: i64,
}

/// Traffic reporter that collects and reports traffic data
pub struct TrafficReporter {
    config: Arc<Config>,
    redis_client: Option<ConnectionManager>,
    xray_api_client: reqwest::Client,
}

impl TrafficReporter {
    pub fn new(config: Arc<Config>, redis_client: Option<ConnectionManager>) -> Self {
        Self {
            config,
            redis_client,
            xray_api_client: reqwest::Client::new(),
        }
    }

    /// Start the traffic reporting loop
    pub async fn start(&self) -> Result<()> {
        let redis_client = match &self.redis_client {
            Some(client) => client.clone(),
            None => {
                warn!("Redis client not available, traffic reporting disabled");
                return Ok(());
            }
        };

        let config = Arc::clone(&self.config);
        let xray_api_client = self.xray_api_client.clone();

        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(config.traffic_report_interval));

            loop {
                ticker.tick().await;

                match Self::collect_and_report_traffic(
                    &config,
                    &xray_api_client,
                    &redis_client,
                )
                .await
                {
                    Ok(count) => {
                        if count > 0 {
                            info!("Successfully reported traffic for {} users", count);
                        }
                    }
                    Err(e) => {
                        error!("Failed to collect and report traffic: {}", e);
                    }
                }
            }
        });

        info!(
            "Traffic reporter started with interval: {} seconds",
            self.config.traffic_report_interval
        );

        Ok(())
    }

    /// Collect traffic data from Xray-core and report to Redis
    async fn collect_and_report_traffic(
        config: &Config,
        xray_api_client: &reqwest::Client,
        redis_client: &ConnectionManager,
    ) -> Result<usize> {
        // Fetch traffic statistics from Xray-core API
        let traffic_data = Self::fetch_xray_traffic(config, xray_api_client).await?;

        if traffic_data.is_empty() {
            return Ok(0);
        }

        // Report traffic data to Redis Streams
        let mut redis_conn = redis_client.clone();
        let timestamp = chrono::Utc::now().timestamp();

        for user_traffic in &traffic_data {
            // Extract user ID from email (assuming email format: user_id@domain)
            let user_id = user_traffic
                .user_email
                .split('@')
                .next()
                .unwrap_or(&user_traffic.user_email);

            // Add to Redis Stream
            let stream_key = "traffic_stream";
            
            // Convert values to strings with proper lifetimes
            let upload_str = user_traffic.upload.to_string();
            let download_str = user_traffic.download.to_string();
            let timestamp_str = timestamp.to_string();
            
            let fields = vec![
                ("node_id", config.node_id.as_str()),
                ("user_id", user_id),
                ("upload", upload_str.as_str()),
                ("download", download_str.as_str()),
                ("timestamp", timestamp_str.as_str()),
            ];

            redis_conn
                .xadd::<_, _, _, _, ()>(stream_key, "*", &fields)
                .await
                .context("Failed to add traffic data to Redis stream")?;
        }

        Ok(traffic_data.len())
    }

    /// Fetch traffic statistics from Xray-core API
    async fn fetch_xray_traffic(
        config: &Config,
        xray_api_client: &reqwest::Client,
    ) -> Result<Vec<UserTraffic>> {
        let api_url = format!("http://127.0.0.1:{}", config.xray_api_port);

        // First, get the list of users
        // This is a simplified implementation - actual Xray API might differ
        let stats_url = format!("{}/stats/query", api_url);

        let response = xray_api_client
            .post(&stats_url)
            .json(&serde_json::json!({
                "pattern": "user>>>",
                "reset": true
            }))
            .send()
            .await;

        match response {
            Ok(resp) if resp.status().is_success() => {
                let stats: XrayStatsResponse = resp
                    .json()
                    .await
                    .context("Failed to parse Xray stats response")?;

                let traffic_data = Self::parse_xray_stats(&stats)?;
                Ok(traffic_data)
            }
            Ok(resp) => {
                warn!("Xray API returned non-success status: {}", resp.status());
                Ok(Vec::new())
            }
            Err(e) => {
                warn!("Failed to fetch Xray stats: {}", e);
                Ok(Vec::new())
            }
        }
    }

    /// Parse Xray stats response into user traffic data
    fn parse_xray_stats(stats: &XrayStatsResponse) -> Result<Vec<UserTraffic>> {
        let mut traffic_map: HashMap<String, UserTraffic> = HashMap::new();

        for stat in &stats.stat {
            // Parse stat name: "user>>>email>>>traffic>>>uplink" or "user>>>email>>>traffic>>>downlink"
            let parts: Vec<&str> = stat.name.split(">>>").collect();
            if parts.len() < 4 || parts[0] != "user" {
                continue;
            }

            let email = parts[1].to_string();
            let direction = parts[3];

            let traffic = traffic_map.entry(email.clone()).or_insert(UserTraffic {
                user_email: email,
                upload: 0,
                download: 0,
            });

            match direction {
                "uplink" => traffic.upload = stat.value as u64,
                "downlink" => traffic.download = stat.value as u64,
                _ => {}
            }
        }

        Ok(traffic_map.into_values().collect())
    }
}

/// Xray stats API response
#[derive(Debug, Deserialize)]
struct XrayStatsResponse {
    stat: Vec<XrayStat>,
}

#[derive(Debug, Deserialize)]
struct XrayStat {
    name: String,
    value: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_xray_stats() {
        let stats = XrayStatsResponse {
            stat: vec![
                XrayStat {
                    name: "user>>>user1@example.com>>>traffic>>>uplink".to_string(),
                    value: 1024000,
                },
                XrayStat {
                    name: "user>>>user1@example.com>>>traffic>>>downlink".to_string(),
                    value: 2048000,
                },
                XrayStat {
                    name: "user>>>user2@example.com>>>traffic>>>uplink".to_string(),
                    value: 512000,
                },
                XrayStat {
                    name: "user>>>user2@example.com>>>traffic>>>downlink".to_string(),
                    value: 1024000,
                },
            ],
        };

        let traffic_data = TrafficReporter::parse_xray_stats(&stats).unwrap();

        assert_eq!(traffic_data.len(), 2);

        let user1 = traffic_data
            .iter()
            .find(|t| t.user_email == "user1@example.com")
            .unwrap();
        assert_eq!(user1.upload, 1024000);
        assert_eq!(user1.download, 2048000);

        let user2 = traffic_data
            .iter()
            .find(|t| t.user_email == "user2@example.com")
            .unwrap();
        assert_eq!(user2.upload, 512000);
        assert_eq!(user2.download, 1024000);
    }

    #[test]
    fn test_traffic_report_serialization() {
        let report = TrafficReport {
            node_id: "node-001".to_string(),
            user_id: "user123".to_string(),
            upload: 1024000,
            download: 2048000,
            timestamp: 1234567890,
        };

        let json = serde_json::to_string(&report).unwrap();
        let deserialized: TrafficReport = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.node_id, "node-001");
        assert_eq!(deserialized.user_id, "user123");
        assert_eq!(deserialized.upload, 1024000);
        assert_eq!(deserialized.download, 2048000);
        assert_eq!(deserialized.timestamp, 1234567890);
    }
}
