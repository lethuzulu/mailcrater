pub(super) struct MessageRow {
    pub(super) id: String,
    pub(super) received_at: String,
    pub(super) from_addr: String,
    pub(super) to_addrs: String,
    pub(super) cc_addrs: Option<String>,
    pub(super) subject: Option<String>,
    pub(super) body_text: Option<String>,
    pub(super) body_html: Option<String>,
}

pub(super) struct AttachmentRow {
    pub(super) id: String,
    pub(super) filename: Option<String>,
    pub(super) content_type: Option<String>,
    pub(super) size: i64,
}

pub(super) struct MessageSummaryRow {
    pub(super) id: String,
    pub(super) received_at: String,
    pub(super) from_addr: String,
    pub(super) to_addrs: String,
    pub(super) subject: Option<String>,
}

pub(super) struct AttachmentDownloadRow {
    pub(super) filename: Option<String>,
    pub(super) content_type: Option<String>,
    pub(super) data: Vec<u8>,
}
