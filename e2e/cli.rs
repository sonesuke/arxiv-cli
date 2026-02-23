use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;

// ============ Help / Usage tests ============

#[test]
fn test_help() {
    cargo_bin_cmd!("arxiv-cli")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Search and fetch papers from Arxiv"));
}

#[test]
fn test_search_help() {
    cargo_bin_cmd!("arxiv-cli")
        .args(["search", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Search query"));
}

#[test]
fn test_fetch_help() {
    cargo_bin_cmd!("arxiv-cli")
        .args(["fetch", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Arxiv ID"));
}

// ============ Config subcommand tests ============

#[test]
fn test_config_path() {
    cargo_bin_cmd!("arxiv-cli")
        .args(["config", "path"])
        .assert()
        .success()
        .stdout(predicate::str::contains("config.json"));
}

#[test]
fn test_config_list() {
    cargo_bin_cmd!("arxiv-cli")
        .args(["config", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("headless"));
}

#[test]
fn test_config_set_get_headless() {
    let temp_dir = tempfile::tempdir().unwrap();
    // Isolate config directory
    cargo_bin_cmd!("arxiv-cli")
        .env("HOME", temp_dir.path())
        .env("XDG_CONFIG_HOME", temp_dir.path())
        .env("APPDATA", temp_dir.path())
        .env("USERPROFILE", temp_dir.path())
        .args(["config", "set", "headless", "false"])
        .assert()
        .success();

    cargo_bin_cmd!("arxiv-cli")
        .env("HOME", temp_dir.path())
        .env("XDG_CONFIG_HOME", temp_dir.path())
        .env("APPDATA", temp_dir.path())
        .env("USERPROFILE", temp_dir.path())
        .args(["config", "get", "headless"])
        .assert()
        .success()
        .stdout(predicate::str::contains("false"));

    // Reset to default
    cargo_bin_cmd!("arxiv-cli")
        .env("HOME", temp_dir.path())
        .env("XDG_CONFIG_HOME", temp_dir.path())
        .env("APPDATA", temp_dir.path())
        .env("USERPROFILE", temp_dir.path())
        .args(["config", "set", "headless", "true"])
        .assert()
        .success();
}

#[test]
fn test_config_set_unknown_key() {
    cargo_bin_cmd!("arxiv-cli")
        .args(["config", "set", "unknown_key", "value"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Unknown config key"));
}

#[test]
fn test_config_get_unknown_key() {
    cargo_bin_cmd!("arxiv-cli")
        .args(["config", "get", "unknown_key"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Unknown config key"));
}

// ============ Invalid usage tests ============

#[test]
fn test_no_subcommand() {
    cargo_bin_cmd!("arxiv-cli").assert().failure().stderr(predicate::str::contains("Usage"));
}

#[test]
fn test_search_without_query() {
    cargo_bin_cmd!("arxiv-cli")
        .arg("search")
        .assert()
        .failure()
        .stderr(predicate::str::contains("--query"));
}

// ============ CDP specific tests ============

#[test]
fn test_head_flag_exists() {
    // Just verify the flag is accepted by help
    cargo_bin_cmd!("arxiv-cli")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--head"));
}

// ============ Execution tests (Real network/browser) ============
// Note: These tests depend on a working network and Chrome/Chromium installation.
// We use small limits and specific IDs to keep them fast.

#[test]
fn test_search_execution() {
    // Use verbose mode for better debugging in CI
    let result = cargo_bin_cmd!("arxiv-cli")
        .args(["--verbose", "search", "--query", "LLM", "--limit", "1"])
        .assert();

    eprintln!("STDOUT: {}", String::from_utf8_lossy(&result.get_output().stdout));
    eprintln!("STDERR: {}", String::from_utf8_lossy(&result.get_output().stderr));
    eprintln!("EXIT CODE: {:?}", result.get_output().status.code());

    result
        .success()
        .stdout(predicate::str::contains("\"title\""))
        .stdout(predicate::str::contains("\"authors\""))
        .stdout(predicate::str::contains("\"url\""));
}

#[test]
fn test_fetch_execution() {
    // Use verbose mode for better debugging in CI
    let result = cargo_bin_cmd!("arxiv-cli").args(["--verbose", "fetch", "2301.00001"]).assert();

    eprintln!("STDOUT: {}", String::from_utf8_lossy(&result.get_output().stdout));
    eprintln!("STDERR: {}", String::from_utf8_lossy(&result.get_output().stderr));
    eprintln!("EXIT CODE: {:?}", result.get_output().status.code());

    result
        .success()
        .stdout(predicate::str::contains("\"title\""))
        .stdout(predicate::str::contains("\"2301.00001\""));
}
