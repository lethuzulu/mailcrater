use std::{fs::create_dir_all, path::Path};

use anyhow::Result;
use chrono::Utc;
use sqlx::{SqlitePool, sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions}};
use tracing::info;
use uuid::Uuid;

use crate::message::NewMessage;

pub struct Storage {
    pub pool: SqlitePool,
}


impl Storage {
    pub async fn new(dir: &Path) -> Result<Self> {
        create_dir_all(dir)?;

        let db_path = dir.join("mailcrater.db");

        let options = SqliteConnectOptions::new()
            .filename(db_path)
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal);

        let pool = SqlitePoolOptions::new()
            .connect_with(options)
            .await?;

        sqlx::migrate!("./migrations").run(&pool).await?;

        Ok(Self { pool })
    }

    pub async fn insert_message(&self, msg: NewMessage) -> Result<()> {
        let NewMessage {
            from_addr,
            to_addrs,
            cc_addrs,
            subject,
            body_text,
            body_html,
            raw_size,
            raw_source,
            attachments,
        } = msg;

        let id = Uuid::new_v4();
        // TODO: received_at is when we stored the message, not the message's own `Date:`.
        let received_at = Utc::now().to_rfc3339();

        let to_addrs_json = serde_json::to_string(&to_addrs)?;
        let cc_addrs_json = if cc_addrs.is_empty() {
            None
        } else {
            Some(serde_json::to_string(&cc_addrs)?)
        };

        let mut tx = self.pool.begin().await?;

        sqlx::query(
            "INSERT INTO messages
                (id, received_at, from_addr, to_addrs, cc_addrs, subject, body_text, body_html, raw_size, raw_source)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id.to_string())
        .bind(&received_at)
        .bind(&from_addr)
        .bind(&to_addrs_json)
        .bind(&cc_addrs_json)
        .bind(&subject)
        .bind(&body_text)
        .bind(&body_html)
        .bind(raw_size as i64)
        .bind(raw_source)
        .execute(&mut *tx)
        .await?;

        for attachment in attachments {
            sqlx::query(
                "INSERT INTO attachments (id, message_id, filename, content_type, size, data)
                 VALUES (?, ?, ?, ?, ?, ?)",
            )
            .bind(Uuid::new_v4().to_string())
            .bind(id.to_string())
            .bind(&attachment.filename)
            .bind(&attachment.content_type)
            .bind(attachment.size as i64)
            .bind(attachment.data)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        info!(%id, "stored message");

        Ok(())
    }
}
