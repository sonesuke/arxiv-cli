# ArXiv Fetch

Fetch detailed information about a specific paper from arXiv by its ID.

## Usage

```
arxiv-fetch <arxiv_id>
```

## Arguments

- `arxiv_id` (required): The arXiv ID of the paper (e.g., "2301.00001", "cs.AI/2301.00001")

## Examples

```
arxiv-fetch "2301.00001"
arxiv-fetch "cs.AI/2301.00001"
```

## Notes

- The paper details are automatically cached (up to 100 recent fetches)
- Same arxiv_id will return cached results instantly
- Returns full metadata including title, authors, summary, and description paragraphs
