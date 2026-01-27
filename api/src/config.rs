use anyhow::{Context, Result};
use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub redis_url: String,
    pub jwt_secret: String,
    pub jwt_expiration: i64,
    pub host: String,
    pub port: u16,
    pub cors_origins: Vec<String>,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Config {
            database_url: env::var("DATABASE_URL")
                .context("DATABASE_URL must be set")?,
            redis_url: env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string()),
            jwt_secret: env::var("JWT_SECRET")
                .context("JWT_SECRET must be set")?,
            jwt_expiration: env::var("JWT_EXPIRATION")
                .unwrap_or_else(|_| "86400".to_string())
                .parse()
                .context("JWT_EXPIRATION must be a valid number")?,
            host: env::var("API_HOST")
                .unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: env::var("API_PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .context("API_PORT must be a valid number")?,
            cors_origins: env::var("CORS_ORIGINS")
                .unwrap_or_else(|_| "http://localhost:3000".to_string())
                .split(',')
                .map(|s| s.trim().to_string())
                .collect(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Use a mutex to ensure tests don't run concurrently and interfere with each other
    static TEST_MUTEX: Mutex<()> = Mutex::new(());

    #[test]
    fn test_config_from_env_loads_successfully() {
        let _lock = TEST_MUTEX.lock().unwrap();
        
        // Save current values
        let db_url = env::var("DATABASE_URL").ok();
        let jwt_secret = env::var("JWT_SECRET").ok();
        
        // Set required environment variables
        env::set_var("DATABASE_URL", "postgres://test:test@localhost/test");
        env::set_var("JWT_SECRET", "test-secret");

        let config = Config::from_env();
        assert!(config.is_ok());
        
        let config = config.unwrap();
        // Just verify the required fields are set
        assert!(!config.database_url.is_empty());
        assert!(!config.jwt_secret.is_empty());
        assert!(config.jwt_expiration > 0);
        assert!(config.port > 0);
        
        // Restore original values
        env::remove_var("DATABASE_URL");
        env::remove_var("JWT_SECRET");
        if let Some(url) = db_url {
            env::set_var("DATABASE_URL", url);
        }
        if let Some(secret) = jwt_secret {
            env::set_var("JWT_SECRET", secret);
        }
    }

    #[test]
    fn test_config_missing_required_fields() {
        let _lock = TEST_MUTEX.lock().unwrap();
        
        // Save current values
        let db_url = env::var("DATABASE_URL").ok();
        let jwt_secret = env::var("JWT_SECRET").ok();
        
        // Remove required environment variables
        env::remove_var("DATABASE_URL");
        env::remove_var("JWT_SECRET");

        let config = Config::from_env();
        // Should fail when required fields are missing
        assert!(config.is_err());
        
        // Restore for other tests
        if let Some(url) = db_url {
            env::set_var("DATABASE_URL", url);
        }
        if let Some(secret) = jwt_secret {
            env::set_var("JWT_SECRET", secret);
        }
    }
}
