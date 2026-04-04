---
name: arxiv-search
description: "Search arXiv for academic papers by query, with optional limit, date, and category filters. Always use this skill for any paper search request — even when specific parameters are provided."
metadata:
  author: sonesuke
  version: 1.0.0
context: fork
agent: general-purpose
---

# ArXiv Search

Search for papers from arXiv using the arxiv-cli MCP server.

## Purpose

Execute paper searches with various filters including query, category, date range, and result limits.

## MCP Tool

Uses `search_papers` MCP tool provided by arxiv-cli.

## Usage

Search for papers, then use the returned `dataset` name to query with Cypher:

```
search_papers({
  query: "machine learning",
  limit: 20
})
# Returns dataset name like "search_abc123"
# Then query with execute_cypher:
execute_cypher({
  dataset: "search-abc123",
  query: "MATCH (p:papers) RETURN p.id, p.title, p.authors LIMIT 5"
})
```

**CRITICAL**:
1. When the user specifies a number of papers, ALWAYS pass it as the `limit` parameter to `search_papers`. Do NOT rely on cypher LIMIT — `limit` controls how many papers are fetched from the arXiv API.
2. After searching, always use `execute_cypher` to retrieve results. Do NOT read the output JSON file directly.

### Result Retrieval Patterns

Use these cypher patterns to retrieve search results:

**Total count**:
```cypher
MATCH (p:papers) RETURN COUNT(*) AS count
```

**Top 20 papers for overview**:
```cypher
MATCH (p:papers) RETURN p.id, p.title, p.authors, p.published_date LIMIT 20
```

**Papers by specific author**:
```cypher
MATCH (p:papers) WHERE "Smith, John" IN p.authors RETURN p.id, p.title
```

**Date range summary**:
```cypher
MATCH (p:papers) RETURN p.published_date, p.title LIMIT 10
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

### Filter Examples

Search with category and date filters:
```
search_papers({
  query: "quantum computing",
  category: "cs.AI",
  after: "2025-01-01",
  limit: 10
})
```

Search with date range:
```
search_papers({
  query: "neural network",
  after: "2024-01-01",
  before: "2024-12-31",
  limit: 20
})
```

## Parameters

- `query` (string, required): Free-text search query
- `limit` (number, optional): Maximum number of results (default: 10). Pass the number the user specifies.
- `category` (string, optional): Filter by arXiv category (e.g., "cs.AI", "physics.quant-ph", "math.NA")
- `after` (string, optional): Filter by date after (YYYY-MM-DD)
- `before` (string, optional): Filter by date before (YYYY-MM-DD)
