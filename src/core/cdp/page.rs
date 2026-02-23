use crate::core::{ArxivError, Result};
use serde_json::{Value, json};
use std::time::Duration;
use tokio::time::sleep;

use super::connection::CdpConnection;

/// CDP Page for browser automation
pub struct CdpPage {
    connection: CdpConnection,
    verbose: bool,
}

impl CdpPage {
    /// Create a new page with the given connection
    pub async fn new(ws_url: &str) -> Result<Self> {
        Self::new_with_verbose(ws_url, false).await
    }

    /// Create a new page with verbose mode
    pub async fn new_with_verbose(ws_url: &str, verbose: bool) -> Result<Self> {
        let connection = CdpConnection::connect(ws_url).await?;

        // Enable necessary domains
        connection.send_command("Page.enable", json!({})).await?;
        connection.send_command("Runtime.enable", json!({})).await?;

        Ok(Self { connection, verbose })
    }

    /// Navigate to a URL
    pub async fn goto(&self, url: &str) -> Result<()> {
        self.connection.send_command("Page.navigate", json!({ "url": url })).await?;

        Ok(())
    }

    /// Wait for an element to appear on the page
    pub async fn wait_for_element(&self, selector: &str, timeout_secs: u64) -> Result<bool> {
        let start = std::time::Instant::now();

        while start.elapsed().as_secs() < timeout_secs {
            let script = format!("!!document.querySelector(\"{}\")", selector.replace('"', "\\\""));

            let result = self.evaluate(&script).await?;
            if result.as_bool().unwrap_or(false) {
                return Ok(true);
            }

            sleep(Duration::from_millis(500)).await;
        }

        Ok(false)
    }

    /// Evaluate JavaScript and return the result
    pub async fn evaluate(&self, script: &str) -> Result<Value> {
        let result = self
            .connection
            .send_command(
                "Runtime.evaluate",
                json!({
                    "expression": script,
                    "returnByValue": true,
                    "awaitPromise": true
                }),
            )
            .await?;

        if let Some(exception) = result.get("exceptionDetails") {
            return Err(ArxivError::Cdp(format!("JavaScript error: {:?}", exception)));
        }

        let value = result["result"]["value"].clone();

        if self.verbose {
            eprintln!(
                "[VERBOSE] JS evaluate result: {}",
                serde_json::to_string_pretty(&value).unwrap_or_else(|_| format!("{:?}", value))
            );
        }

        Ok(value)
    }

    /// Close the page/tab
    pub async fn close(&self) -> Result<()> {
        self.connection.send_command("Page.close", json!({})).await?;
        Ok(())
    }
}
