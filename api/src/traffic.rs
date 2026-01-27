use anyhow::{Context, Result};
use redis::aio::ConnectionManager;
use redis::{AsyncCommands, streams::StreamReadReply};
use sqlx::PgPool;
use std::collections::HashMap;
use std::time::Duration;
use tokio::time;

/// Traffic report data structure
#[derive(Debug, Clone)]
pub struct TrafficReport {
    pub node_id: i64,
    pub user_id: i64,
    pub upload: i64,
    pub download: i64,
    pub timestamp: i64,
}

/// Traffic processor for consuming traffic reports from Redis Streams
pub struct TrafficProcessor {
    redis_conn: ConnectionManager,
    db_pool: PgPool,
    stream_name: String,
    consumer_group: String,
    consumer_name: String,
    batch_size: usize,
}

impl TrafficProcessor {
    /// Create a new traffic processor
    pub fn new(
        redis_conn: ConnectionManager,
        db_pool: PgPool,
        stream_name: String,
        consumer_group: String,
        consumer_name: String,
    ) -> Self {
        Self {
            redis_conn,
            db_pool,
            stream_name,
            consumer_group,
            consumer_name,
            batch_size: 100,
        }
    }

    /// Initialize the consumer group (create if not exists)
    pub async fn initialize(&mut self) -> Result<()> {
        let mut conn = self.redis_conn.clone();
        
        // Try to create consumer group, ignore error if it already exists
        let _: Result<String, redis::RedisError> = redis::cmd("XGROUP")
            .arg("CREATE")
            .arg(&self.stream_name)
            .arg(&self.consumer_group)
            .arg("0")
            .arg("MKSTREAM")
            .query_async(&mut conn)
            .await;

        tracing::info!(
            "Traffic processor initialized for stream: {}, group: {}",
            self.stream_name,
            self.consumer_group
        );

        Ok(())
    }

    /// Start consuming traffic reports from Redis Streams
    pub async fn start(&mut self) -> Result<()> {
        tracing::info!("Starting traffic processor...");

        loop {
            match self.process_batch().await {
                Ok(count) => {
                    if count > 0 {
                        tracing::debug!("Processed {} traffic reports", count);
                    }
                }
                Err(e) => {
                    tracing::error!("Error processing traffic batch: {}", e);
                    // Wait a bit before retrying
                    time::sleep(Duration::from_secs(5)).await;
                }
            }

            // Small delay between batches
            time::sleep(Duration::from_millis(100)).await;
        }
    }

    /// Process a batch of traffic reports
    async fn process_batch(&mut self) -> Result<usize> {
        let mut conn = self.redis_conn.clone();

        // Read from stream using consumer group
        let opts = redis::streams::StreamReadOptions::default()
            .group(&self.consumer_group, &self.consumer_name)
            .count(self.batch_size)
            .block(1000); // Block for 1 second if no messages

        let results: StreamReadReply = conn
            .xread_options(&[&self.stream_name], &[">"], &opts)
            .await
            .context("Failed to read from stream")?;

        if results.keys.is_empty() {
            return Ok(0);
        }

        let mut reports = Vec::new();
        let mut message_ids = Vec::new();

        // Parse messages
        for stream_key in &results.keys {
            for stream_id in &stream_key.ids {
                message_ids.push(stream_id.id.clone());

                // Parse traffic report from stream message
                let report = self.parse_traffic_report(&stream_id.map)?;
                reports.push(report);
            }
        }

        if reports.is_empty() {
            return Ok(0);
        }

        // Aggregate traffic by user
        let aggregated = self.aggregate_traffic(&reports);

        // Batch update user traffic in database
        self.update_user_traffic_batch(&aggregated).await?;

        // Acknowledge processed messages
        self.acknowledge_messages(&message_ids).await?;

        Ok(reports.len())
    }

    /// Parse traffic report from Redis stream message
    fn parse_traffic_report(&self, data: &HashMap<String, redis::Value>) -> Result<TrafficReport> {
        let node_id = self.get_i64_field(data, "node_id")?;
        let user_id = self.get_i64_field(data, "user_id")?;
        let upload = self.get_i64_field(data, "upload")?;
        let download = self.get_i64_field(data, "download")?;
        let timestamp = self.get_i64_field(data, "timestamp")?;

        Ok(TrafficReport {
            node_id,
            user_id,
            upload,
            download,
            timestamp,
        })
    }

