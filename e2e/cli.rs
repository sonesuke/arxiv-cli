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
        .stdout(predicate::str::starts_with("{"));
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

// ============ API integration tests ============
// arXiv API rate limit: 1 request per 3 seconds, single connection.
// Run E2E tests with: cargo test --test e2e_cli -- --test-threads=1

#[test]
#[ignore = "hits real arXiv API - run with: cargo test --test e2e_cli -- --ignored --test-threads=1"]
fn test_search_returns_json() {
    // Respect arXiv API rate limit (1 req / 3 sec)
    std::thread::sleep(std::time::Duration::from_secs(3));

    cargo_bin_cmd!("arxiv-cli")
        .args(["search", "-q", "quantum computing", "-l", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("["))
        .stdout(predicate::str::contains("title"));
}

#[test]
#[ignore = "hits real arXiv API - run with: cargo test --test e2e_cli -- --ignored --test-threads=1"]
fn test_fetch_returns_json() {
    // Respect arXiv API rate limit (1 req / 3 sec)
    std::thread::sleep(std::time::Duration::from_secs(3));

    cargo_bin_cmd!("arxiv-cli")
        .args(["fetch", "2301.07041"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"id\""))
        .stdout(predicate::str::contains("\"title\""));
}
