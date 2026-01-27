use anyhow::Result;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod models;
mod db;
mod cache;
mod clash;
mod handlers;
mod middleware;
mod traffic;
mod utils;

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables
    dotenv::dotenv().ok();

    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "api=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting VPN Subscription Platform API");

    // Load configuration
    let config = config::Config::from_env()?;
    tracing::info!("Configuration loaded");

    // Initialize database connection pool
    let db_pool = db::create_pool(&config.database_url).await?;
    tracing::info!("Database connection pool created");

    // Initialize Redis connection
    let redis_client = redis::Client::open(config.redis_url.as_str())?;
    let redis_conn = redis_client.get_connection_manager().await?;
    tracing::info!("Redis connection established");

    // Build application router
    let app = handlers::create_router(db_pool, redis_conn, config.clone());

    // Start server
    let addr = format!("{}:{}", config.host, config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("Server listening on {}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}
