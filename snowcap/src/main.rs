use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or(EnvFilter::new("snowcap=info"));

    tracing_subscriber::fmt()
        .compact()
        .with_env_filter(env_filter)
        .init();

    tokio::task::spawn_blocking(|| snowcap::start(None))
        .await
        .unwrap();

    Ok(())
}
