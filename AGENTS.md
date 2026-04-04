# AGENTS.md

This file contains instructions for AI coding agents working on this project.

## Project Overview

arXiv CLI — A Rust-based command-line tool for searching and fetching papers from arXiv.

## Rules

### Git

- Use **conventional commits** (e.g., `feat:`, `fix:`, `refactor:`, `chore:`). Commit messages are in **English**.
- **NEVER** use `git commit --no-verify`. The pre-commit hook exists to enforce quality. If it fails, fix the issue.
- Do not force-push to `main`.

### Code Quality

- Run `mise run pre-commit` before committing. This runs `cargo fmt --check`, `cargo clippy -D warnings`, and `cargo test`.
- For code coverage, run `mise run coverage`. This is specially configured to measure coverage of subprocesses (like MCP server).
- Follow existing patterns in the codebase.
- Make small, focused changes.

### Language

- Code comments, commit messages, and **Pull Requests**: **English**
- Responses to the user: **日本語**

## Project Structure

```
src/                    # Rust source code
agents/pr-healer/       # PR-Healer autonomous agent
  healer.sh             # Host-side daemon loop
  prompt.txt            # Agent instructions
  tools/                # Agent tools
    load-progress.sh    # Read past context (JSONL)
    record-progress.sh  # Write progress logs (JSONL)
mise.toml               # Task definitions (fmt, clippy, test, pre-commit)
.devcontainer/          # Dev container configuration
```

## Tools

| Command | Description |
|---|---|
| `mise run fmt` | Check formatting with `cargo fmt` |
| `mise run clippy` | Lint with `cargo clippy` |
| `mise run test` | Run tests with `cargo test` |
| `mise run pre-commit` | Run all of the above |
| `mise run coverage` | Measure code coverage (including subprocesses) |
| `mise run skill-test` | Run all skill-bench tests |

## Skill-Bench Testing Framework

Test cases are in `tests/`.

Requires [skill-bench](https://github.com/sonesuke/skill-bench) (set up via post-create script).

```toml
name = "test-name"
description = "Test description"
timeout = 120

test_prompt = """
English prompt that should trigger the skill
"""

[[checks]]
name = "check-name"
command = { command = "mcp-success", tool = "tool_name" }

[[checks]]
name = "param-check"
command = { command = "tool-param", tool = "tool_name", param = "limit", value = "20" }
```

Available check types: `mcp-success`, `mcp-tool-invoked`, `mcp-loaded`, `tool-use`, `tool-param`, `skill-invoked`, `skill-loaded`, `workspace-file`, `workspace-dir`, `file-contains`, `log-contains`, `message-contains`, `db-query`.

**Note:** Test prompts must be in English to ensure consistent skill triggering.
