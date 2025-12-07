# arXiv CLI

A Rust-based command-line tool for searching and fetching papers from arXiv. It retrieves paper metadata (title, summary, authors, published date) and keeps structured data in JSON format. It also supports downloading PDFs and extracting text content as paragraphs.

## Features
- **Search papers** by free-text query.
- **Fetch paper details** by arXiv ID.
- **Formatted JSON output** including `description_paragraphs` (extracted from PDF).
- **Pagination support** via `--limit` option.
- **Date filtering** with `--before` and `--after`.
- **Raw PDF download** with `--raw` flag.
- **Headless mode** by default; use `--head` to show the browser.
- **Robust formatting**: Uses structured JSON for easy machine consumption.

## Installation
```bash
cargo install --path .
```

## Usage

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
Config file location: `~/.config/arxiv-cli/config.toml` (created on first run).

## Implementation Details
- **Stack**: Rust, Clap, Headless Chrome, Serde, Reqwest, PDF-Extract.
- **PDF Extraction**: Downloads the PDF and extracts text, splitting it into structured paragraphs (`description_paragraphs`).

## License
MIT
