#[derive(Debug)]
pub struct Config {
    pub smtp_port: u16,
    pub http_port: u16,
    pub data_dir: String,
}

impl Config {
    pub fn from_env() -> Self {
        // TODO: read from env
        Self {
            smtp_port: 1025,
            http_port: 1080,
            data_dir: "./data".to_string(),
        }
    }
}
