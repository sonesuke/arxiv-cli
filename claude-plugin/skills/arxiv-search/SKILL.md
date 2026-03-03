# ArXiv Search

Search arXiv for academic papers matching your query. Results are cached for efficient repeated queries.

## Usage

```
arxiv-search <query> [limit]
```

## Arguments

- `query` (required): The search query (e.g., "LLM", "quantum computing")
- `limit` (optional): Maximum number of results to return (default: 10)

## Examples

```
arxiv-search "LLM" 10
arxiv-search "quantum computing" 5
arxiv-search "neural networks"
```

## Notes

- The search results are automatically cached (up to 100 recent queries)
- Same query parameters will return cached results instantly
- Use the returned dataset name with Cypher queries for filtering
