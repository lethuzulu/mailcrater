use std::path::Path as FsPath;

use anyhow::Result;
use backend::api;
use backend::config::Config;
use backend::storage::Storage;
use mailcrater_core::connection::MailServer;

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

    let storage = Storage::new(FsPath::new(&config.data_dir)).await?;

    let (tx, mut rx) = channel(config.channel_capacity);

    let consumer_storage = storage.clone();
    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Some(new_message) => match consumer_storage.insert_message(new_message).await {
                    Ok(_id) => {}
                    Err(e) => {
                        tracing::error!(?e, "failed to insert message into storage");
                    }
                },
                None => break,
            }
        }
    });

    let mail_server = MailServer::new(
        ("127.0.0.1", config.smtp_port),
        tx,
        config.max_message_size,
    )
    .await?;
    tokio::spawn(async move {
        mail_server.serve().await;
    });

    let app = api::app(storage);
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", config.http_port)).await?;

    info!(
        smtp_port = config.smtp_port,
        http_port = config.http_port,
        "smtp and http servers listening"
    );

    axum::serve(listener, app).await?;

    Ok(())
}
