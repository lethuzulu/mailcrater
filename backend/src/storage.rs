use std::{fs::create_dir_all, path::Path};

use anyhow::Result;
use chrono::Utc;
use sqlx::{SqlitePool, sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions}};
use tracing::info;
use uuid::Uuid;

use mailcrater_core::message::MailMessage;

use crate::types::{AttachmentDownload, AttachmentMeta, MessageDetail, MessageSummary};

#[derive(Clone)]
pub struct Storage {
    pub pool: SqlitePool,
}

struct MessageRow {
    id: String,
    received_at: String,
    from_addr: String,
    to_addrs: String,
    cc_addrs: Option<String>,
    subject: Option<String>,
    body_text: Option<String>,
    body_html: Option<String>,
}

struct AttachmentRow {
    id: String,
    filename: Option<String>,
    content_type: Option<String>,
    size: i64,
}

struct MessageSummaryRow {
    id: String,
    received_at: String,
    from_addr: String,
    to_addrs: String,
    subject: Option<String>,
}

struct AttachmentDownloadRow {
    filename: Option<String>,
    content_type: Option<String>,
    data: Vec<u8>,
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

    pub async fn insert_message(&self, msg: MailMessage) -> Result<String> {
        let MailMessage {
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
        
        let received_at = Utc::now().to_rfc3339(); // TODO: received_at is when we stored the message, not the message's own `Date:`.

        let to_addrs_json = serde_json::to_string(&to_addrs)?;
        let cc_addrs_json = if cc_addrs.is_empty() {
            None
        } else {
            Some(serde_json::to_string(&cc_addrs)?)
        };

        let mut tx = self.pool.begin().await?;

        let id_str = id.to_string();
        let raw_size = raw_size as i64;

        sqlx::query!(
            "INSERT INTO messages
                (id, received_at, from_addr, to_addrs, cc_addrs, subject, body_text, body_html, raw_size, raw_source)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            id_str,
            received_at,
            from_addr,
            to_addrs_json,
            cc_addrs_json,
            subject,
            body_text,
            body_html,
            raw_size,
            raw_source,
        )
        .execute(&mut *tx)
        .await?;

        for attachment in attachments {
            let attachment_id = Uuid::new_v4().to_string();
            let size = attachment.size as i64;

            sqlx::query!(
                "INSERT INTO attachments (id, message_id, filename, content_type, size, data)
                 VALUES (?, ?, ?, ?, ?, ?)",
                attachment_id,
                id_str,
                attachment.filename,
                attachment.content_type,
                size,
                attachment.data,
            )
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        info!(%id, "stored message");

        Ok(id.to_string())
    }

    pub async fn get_message(&self, id: &str) -> Result<Option<MessageDetail>> {
        let row = sqlx::query_as!(
            MessageRow,
            "SELECT id, received_at, from_addr, to_addrs, cc_addrs, subject, body_text, body_html FROM messages WHERE id = ?",
            id,
        )
        .fetch_optional(&self.pool)
        .await?;

        let row = match row {
            Some(row) => row,
            None => return Ok(None),
        };

        let to_addrs: Vec<String> = serde_json::from_str(&row.to_addrs)?;

        let cc_addrs: Vec<String> = match row.cc_addrs {
            Some(json) => serde_json::from_str(&json)?,
            None => Vec::new(),
        };

        let attachment_rows = sqlx::query_as!(
            AttachmentRow,
            "SELECT id, filename, content_type, size FROM attachments WHERE message_id = ?",
            id,
        )
        .fetch_all(&self.pool)
        .await?;

        let mut attachments = Vec::new();
        for attachment_row in attachment_rows {
            attachments.push(AttachmentMeta {
                id: attachment_row.id,
                filename: attachment_row.filename,
                content_type: attachment_row.content_type,
                size: attachment_row.size,
            });
        }

        Ok(Some(MessageDetail {
            id: row.id,
            received_at: row.received_at,
            from_addr: row.from_addr,
            to_addrs,
            cc_addrs,
            subject: row.subject,
            body_text: row.body_text,
            body_html: row.body_html,
            attachments,
        }))
    }

    pub async fn list_messages(
        &self,
        search: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<MessageSummary>> {
        let rows = match search {
            Some(search) => {
                let pattern = format!("%{}%", search);
                let subject_pattern = pattern.clone();
                let from_addr_pattern = pattern.clone();
                let to_addrs_pattern = pattern;
                sqlx::query_as!(
                    MessageSummaryRow,
                    "SELECT id, received_at, from_addr, to_addrs, subject FROM messages
                     WHERE subject LIKE ? OR from_addr LIKE ? OR to_addrs LIKE ?
                     ORDER BY received_at DESC
                     LIMIT ? OFFSET ?",
                    subject_pattern,
                    from_addr_pattern,
                    to_addrs_pattern,
                    limit,
                    offset,
                )
                .fetch_all(&self.pool)
                .await?
            }
            None => {
                sqlx::query_as!(
                    MessageSummaryRow,
                    "SELECT id, received_at, from_addr, to_addrs, subject FROM messages
                     ORDER BY received_at DESC
                     LIMIT ? OFFSET ?",
                    limit,
                    offset,
                )
                .fetch_all(&self.pool)
                .await?
            }
        };

        let mut messages = Vec::new();
        for row in rows {
            let to_addrs: Vec<String> = serde_json::from_str(&row.to_addrs)?;
            messages.push(MessageSummary {
                id: row.id,
                received_at: row.received_at,
                from_addr: row.from_addr,
                to_addrs,
                subject: row.subject,
            });
        }

        Ok(messages)
    }

    pub async fn count_messages(&self, search: Option<&str>) -> Result<i64> {
        let count = match search {
            Some(search) => {
                let pattern = format!("%{}%", search);
                let subject_pattern = pattern.clone();
                let from_addr_pattern = pattern.clone();
                let to_addrs_pattern = pattern;
                sqlx::query_scalar!(
                    "SELECT COUNT(*) FROM messages WHERE subject LIKE ? OR from_addr LIKE ? OR to_addrs LIKE ?",
                    subject_pattern,
                    from_addr_pattern,
                    to_addrs_pattern,
                )
                .fetch_one(&self.pool)
                .await?
            }
            None => {
                sqlx::query_scalar!("SELECT COUNT(*) FROM messages")
                    .fetch_one(&self.pool)
                    .await?
            }
        };

        Ok(count)
    }

    pub async fn get_raw_source(&self, id: &str) -> Result<Option<Vec<u8>>> {
        let raw_source =
            sqlx::query_scalar!("SELECT raw_source FROM messages WHERE id = ?", id)
                .fetch_optional(&self.pool)
                .await?;

        Ok(raw_source)
    }

    pub async fn get_attachment(
        &self,
        message_id: &str,
        attachment_id: &str,
    ) -> Result<Option<AttachmentDownload>> {
        let row = sqlx::query_as!(
            AttachmentDownloadRow,
            "SELECT filename, content_type, data FROM attachments WHERE id = ? AND message_id = ?",
            attachment_id,
            message_id,
        )
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => Ok(Some(AttachmentDownload {
                filename: row.filename,
                content_type: row.content_type,
                data: row.data,
            })),
            None => Ok(None),
        }
    }

    
    pub async fn delete_message(&self, id: &str) -> Result<bool> {
        let result = sqlx::query!("DELETE FROM messages WHERE id = ?", id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn delete_all_messages(&self) -> Result<()> {
        sqlx::query!("DELETE FROM messages")
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}
