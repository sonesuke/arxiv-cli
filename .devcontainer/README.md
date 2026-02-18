# Dev Container

This folder contains the development container configuration for the Arxiv-CLI project.

## CLI Usage

When using the `devcontainer` CLI, version **0.80.2** or later is required.

```bash
npx -y @devcontainers/cli@0.80.2 up --workspace-folder . --remove-existing-container
```

**Important:** Earlier versions (including 0.80.0) have a bug that causes the CLI to hang after "Container started".

## Files

- `devcontainer.json` - Container configuration
- `Dockerfile` - Base image with Rust, Node.js, and development tools
- `post-create.sh` - Setup script that runs in background after container creation

## Setup Process

The `post-create.sh` script runs automatically after the container starts. It:

1. Fixes permissions for `CARGO_HOME`
2. Installs Claude CLI
3. Configures `tmux`
4. Runs `cargo check` to verify the project
5. Installs the `arxiv-cli` binary
6. Configures the `claude` alias
7. Authenticates with Z.ai (if `Z_AI_API_KEY` is set)

The `devcontainer up` command will wait for the setup to complete before exiting.

## CI Environment

In CI (when `CI` or `GITHUB_ACTIONS` is set), the setup script skips all development setup steps.
