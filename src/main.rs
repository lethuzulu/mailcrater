use anyhow::Result;
use mailcrater::config::Config;
use mailcrater::connection::MailServer;
use tracing::info;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::from_env();
    info!(?config, "starting mailcrater");

    let mail_server = MailServer::new(("127.0.0.1", config.smtp_port)).await?;
    mail_server.serve().await;

    Ok(())
}
