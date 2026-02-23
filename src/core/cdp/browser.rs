use crate::core::{ArxivError, Result};
use serde_json::Value;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex as StdMutex};
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tokio::time::sleep;

/// Chrome browser process manager
pub struct CdpBrowser {
    process: Option<Child>,
    port: u16,
    #[allow(dead_code)]
    ws_url: String,
}

impl CdpBrowser {
    pub async fn launch(
        executable_path: Option<PathBuf>,
        args: Vec<&str>,
        headless: bool,
        debug: bool,
    ) -> Result<Self> {
        let chrome_path = executable_path.unwrap_or_else(|| {
            #[cfg(target_os = "windows")]
            {
                PathBuf::from("C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe")
            }
            #[cfg(target_os = "macos")]
            {
                PathBuf::from("/Applications/Google Chrome.app/Contents/MacOS/Google Chrome")
            }
            #[cfg(target_os = "linux")]
            {
                PathBuf::from("/usr/bin/google-chrome")
            }
            #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
            {
                PathBuf::from("chrome")
            }
        });

        // Create a temporary user data directory with a unique ID
        let unique_id = uuid::Uuid::new_v4();
        let temp_dir = std::env::temp_dir().join(format!("chrome-{}", unique_id));
        std::fs::create_dir_all(&temp_dir).map_err(ArxivError::Io)?;

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
        let stderr_handle = std::fs::File::create(&stderr_file).map_err(ArxivError::Io)?;

        cmd.stdout(Stdio::null());
        cmd.stderr(Stdio::from(stderr_handle));

        let mut process = cmd.spawn().map_err(ArxivError::Io)?;

        // Read the port from the stderr file
        let port = Arc::new(StdMutex::new(None::<u16>));
        let port_clone = port.clone();
        let stderr_path = stderr_file.clone();
        let debug_flag = debug;

        // Spawn a thread to read stderr and look for the port
        tokio::spawn(async move {
            let start = std::time::Instant::now();
            // Try for up to 30 seconds (CI environments may be slower)
            while start.elapsed().as_secs() < 30 {
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
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        });

        // Wait for the port to be discovered (up to 30 seconds for slower CI environments)
        let discovered_port = tokio::task::spawn_blocking(move || {
            for _ in 0..300 {
                let port_val = port.lock().map_or(None, |guard| *guard);

                if let Some(p) = port_val {
                    return Ok(p);
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(ArxivError::Cdp("Failed to discover Chrome debugging port".to_string()))
        })
        .await
        .map_err(|e| ArxivError::Cdp(format!("Join error: {}", e)))??;

        // Wait for Chrome to start and expose the debugging port
        // Retry get_ws_url with backoff instead of fixed sleep
        let ws_url_result =
            Self::get_ws_url_with_retry(discovered_port, 10, Duration::from_millis(500)).await;

        // Enhanced error handling with helpful messages
        let ws_url = match ws_url_result {
            Ok(url) => url,
            Err(e) => {
                // Check if Chrome process is still running
                let status = process.try_wait();
                match status {
                    Ok(Some(exit_status)) => {
                        return Err(ArxivError::Cdp(format!(
                            "Chrome process exited early with status: {}",
                            exit_status
                        )));
                    }
                    Ok(None) => {
                        // Process is still running but we couldn't connect
                        return Err(ArxivError::Cdp(format!(
                            "Chrome process is still running but debugging port was not found after 30 seconds.\n\n\
                             Troubleshooting:\n\
                             - If running in CI, ensure Chrome/Chromium is installed and accessible\n\
                             - Check if Chrome requires additional flags (e.g., --no-sandbox for Linux CI)\n\
                             - Verify the temp directory is writable\n\
                             - Try using 'config --set-browser' to specify the correct Chrome path\n\n\
                             Original error: {}",
                            e
                        )));
                    }
                    Err(err) => {
                        return Err(ArxivError::Cdp(format!(
                            "Failed to check Chrome process status: {}\n\nOriginal error: {}",
                            err, e
                        )));
                    }
                }
            }
        };

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

        Err(last_error.unwrap_or_else(|| {
            ArxivError::Cdp("Failed to get WebSocket URL after retries".to_string())
        }))
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
            .ok_or_else(|| ArxivError::Cdp("Could not find webSocketDebuggerUrl".to_string()))
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

        response["webSocketDebuggerUrl"].as_str().map(String::from).ok_or_else(|| {
            ArxivError::Cdp("Could not find webSocketDebuggerUrl for new page".to_string())
        })
    }
}

impl Drop for CdpBrowser {
    fn drop(&mut self) {
        if let Some(mut process) = self.process.take() {
            let _ = process.kill();
        }
    }
}

pub struct BrowserState {
    pub browser: Option<Arc<CdpBrowser>>,
    pub last_used: Instant,
}

#[derive(Clone)]
pub struct BrowserManager {
    pub config: crate::core::Config,
    state: Arc<Mutex<BrowserState>>,
}

impl BrowserManager {
    pub fn new(config: crate::core::Config) -> Self {
        let state = Arc::new(Mutex::new(BrowserState { browser: None, last_used: Instant::now() }));

        // Spawn the inactivity monitor task
        let state_clone = state.clone();
        tokio::spawn(async move {
            loop {
                sleep(Duration::from_secs(60)).await;
                let mut s = state_clone.lock().await;
                if s.browser.is_some() && s.last_used.elapsed() > Duration::from_secs(5 * 60) {
                    s.browser = None; // Drops Arc<CdpBrowser>, which triggers process kill
                }
            }
        });

        Self { config, state }
    }

    pub async fn get_browser(&self) -> Result<Arc<CdpBrowser>> {
        let mut s = self.state.lock().await;
        s.last_used = Instant::now();

        if let Some(browser) = &s.browser {
            return Ok(Arc::clone(browser));
        }

        // Build Chrome args from config
        let mut args = vec![
            "--user-agent=Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
        ];

        // Add custom Chrome args from config
        args.extend(self.config.chrome_args.iter().map(|s| s.as_str()));

        // In CI environments, automatically add sandbox-disabling flags
        if std::env::var("CI").is_ok() {
            args.push("--disable-gpu");
            args.push("--no-sandbox");
            args.push("--disable-setuid-sandbox");
        }

        let browser_path = self.config.browser_path.as_ref().map(PathBuf::from);

        let browser =
            Arc::new(CdpBrowser::launch(browser_path, args, self.config.headless, false).await?);
        s.browser = Some(Arc::clone(&browser));

        Ok(browser)
    }
}
