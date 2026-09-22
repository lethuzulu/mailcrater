use std::path::PathBuf;

#[derive(Debug)]
pub struct Config {
    pub smtp_port: u16,
    pub http_port: u16,
    pub data_dir: PathBuf,
    pub max_message_size: usize,
    pub channel_capacity: usize
}

impl Config {
    pub fn from_env() -> Self {
        // TODO: read from env
        Self {
            smtp_port: 1025,
            http_port: 1080,
            data_dir: PathBuf::from("./data"),
            // 25 MB, matching real-world provider limits (Gmail ~25MB)
            max_message_size: 25 * 1024 * 1024,
            channel_capacity: 1024
        }
    }
}
