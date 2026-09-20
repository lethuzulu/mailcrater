use mail_parser::{Message, MessagePart, MimeHeaders};

#[derive(Debug)]
pub struct NewMessage {
    pub from_addr: String,
    pub to_addrs: Vec<String>,
    pub cc_addrs: Vec<String>,
    pub subject: Option<String>,
    pub body_text: Option<String>,
    pub body_html: Option<String>,
    pub raw_size: usize,
    pub raw_source: Vec<u8>,
    pub attachments: Vec<NewAttachment>,
}

#[derive(Debug)]
pub struct NewAttachment {
    pub filename: Option<String>,
    pub content_type: Option<String>,
    pub size: usize,
    pub data: Vec<u8>,
}

impl From<&Message<'_>> for NewMessage {
    fn from(message: &Message<'_>) -> Self {
        let from_addr = match message.from() {
            Some(address) => match address.first() {
                Some(addr) => match addr.address() {
                    Some(a) => a.to_string(),
                    // TODO: falls back to an empty string when the message has no parseable `From:` header.
                    None => String::new(),
                },
                None => String::new(),
            },
            None => String::new(),
        };

        let to_addrs = match message.to() {
            Some(address) => {
                let mut addrs = Vec::new();
                for addr in address.iter() {
                    match addr.address() {
                        Some(a) => addrs.push(a.to_string()),
                        None => {}
                    }
                }
                addrs
            }
            None => Vec::new(),
        };

        let cc_addrs = match message.cc() {
            Some(address) => {
                let mut addrs = Vec::new();
                for addr in address.iter() {
                    match addr.address() {
                        Some(a) => addrs.push(a.to_string()),
                        None => {}
                    }
                }
                addrs
            }
            None => Vec::new(),
        };

        let subject = match message.subject() {
            Some(s) => Some(s.to_string()),
            None => None,
        };

        let body_text = match message.body_text(0) {
            Some(c) => Some(c.into_owned()),
            None => None,
        };

        let body_html = match message.body_html(0) {
            Some(c) => Some(c.into_owned()),
            None => None,
        };

        let mut attachments = Vec::new();
        for part in message.attachments() {
            attachments.push(NewAttachment::from(part));
        }

        NewMessage {
            from_addr,
            to_addrs,
            cc_addrs,
            subject,
            body_text,
            body_html,
            raw_size: 0,
            raw_source: Vec::new(),
            attachments,
        }
    }
}

impl From<&MessagePart<'_>> for NewAttachment {
    fn from(part: &MessagePart<'_>) -> Self {
        let filename = match part.attachment_name() {
            Some(name) => Some(name.to_string()),
            None => None,
        };

        let content_type = match part.content_type() {
            Some(ct) => {
                let subtype = match &ct.c_subtype {
                    Some(s) => s.as_ref(),
                    None => "",
                };
                Some(format!("{}/{}", ct.c_type, subtype))
            }
            None => None,
        };

        NewAttachment {
            filename,
            content_type,
            size: part.len(),
            data: part.contents().to_vec(),
        }
    }
}
