use anyhow::{Result, anyhow};
use serde_json::Value;
use std::io::BufReader;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::sleep;

/// Chrome browser process manager
pub struct CdpBrowser {
    process: Option<Child>,
    port: u16,
    #[allow(dead_code)]
    ws_url: String,
}

impl CdpBrowser {
    /// Launch Chrome/Chromium and get WebSocket debugger URL
    pub async fn launch(
        executable_path: Option<PathBuf>,
        args: Vec<&str>,
        headless: bool,
        debug: bool,
    ) -> Result<Self> {
        let chrome_path = executable_path.unwrap_or_else(|| {
            PathBuf::from("/Applications/Google Chrome.app/Contents/MacOS/Google Chrome")
        });

        // Create a temporary user data directory with a unique ID
        let unique_id = uuid::Uuid::new_v4();
        let temp_dir = std::env::temp_dir().join(format!("chrome-{}", unique_id));
        std::fs::create_dir_all(&temp_dir)?;

        let mut cmd = Command::new(&chrome_path);
        cmd.arg("--remote-debugging-port=0"); // Let OS assign a random port
        cmd.arg(format!("--user-data-dir={}", temp_dir.display()));

        if headless {
            cmd.arg("--headless");
        }

        for arg in args {
            cmd.arg(arg);
        }

        // Always capture stderr to read the assigned port
        // Use a temporary file for stderr to avoid buffering issues with pipes
        let stderr_file = temp_dir.join("chrome_stderr.log");
        let stderr_handle = std::fs::File::create(&stderr_file)?;

        cmd.stdout(Stdio::null());
        cmd.stderr(Stdio::from(stderr_handle));

        let process = cmd.spawn()?;

        // Read the port from the stderr file
        let port = Arc::new(Mutex::new(None::<u16>));
        let port_clone = port.clone();
        let stderr_path = stderr_file.clone();
        let debug_flag = debug;

        // Spawn a thread to read the file and extract the port
        std::thread::spawn(move || {
            let file = match std::fs::File::open(&stderr_path) {
                Ok(f) => f,
                Err(_) => return,
            };
            let _reader = BufReader::new(file);

            // Poll the file for the port message
            for _ in 0..100 {
                // Try for 10 seconds
                // We need to re-open or seek to read new content, but simple polling works for now
                // Actually, let's just read the whole file each time since it's small
                if let Ok(content) = std::fs::read_to_string(&stderr_path) {
                    for line in content.lines() {
                        if debug_flag && line.contains("DevTools listening on") {
                            eprintln!("Chrome: {}", line);
                        }

                        if line.contains("DevTools listening on") {
                            #[allow(clippy::collapsible_if)]
                            if let Some(port_str) = line.split("127.0.0.1:").nth(1) {
                                if let Some(port_num) = port_str.split('/').next() {
                                    if let Ok(p) = port_num.parse::<u16>() {
                                        if let Ok(mut guard) = port_clone.lock() {
                                            *guard = Some(p);
                                        }
                                        return;
                                    }
                                }
                            }
                        }
                    }
                }
                std::thread::sleep(Duration::from_millis(100));
            }
        });

        // Wait for the port to be discovered (up to 10 seconds)
        let discovered_port = tokio::task::spawn_blocking(move || {
            for _ in 0..100 {
                let port_val = port.lock().map_or(None, |guard| *guard);

                if let Some(p) = port_val {
                    return Ok(p);
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(anyhow!("Failed to discover Chrome debugging port"))
        })
        .await??;

        // Wait for Chrome to start and expose the debugging port
        // Retry get_ws_url with backoff instead of fixed sleep
        let ws_url =
            Self::get_ws_url_with_retry(discovered_port, 10, Duration::from_millis(500)).await?;

        Ok(Self { process: Some(process), port: discovered_port, ws_url })
    }

    /// Get WebSocket debugger URL from Chrome with retry logic
    async fn get_ws_url_with_retry(
        port: u16,
        max_retries: u32,
        retry_delay: Duration,
    ) -> Result<String> {
        let mut last_error = None;

        for attempt in 0..max_retries {
            match Self::get_ws_url(port).await {
                Ok(url) => return Ok(url),
                Err(e) => {
                    last_error = Some(e);
                    if attempt < max_retries - 1 {
                        sleep(retry_delay).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow!("Failed to get WebSocket URL after retries")))
    }

    /// Get WebSocket debugger URL from Chrome
    async fn get_ws_url(port: u16) -> Result<String> {
        let client = reqwest::Client::new();
        let response: Value = client
            .get(format!("http://127.0.0.1:{}/json/version", port))
            .send()
            .await?
            .json()
            .await?;

        response["webSocketDebuggerUrl"]
            .as_str()
            .map(String::from)
            .ok_or_else(|| anyhow!("Could not find webSocketDebuggerUrl"))
    }

    /// Create a new page and return its WebSocket URL
    pub async fn new_page(&self) -> Result<String> {
        let client = reqwest::Client::new();
        let response: Value = client
            .put(format!("http://127.0.0.1:{}/json/new", self.port))
            .send()
            .await?
            .json()
            .await?;

        response["webSocketDebuggerUrl"]
            .as_str()
            .map(String::from)
            .ok_or_else(|| anyhow!("Could not find webSocketDebuggerUrl for new page"))
    }
}

impl Drop for CdpBrowser {
    fn drop(&mut self) {
        if let Some(mut process) = self.process.take() {
            let _ = process.kill();
        }
    }
}
