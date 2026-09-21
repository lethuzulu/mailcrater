use mail_parser::MessageParser;
use mailin::{Action, Handler, Response, response::OK};
use tokio::sync::mpsc::Sender;
use std::io::Result;
use tracing::{error, info};

use crate::message::NewMessage;

#[derive(Clone)]
pub struct MailHandler {
    buffer: Vec<u8>,
    message_parser: MessageParser,
    sender: Sender<NewMessage>,
    max_message_size: usize,
    oversized: bool,
}

impl MailHandler {
    pub fn new(sender: Sender<NewMessage>, max_message_size: usize) -> Self {
        Self {
            buffer: Vec::new(),
            message_parser: MessageParser::new(),
            sender,
            max_message_size,
            oversized: false,
        }
    }
}

impl Handler for MailHandler {
    fn data(&mut self, _buf: &[u8]) -> Result<()> {
        // Once oversized, stop growing the buffer — the client still sends
        // the rest of the message (SMTP has no way to tell it to stop midway)
        if self.buffer.len() + _buf.len() > self.max_message_size {
            self.oversized = true;
        } else {
            self.buffer.extend_from_slice(_buf);
        }
        Ok(())
    }

    fn data_end(&mut self) -> Response {
        if self.oversized {
            self.buffer.clear();
            self.oversized = false;
            let mut response = Response::custom(552, "5.3.4 Message too large".to_string());
            response.action = Action::Close;
            return response;
        }

        // parse the self.buffer
        let message = self.message_parser.parse(&self.buffer);
        match message {
            Some(message) => {
                info!(?message, "received message");

                let raw_size = self.buffer.len();
                let mut new_message = NewMessage::from(&message);

                new_message.raw_size = raw_size;
                new_message.raw_source = std::mem::take(&mut self.buffer); // use mem::take to take ownership of the data buffer and clear it in one call

                // TODO: messages will silently drop once capacity of the channel is reached
                match self.sender.try_send(new_message) {
                    Ok(_) => {}
                    Err(e) => {
                        error!(?e, "storage channel full or closed, dropping message");
                    }
                }
                Response::custom(250, "2.0.0 Ok: message accepted".to_string())
            }
            None => {
                error!("failed to parse message: invalid or malformed data");
                self.buffer.clear();
                let mut response = Response::custom(500, "Error parsing message".to_string());
                response.action = Action::Close;
                response
            }
        }
    }
}
