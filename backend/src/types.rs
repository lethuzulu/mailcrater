use serde::Serialize;

#[derive(Serialize)]
pub struct MessageDetail {
    pub id: String,
    pub received_at: String,
    pub from_addr: String,
    pub to_addrs: Vec<String>,
    pub cc_addrs: Vec<String>,
    pub subject: Option<String>,
    pub body_text: Option<String>,
    pub body_html: Option<String>,
    pub attachments: Vec<AttachmentMeta>,
}

#[derive(Serialize)]
pub struct AttachmentMeta {
    pub id: String,
    pub filename: Option<String>,
    pub content_type: Option<String>,
    pub size: i64,
}

#[derive(Serialize)]
pub struct MessageSummary {
    pub id: String,
    pub received_at: String,
    pub from_addr: String,
    pub to_addrs: Vec<String>,
    pub subject: Option<String>,
}

pub struct AttachmentDownload {
    pub filename: Option<String>,
    pub content_type: Option<String>,
    pub data: Vec<u8>,
}
