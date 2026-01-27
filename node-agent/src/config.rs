use anyhow::{Context, Result};
use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub api_url: String,
    pub node_id: String,
    pub node_secret: String,
    pub xray_api_port: u16,
    pub traffic_report_interval: u64,
    pub heartbeat_interval: u64,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Config {
            api_url: env::var("API_URL")
                .context("API_URL must be set")?,
            node_id: env::var("NODE_ID")
                .context("NODE_ID must be set")?,
            node_secret: env::var("NODE_SECRET")
                .context("NODE_SECRET must be set")?,
            xray_api_port: env::var("XRAY_API_PORT")
                .unwrap_or_else(|_| "10085".to_string())
                .parse()
                .context("XRAY_API_PORT must be a valid number")?,
            traffic_report_interval: env::var("TRAFFIC_REPORT_INTERVAL")
                .unwrap_or_else(|_| "30".to_string())
                .parse()
                .context("TRAFFIC_REPORT_INTERVAL must be a valid number")?,
            heartbeat_interval: env::var("HEARTBEAT_INTERVAL")
                .unwrap_or_else(|_| "60".to_string())
                .parse()
                .context("HEARTBEAT_INTERVAL must be a valid number")?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::sync::Mutex;

    // Use a mutex to ensure tests don't run concurrently and interfere with each other
    static TEST_MUTEX: Mutex<()> = Mutex::new(());

    #[test]
    fn test_config_from_env() {
        let _lock = TEST_MUTEX.lock().unwrap();
        
        env::set_var("API_URL", "https://api.example.com");
        env::set_var("NODE_ID", "node-001");
        env::set_var("NODE_SECRET", "secret-key");
        env::remove_var("XRAY_API_PORT");
        env::remove_var("TRAFFIC_REPORT_INTERVAL");
        env::remove_var("HEARTBEAT_INTERVAL");

        let config = Config::from_env().unwrap();
        assert_eq!(config.api_url, "https://api.example.com");
        assert_eq!(config.node_id, "node-001");
        assert_eq!(config.node_secret, "secret-key");
        assert_eq!(config.xray_api_port, 10085);
        assert_eq!(config.traffic_report_interval, 30);
        assert_eq!(config.heartbeat_interval, 60);
    }

    #[test]
    fn test_config_with_custom_values() {
        let _lock = TEST_MUTEX.lock().unwrap();
        
        env::set_var("API_URL", "https://api.example.com");
        env::set_var("NODE_ID", "node-002");
        env::set_var("NODE_SECRET", "secret-key-2");
        env::set_var("XRAY_API_PORT", "20085");
        env::set_var("TRAFFIC_REPORT_INTERVAL", "60");
        env::set_var("HEARTBEAT_INTERVAL", "120");

        let config = Config::from_env().unwrap();
        assert_eq!(config.xray_api_port, 20085);
        assert_eq!(config.traffic_report_interval, 60);
        assert_eq!(config.heartbeat_interval, 120);
    }

    // Feature: vpn-subscription-platform, Property 20: 环境变量配置正确性
    // **Validates: Requirements 13.4**
    // For any configuration parameter (database connection, Redis connection, etc.),
    // if provided through environment variables, the system should correctly read
    // and use that value rather than the default value.
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]
        
        #[test]
        fn test_env_var_config_correctness(
            api_url in "https?://[a-z0-9]+\\.[a-z]{2,}",
            node_id in "[a-z0-9-]{5,20}",
            node_secret in "[a-zA-Z0-9_]{10,50}",
            xray_port in 1024u16..65535u16,
            traffic_interval in 10u64..300u64,
            heartbeat_interval in 30u64..600u64,
        ) {
            let _lock = TEST_MUTEX.lock().unwrap();
            
            // Set environment variables with generated values
            env::set_var("API_URL", &api_url);
            env::set_var("NODE_ID", &node_id);
            env::set_var("NODE_SECRET", &node_secret);
            env::set_var("XRAY_API_PORT", xray_port.to_string());
            env::set_var("TRAFFIC_REPORT_INTERVAL", traffic_interval.to_string());
            env::set_var("HEARTBEAT_INTERVAL", heartbeat_interval.to_string());

            // Load configuration
            let config = Config::from_env().expect("Config should load from env vars");

            // Verify that the loaded config matches the environment variables
            prop_assert_eq!(&config.api_url, &api_url);
            prop_assert_eq!(&config.node_id, &node_id);
            prop_assert_eq!(&config.node_secret, &node_secret);
            prop_assert_eq!(config.xray_api_port, xray_port);
            prop_assert_eq!(config.traffic_report_interval, traffic_interval);
            prop_assert_eq!(config.heartbeat_interval, heartbeat_interval);
        }

        #[test]
        fn test_default_values_when_optional_vars_missing(
            api_url in "https?://[a-z0-9]+\\.[a-z]{2,}",
            node_id in "[a-z0-9-]{5,20}",
            node_secret in "[a-zA-Z0-9_]{10,50}",
        ) {
            let _lock = TEST_MUTEX.lock().unwrap();
            
            // Set only required environment variables
            env::set_var("API_URL", &api_url);
            env::set_var("NODE_ID", &node_id);
            env::set_var("NODE_SECRET", &node_secret);
            
            // Remove optional environment variables
            env::remove_var("XRAY_API_PORT");
            env::remove_var("TRAFFIC_REPORT_INTERVAL");
            env::remove_var("HEARTBEAT_INTERVAL");

            // Load configuration
            let config = Config::from_env().expect("Config should load with defaults");

            // Verify required values are set correctly
            prop_assert_eq!(&config.api_url, &api_url);
            prop_assert_eq!(&config.node_id, &node_id);
            prop_assert_eq!(&config.node_secret, &node_secret);
            
            // Verify default values are used for optional parameters
            prop_assert_eq!(config.xray_api_port, 10085);
            prop_assert_eq!(config.traffic_report_interval, 30);
            prop_assert_eq!(config.heartbeat_interval, 60);
        }
    }
}
