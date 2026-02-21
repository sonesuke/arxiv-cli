use serde_json::json;
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::time::Duration;

/// Gracefully shut down the MCP child process by closing stdin,
/// then waiting for it to exit. This ensures coverage profraw files
/// are flushed (SIGKILL would prevent that).
fn graceful_shutdown(stdin: std::process::ChildStdin, mut child: std::process::Child) {
    drop(stdin); // Close stdin → server's reader loop ends → process exits

    // Poll try_wait for up to 5 seconds
    for _ in 0..50 {
        match child.try_wait() {
            Ok(Some(_)) => return, // exited cleanly
            Ok(None) => std::thread::sleep(Duration::from_millis(100)),
            Err(_) => break,
        }
    }
    // Fallback: force kill if still running
    let _ = child.kill();
    let _ = child.wait();
}

#[test]
fn test_mcp_initialize() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_arxiv-cli"))
        .arg("mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to spawn arxiv-cli mcp");

    let mut stdin = child.stdin.take().expect("Failed to open stdin");
    let stdout = child.stdout.take().expect("Failed to open stdout");
    let mut reader = BufReader::new(stdout);

    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "implementation": {
                "name": "test-client",
                "version": "1.0.0"
            },
            "capabilities": {}
        }
    });

    let request_str = init_request.to_string();
    writeln!(stdin, "{}", request_str).expect("Failed to write to stdin");

    let mut response = String::new();
    reader.read_line(&mut response).expect("Failed to read from stdout");

    // The SDK returns ServerCapabilities directly as the result
    assert!(response.contains("tools"), "Response did not contain tools: {}", response);

    // Graceful cleanup to flush coverage data
    graceful_shutdown(stdin, child);
}

#[test]
fn test_mcp_list_tools() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_arxiv-cli"))
        .arg("mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("Failed to spawn arxiv-cli mcp");

    let mut stdin = child.stdin.take().expect("Failed to open stdin");
    let stdout = child.stdout.take().expect("Failed to open stdout");
    let mut reader = BufReader::new(stdout);

    // 1. Initialize
    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "implementation": { "name": "test", "version": "1.0" },
            "capabilities": {}
        }
    });
    writeln!(stdin, "{}", init_request).unwrap();
    let mut response = String::new();
    reader.read_line(&mut response).unwrap();
    assert!(response.contains("tools"), "Init response failed to contain tools: {}", response);

    // 2. Initialized notification
    let initialized_notification = json!({
        "jsonrpc": "2.0",
        "method": "initialized"
    });
    writeln!(stdin, "{}", initialized_notification).unwrap();

    // 3. List tools
    let list_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list",
        "params": {}
    });
    writeln!(stdin, "{}", list_request).unwrap();
    response.clear();
    reader.read_line(&mut response).unwrap();

    assert!(response.contains("search_papers"), "List tools response failed: {}", response);
    assert!(response.contains("fetch_paper"), "List tools response failed: {}", response);

    // Graceful cleanup to flush coverage data
    graceful_shutdown(stdin, child);
}