    /// Helper to extract i64 field from Redis value map
    fn get_i64_field(&self, data: &HashMap<String, redis::Value>, field: &str) -> Result<i64> {
        let value = data
            .get(field)
            .ok_or_else(|| anyhow::anyhow!("Missing field: {}", field))?;

        match value {
            redis::Value::Data(bytes) => {
                let s = String::from_utf8(bytes.clone())
                    .context(format!("Invalid UTF-8 in field: {}", field))?;
                s.parse::<i64>()
                    .context(format!("Failed to parse i64 from field: {}", field))
            }
            redis::Value::Int(i) => Ok(*i),
            _ => Err(anyhow::anyhow!("Unexpected value type for field: {}", field)),
        }
    }

    /// Aggregate traffic reports by user_id
    pub fn aggregate_traffic(&self, reports: &[TrafficReport]) -> HashMap<i64, (i64, i64)> {
        let mut aggregated: HashMap<i64, (i64, i64)> = HashMap::new();

        for report in reports {
            let entry = aggregated.entry(report.user_id).or_insert((0, 0));
            entry.0 += report.upload;
            entry.1 += report.download;
        }

        aggregated
    }

    /// Batch update user traffic in database
    async fn update_user_traffic_batch(&self, aggregated: &HashMap<i64, (i64, i64)>) -> Result<()> {
        for (user_id, (upload, download)) in aggregated {
            // Update user traffic_used
            let result = sqlx::query(
                r#"
                UPDATE users
                SET traffic_used = traffic_used + $2,
                    updated_at = NOW()
                WHERE id = $1
                "#,
            )
            .bind(user_id)
            .bind(upload + download)
            .execute(&self.db_pool)
            .await;

            if let Err(e) = result {
                tracing::error!("Failed to update traffic for user {}: {}", user_id, e);
                // Continue processing other users
            }
        }

        Ok(())
    }

    /// Acknowledge processed messages
    async fn acknowledge_messages(&mut self, message_ids: &[String]) -> Result<()> {
        if message_ids.is_empty() {
            return Ok(());
        }

        let mut conn = self.redis_conn.clone();

        for message_id in message_ids {
            let _: Result<i64, redis::RedisError> = redis::cmd("XACK")
                .arg(&self.stream_name)
                .arg(&self.consumer_group)
                .arg(message_id)
                .query_async(&mut conn)
                .await;
        }

        Ok(())
    }
}

/// Add traffic report to Redis stream (called by Node Agent)
pub async fn add_traffic_report(
    redis_conn: &mut ConnectionManager,
    stream_name: &str,
    node_id: i64,
    user_id: i64,
    upload: i64,
    download: i64,
) -> Result<String> {
    let timestamp = chrono::Utc::now().timestamp();

    let message_id: String = redis::cmd("XADD")
        .arg(stream_name)
        .arg("*") // Auto-generate ID
        .arg("node_id")
        .arg(node_id)
        .arg("user_id")
        .arg(user_id)
        .arg("upload")
        .arg(upload)
        .arg("download")
        .arg(download)
        .arg("timestamp")
        .arg(timestamp)
        .query_async(redis_conn)
        .await
        .context("Failed to add traffic report to stream")?;

    Ok(message_id)
}

/// Check if user has exceeded traffic quota
pub async fn check_traffic_quota(db_pool: &PgPool, user_id: i64) -> Result<bool> {
    let user = sqlx::query_as::<_, crate::models::User>(
        r#"
        SELECT * FROM users WHERE id = $1
        "#,
    )
    .bind(user_id)
    .fetch_optional(db_pool)
    .await
    .context("Failed to fetch user")?;

    match user {
        Some(u) => {
            // User has exceeded quota if traffic_used >= traffic_quota
            Ok(u.traffic_used < u.traffic_quota)
        }
        None => Ok(false), // User not found, deny access
    }
}

/// Check if user has valid active package with remaining traffic
pub async fn has_valid_package_with_traffic(db_pool: &PgPool, user_id: i64) -> Result<bool> {
    let result = sqlx::query_as::<_, (i64,)>(
        r#"
        SELECT COUNT(*) FROM user_packages
        WHERE user_id = $1
          AND status = 'active'
          AND expires_at > NOW()
          AND traffic_used < traffic_quota
        "#,
    )
    .bind(user_id)
    .fetch_one(db_pool)
    .await
    .context("Failed to check user packages")?;

    Ok(result.0 > 0)
}

