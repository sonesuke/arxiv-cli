---
name: arxiv-fetch
description: "Get complete paper details including title, authors, summary, and publication date from arXiv. Use when the user provides an arXiv ID and needs full paper information. Always use this skill for any paper fetch request — even when specific parameters are provided."
metadata:
  author: sonesuke
  version: 1.0.0
context: fork
agent: general-purpose
---

# ArXiv Fetch

Fetch detailed information about a specific paper from arXiv using the arxiv-cli MCP server.

## Purpose

Retrieve paper metadata, abstract, and optionally download the raw PDF.

## MCP Tool

Uses `fetch_paper` MCP tool provided by arxiv-cli.

## Usage

Fetch a paper, then use the returned `dataset` name to query with Cypher:

```
fetch_paper({
  id: "2301.00001"
})
# Returns dataset name like "fetch_abc123"
# Then query with execute_cypher:
execute_cypher({
  dataset: "fetch_abc123",
  query: "MATCH (p:papers) RETURN p.title, p.authors, p.summary"
})
```

**CRITICAL**: After fetching, always use `execute_cypher` to retrieve results.
Do NOT read the output JSON file directly. The JSON file is an internal
artifact — all data is available through cypher queries.

### Result Retrieval Patterns

Use these cypher patterns to retrieve paper details:

**Full paper details**:
```cypher
MATCH (p:papers) RETURN p.id, p.title, p.authors, p.summary, p.published_date, p.url, p.pdf_url
```

**Title and authors only**:
```cypher
MATCH (p:papers) RETURN p.title, p.authors
```

**Abstract/summary only**:
```cypher
MATCH (p:papers) RETURN p.summary
```

**Description paragraphs**:
```cypher
MATCH (p:papers) RETURN p.description_paragraphs
```

### Available Paper Node Fields

| Field | Description |
| --- | --- |
| `id` | arXiv ID (e.g., "2301.00001") |
| `title` | Paper title |
| `authors` | Array of author names |
| `summary` | Abstract/summary text |
| `published_date` | Publication date |
| `url` | arXiv URL |
| `pdf_url` | Direct PDF link |
| `description_paragraphs` | Array of paragraph objects (number, id, text) |

### PDF Download

To download the raw PDF:
```
fetch_paper({
  id: "2512.04518",
  raw: true
})
```
Then retrieve the PDF path with:
```cypher
MATCH (p:results) RETURN p.id, p.pdf_path
```

## Parameters

- `id` (string, required): arXiv ID of the paper (e.g., "2301.00001", "2512.04518")
- `raw` (boolean, optional): If true, downloads the raw PDF to a local temporary file and returns its path
