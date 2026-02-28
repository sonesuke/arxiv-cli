mod arxiv_search;
mod config;
mod error;
mod models;

pub use arxiv_search::ArxivClient;
pub use config::Config;
pub use error::{ArxivError, Result};

// Re-export chrome-cdp types for public API
#[allow(unused_imports)]
pub use chrome_cdp::{BrowserManager, CdpBrowser, CdpPage};
