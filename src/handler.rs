use mail_parser::MessageParser;
use mailin::{Handler, Response};
use std::io::Result;
use tracing::{error, info};

#[derive(Clone)]
pub struct MailHandler {
    buffer: Vec<u8>,
    message_parser: MessageParser,
}

impl MailHandler {
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            message_parser: MessageParser::new(),
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
                self.buffer.clear();
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
