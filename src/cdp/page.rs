use anyhow::{Result, anyhow};
use serde_json::{Value, json};
use std::time::Duration;
use tokio::time::sleep;

use super::connection::CdpConnection;

/// CDP Page for browser automation
pub struct CdpPage {
    connection: CdpConnection,
}

impl CdpPage {
    /// Create a new page with the given connection
    pub async fn new(ws_url: &str) -> Result<Self> {
        let connection = CdpConnection::connect(ws_url).await?;

        // Enable necessary domains
        connection.send_command("Page.enable", json!({})).await?;
        connection.send_command("Runtime.enable", json!({})).await?;

        Ok(Self { connection })
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
            return Err(anyhow!("JavaScript error: {:?}", exception));
        }

        Ok(result["result"]["value"].clone())
    }
}
