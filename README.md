# arXiv CLI - AI-ready

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
- **Cypher query support**: Query search results with Cypher (graph query language).
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

| Tool Name | Description | Parameters | Response |
|---|---|---|---|
| `search_papers` | Search arXiv for papers matching a free-text query. | `query` (required), `limit`, `before`, `after` | `{ dataset, count, graph_schema }` |
| `fetch_paper` | Fetch details (metadata & PDF text) of a specific paper. | `id` (required), `raw` (optional) | `{ dataset, graph_schema }` |
| `execute_cypher` | Execute a Cypher query against a loaded dataset. | `dataset`, `query` (required) | Query results as JSON array |

### Usage
To start the MCP server over `stdio`:
```bash
arxiv-cli mcp
```

### Cypher Query Support

After calling `search_papers` or `fetch_paper`, the results are loaded into an in-memory graph engine. The dataset name is automatically generated from the query parameters (e.g., `search_a1b2c3d4`, `fetch_e5f6g7h8`). You can then query the results using **Cypher** (the graph query language from Neo4j).

#### Caching

Results are cached automatically (up to 100 recent queries). If you call `search_papers` or `fetch_paper` with the same parameters, the cached dataset is returned immediately without fetching from arXiv.

#### Workflow

```javascript
// 1. Search papers (dataset name is auto-generated, e.g., "search_a1b2c3d4")
search_papers({ query: "LLM", limit: 10 })
// Returns: { dataset: "search_a1b2c3d4", count: 10, graph_schema: "..." }

// 2. Query the dataset using Cypher
execute_cypher({ dataset: "search_a1b2c3d4", query: "MATCH (p) RETURN p.title, p.authors LIMIT 5" })
// Returns: [{ "p.title": "...", "p.authors": ["..."] }, ...]

// 3. Same query returns cached results (instant response)
search_papers({ query: "LLM", limit: 10 })
// Returns: { dataset: "search_a1b2c3d4", count: "cached", graph_schema: "..." }

// You can have multiple datasets
fetch_paper({ id: "2512.04518" })
// Returns: { dataset: "fetch_e5f6g7h8", graph_schema: "..." }
execute_cypher({ dataset: "fetch_e5f6g7h8", query: "MATCH (p) RETURN p.title" })
```

#### Example Queries

```cypher
-- Get all paper titles
MATCH (p) RETURN p.title

-- Count papers by author
MATCH (p) RETURN p.authors, COUNT(*)

-- Filter papers with specific keywords in title
MATCH (p) WHERE p.title CONTAINS "GPT" RETURN p.title, p.published_date

-- Get papers with their summary
MATCH (p) RETURN p.title, p.summary LIMIT 3
```

#### Graph Schema

The `graph_schema` in the response shows you the available node types and properties:

```
Graph Schema
============

Node Types:
  (:Paper N nodes)

Properties:
  :Paper {id: STRING, title: STRING, authors: ARRAY, published_date: STRING, summary: STRING, url: STRING, pdf_url: STRING, description_paragraphs: ARRAY}
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

## CLI Usage

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
- macOS: `~/Library/Application Support/com.sonesuke.arxiv-cli/config.toml`
- Linux: `~/.config/arxiv-cli/config.toml`
- Windows: `C:\Users\{User}\AppData\Roaming\sonesuke\arxiv-cli\config\config.toml`

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

### Chrome Arguments
For Docker/devcontainer environments, you may need to pass additional Chrome flags:

```toml
browser_path = "/usr/bin/google-chrome"
chrome_args = [
    "--no-sandbox",
    "--disable-setuid-sandbox",
    "--disable-gpu"
]
```

**Note**: When the `CI` environment variable is set, the following flags are automatically added:
- `--disable-gpu`
- `--no-sandbox`
- `--disable-setuid-sandbox`

## Implementation Details
- **Stack**: Rust, Clap, Custom CDP Client (`tokio-tungstenite`), Serde, Reqwest, PDF-Extract, `mcp-sdk-rs`.
- **Search Scraping**: Uses a custom Chrome DevTools Protocol (CDP) client to handle dynamic search result loaded via JS.
- **PDF Extraction**: Downloads the PDF and extracts text using `pdf-extract`, splitting it into structured paragraphs (`description_paragraphs`).

## License
MIT
