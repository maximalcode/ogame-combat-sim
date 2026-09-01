#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "combat_api=info".into()),
        )
        .init();
    let config = combat_api::ServerConfig::from_env()
        .map_err(|error| format!("invalid server configuration: {error}"))?;
    combat_api::run(config).await
}