/// Get user traffic statistics
pub async fn get_user_traffic_stats(
    db_pool: &PgPool,
    user_id: i64,
) -> Result<Option<(i64, i64, i64)>> {
    let user = sqlx::query_as::<_, crate::models::User>(
        r#"
        SELECT * FROM users WHERE id = $1
        "#,
    )
    .bind(user_id)
    .fetch_optional(db_pool)
    .await
    .context("Failed to fetch user")?;

    match user {
        Some(u) => {
            let remaining = if u.traffic_quota > u.traffic_used {
                u.traffic_quota - u.traffic_used
            } else {
                0
            };
            Ok(Some((u.traffic_quota, u.traffic_used, remaining)))
        }
        None => Ok(None),
    }
}

/// Persist traffic logs from memory to database
/// This should be called periodically (e.g., every hour) to aggregate and persist traffic data
pub async fn persist_traffic_logs(db_pool: &PgPool) -> Result<usize> {
    // Get all users with traffic usage
    let users_with_traffic = sqlx::query_as::<_, (i64, i64)>(
        r#"
        SELECT id, traffic_used
        FROM users
        WHERE traffic_used > 0
        ORDER BY id
        "#,
    )
    .fetch_all(db_pool)
    .await
    .context("Failed to fetch users with traffic")?;

    let mut persisted_count = 0;

    for (user_id, traffic_used) in users_with_traffic {
        // For each user, create a traffic log entry
        // In a real system, you would aggregate from Redis or other temporary storage
        // For now, we'll create a log entry with the current traffic_used value
        
        // Note: In production, you would track which node the traffic came from
        // For this implementation, we'll use node_id = 0 to indicate aggregated traffic
        let result = sqlx::query(
            r#"
            INSERT INTO traffic_logs (user_id, node_id, upload, download, recorded_at)
            VALUES ($1, 0, 0, $2, NOW())
            "#,
        )
        .bind(user_id)
        .bind(traffic_used)
        .execute(db_pool)
        .await;

        if result.is_ok() {
            persisted_count += 1;
        }
    }

    Ok(persisted_count)
}

/// Background task to periodically persist traffic logs
/// This function should be run in a separate tokio task
pub async fn start_traffic_log_persistence_task(db_pool: PgPool, interval_hours: u64) {
    let mut interval = time::interval(Duration::from_secs(interval_hours * 3600));

    loop {
        interval.tick().await;

        tracing::info!("Starting traffic log persistence task");

        match persist_traffic_logs(&db_pool).await {
            Ok(count) => {
                tracing::info!("Persisted {} traffic log entries", count);
            }
            Err(e) => {
                tracing::error!("Failed to persist traffic logs: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aggregate_traffic() {
        let reports = vec![
            TrafficReport {
                node_id: 1,
                user_id: 100,
                upload: 1000,
                download: 2000,
                timestamp: 1234567890,
            },
            TrafficReport {
                node_id: 1,
                user_id: 100,
                upload: 500,
                download: 1500,
                timestamp: 1234567891,
            },
            TrafficReport {
                node_id: 2,
                user_id: 200,
                upload: 3000,
                download: 4000,
                timestamp: 1234567892,
            },
        ];

        let redis_url = "redis://127.0.0.1:6379";
        let redis_conn = tokio::runtime::Runtime::new().unwrap().block_on(async {
            redis::Client::open(redis_url)
                .unwrap()
                .get_connection_manager()
                .await
                .unwrap()
        });

        let db_url = "postgres://user:pass@localhost/db";
        let db_pool = tokio::runtime::Runtime::new().unwrap().block_on(async {
            sqlx::postgres::PgPoolOptions::new()
                .max_connections(1)
                .connect(db_url)
                .await
                .unwrap()
        });

        let processor = TrafficProcessor::new(
            redis_conn,
            db_pool,
            "traffic_stream".to_string(),
            "traffic_processor".to_string(),
            "consumer1".to_string(),
        );

        let aggregated = processor.aggregate_traffic(&reports);

        assert_eq!(aggregated.len(), 2);
        assert_eq!(aggregated.get(&100), Some(&(1500, 3500)));
        assert_eq!(aggregated.get(&200), Some(&(3000, 4000)));
    }

    #[test]
    fn test_traffic_report_creation() {
        let report = TrafficReport {
            node_id: 1,
            user_id: 100,
            upload: 1024,
            download: 2048,
            timestamp: 1234567890,
        };

        assert_eq!(report.node_id, 1);
        assert_eq!(report.user_id, 100);
        assert_eq!(report.upload, 1024);
        assert_eq!(report.download, 2048);
        assert_eq!(report.timestamp, 1234567890);
    }
}
