# arXiv CLI - AI-ready 🚀

An AI-ready search and fetch tool for arXiv papers, designed for both humans and AI agents.

## Features
- **Search papers** by free-text query.
- **Fetch paper details** by arXiv ID.
- **Formatted JSON output** including `description_paragraphs` (extracted from PDF).
- **Pagination support** via `--limit` option.
- **Date filtering** with `--before` and `--after`.
- **Raw PDF download** with `--raw` flag.
- **Headless mode** by default; use `--head` to show the browser.
- **Model Context Protocol (MCP)** support to integrate with AI agents.
- **Robust formatting**: Uses structured JSON for easy machine consumption.

## Installation

### Easy Install (Recommended)

**Linux & macOS:**
```bash
curl -fsSL https://raw.githubusercontent.com/sonesuke/arxiv-cli/main/install.sh | bash
```
> Note: On Linux, this installs to `~/.local/bin` without requiring `sudo`. Make sure `~/.local/bin` is in your `PATH`.

**Windows (PowerShell):**
```powershell
irm https://raw.githubusercontent.com/sonesuke/arxiv-cli/main/install.ps1 | iex
```

### From Source (Cargo)
If you have Rust installed, you can build from source:
```bash
cargo install --path .
```

## Model Context Protocol (MCP)

`arxiv-cli` supports the [Model Context Protocol](https://modelcontextprotocol.io/), allowing AI agents (like Claude Desktop) to search and fetch papers directly.

### Available Tools

| Tool Name | Description | Parameters |
|---|---|---|
| `search_papers` | Search arXiv for papers matching a free-text query. | `query` (required), `limit`, `before`, `after` |
| `fetch_paper` | Fetch details (metadata & PDF text) of a specific paper. | `paper_id` (required, e.g., "2512.04518") |

### Usage
To start the MCP server over `stdio`:
```bash
arxiv-cli mcp
```

### Configuration for Claude Desktop

Add this to your `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "arxiv-cli": {
      "command": "/path/to/arxiv-cli",
      "args": ["mcp"]
    }
  }
}
```

## Usage

### CLI Commands

| Command | Description | Example |
|---|---|---|
| `search` | Search for papers matching a query. | `arxiv-cli search --query "LLM" --limit 10` |
| `fetch` | Fetch a single paper's metadata and text. | `arxiv-cli fetch 2512.04518` |
| `config` | Manage configuration settings. | `arxiv-cli config list` |
| `mcp` | Start the MCP server over stdio. | `arxiv-cli mcp` |


### Search by query
Search for papers matching a query.
```bash
arxiv-cli search --query "LLM" --limit 10
```

### Filter by date
```bash
# Papers submitted after 2024-01-01
arxiv-cli search --query "machine learning" --after "2024-01-01"

# Papers submitted between 2023-01-01 and 2023-12-31
arxiv-cli search --query "blockchain" --after "2023-01-01" --before "2023-12-31"
```

### Fetch paper details
Fetch a single paper's metadata and extracted text.
```bash
arxiv-cli fetch 2512.04518
```

### Fetch raw PDF
Download the PDF file directly to stdout.
```bash
arxiv-cli fetch 2512.04518 --raw > paper.pdf
```

### Show the browser window
Useful for debugging.
```bash
arxiv-cli search --query "AI" --head
```

## Configuration
This tool relies on a compatible Chrome/Chromium installation for scraping.
Config file location:
- macOS: `~/Library/Application Support/com.sonesuke.arxiv-cli/config.json`
- Linux: `~/.config/arxiv-cli/config.json`
- Windows: `C:\Users\{User}\AppData\Roaming\sonesuke\arxiv-cli\config\config.json`

### Manage Configuration
You can manage the configuration via CLI:

```bash
# List current configuration
arxiv-cli config list

# Set a value
arxiv-cli config set headless false
arxiv-cli config set browser_path "/usr/bin/google-chrome"

# Get a value
arxiv-cli config get headless

# Show config file path
arxiv-cli config path
```

## Implementation Details
- **Stack**: Rust, Clap, Headless Chrome, Serde, Reqwest, PDF-Extract, `mcp-sdk-rs`.
- **PDF Extraction**: Downloads the PDF and extracts text, splitting it into structured paragraphs (`description_paragraphs`).

## License
MIT
