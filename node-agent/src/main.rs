use anyhow::Result;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub mod config;
pub mod health;
pub mod sync;
pub mod traffic;
pub mod users;

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables
    dotenv::dotenv().ok();

    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "node_agent=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting VPN Node Agent");

    // Load configuration
    let config = std::sync::Arc::new(config::Config::from_env()?);
    tracing::info!("Configuration loaded");
    tracing::info!("API URL: {}", config.api_url);
    tracing::info!("Node ID: {}", config.node_id);

    // Node Agent implementation will be completed in task 18
    tracing::info!("Node Agent initialized successfully");

    // Keep the agent running
    tokio::signal::ctrl_c().await?;
    tracing::info!("Shutting down Node Agent");

    Ok(())
}
