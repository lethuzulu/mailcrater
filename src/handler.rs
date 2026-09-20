use mail_parser::MessageParser;
use mailin::{Handler, Response, response::OK};
use tokio::sync::mpsc::Sender;
use std::io::Result;
use tracing::{error, info};

use crate::message::NewMessage;

#[derive(Clone)]
pub struct MailHandler {
    buffer: Vec<u8>,
    message_parser: MessageParser,
    sender: Sender<NewMessage>
}

impl MailHandler {
    pub fn new(sender: Sender<NewMessage>) -> Self {
        Self {
            buffer: Vec::new(),
            message_parser: MessageParser::new(),
            sender
        }
    }
}

impl Handler for MailHandler {
    fn data(&mut self, _buf: &[u8]) -> Result<()> {
        self.buffer.extend_from_slice(_buf);
        Ok(())
    }

    fn data_end(&mut self) -> Response {
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
                Response::custom(500, "Error parsing message".to_string())
            }
        }
    }
}
