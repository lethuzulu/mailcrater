use std::path::Path;

use anyhow::Result;
use mailcrater::config::Config;
use mailcrater::connection::MailServer;
use mailcrater::storage::Storage;

use tokio::sync::mpsc::channel;
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

    let storage = Storage::new(Path::new(&config.data_dir)).await?;

    let (tx, mut rx) = channel(1024);

    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Some(new_message) => match storage.insert_message(new_message).await {
                    Ok(()) => {}
                    Err(e) => {
                        tracing::error!(?e, "failed to insert message into storage");
                    }
                },
                None => break,
            }
        }
    });

    let mail_server = MailServer::new(("127.0.0.1", config.smtp_port), tx).await?;
    mail_server.serve().await;

    Ok(())
}
