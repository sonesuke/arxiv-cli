use thiserror::Error;

#[derive(Error, Debug)]
pub enum ArxivError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("CDP error: {0}")]
    Cdp(String),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Extraction error: {0}")]
    Extraction(String),

    #[error("Other error: {0}")]
    Other(String),
}

// Implement conversion from chrome_cdp::Error
impl From<chrome_cdp::Error> for ArxivError {
    fn from(err: chrome_cdp::Error) -> Self {
        match err {
            chrome_cdp::Error::Browser(msg) => ArxivError::Cdp(format!("Browser: {}", msg)),
            chrome_cdp::Error::Cdp(msg) => ArxivError::Cdp(format!("Protocol: {}", msg)),
            chrome_cdp::Error::Io(err) => ArxivError::Io(err),
            chrome_cdp::Error::Http(msg) => ArxivError::Cdp(format!("HTTP: {}", msg)),
            chrome_cdp::Error::Json(err) => ArxivError::Serialization(err),
            chrome_cdp::Error::WebSocket(msg) => ArxivError::Cdp(format!("WebSocket: {}", msg)),
        }
    }
}

pub type Result<T> = std::result::Result<T, ArxivError>;
