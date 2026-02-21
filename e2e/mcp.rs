use serde_json::json;
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

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

    // Cleanup
    let _ = child.kill();
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
    writeln!(stdin, "{}", init_request.to_string()).unwrap();
    let mut response = String::new();
    reader.read_line(&mut response).unwrap();
    assert!(response.contains("tools"), "Init response failed to contain tools: {}", response);

    // 2. Initialized notification
    let initialized_notification = json!({
        "jsonrpc": "2.0",
        "method": "initialized"
    });
    writeln!(stdin, "{}", initialized_notification.to_string()).unwrap();

    // 3. List tools
    let list_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list",
        "params": {}
    });
    writeln!(stdin, "{}", list_request.to_string()).unwrap();
    response.clear();
    reader.read_line(&mut response).unwrap();

    assert!(response.contains("search_papers"), "List tools response failed: {}", response);
    assert!(response.contains("fetch_paper"), "List tools response failed: {}", response);

    let _ = child.kill();
}
