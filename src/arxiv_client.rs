use crate::models::{Paper, Paragraph};
use anyhow::{Context, Result};
use quick_xml::de::from_str;
use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;

/// arXiv API client for fetching papers without browser
pub struct ArxivClient {
    client: Client,
    base_url: String,
}

impl ArxivClient {
    const DEFAULT_BASE_URL: &'static str = "https://export.arxiv.org/api/query";
    const DEFAULT_CHUNK_SIZE: usize = 50;

    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("arxiv-cli/0.0.5 (https://github.com/sonesuke/arxiv-cli)")
            .build()
            .unwrap();

        Self { client, base_url: Self::DEFAULT_BASE_URL.to_string() }
    }

    /// Search for papers on arXiv using the query API
    pub async fn search(
        &self,
        query: &str,
        limit: Option<usize>,
        after: Option<String>,
        before: Option<String>,
    ) -> Result<Vec<Paper>> {
        let mut all_papers = Vec::new();
        let limit_val = limit.unwrap_or(usize::MAX);
        let chunk_size = Self::DEFAULT_CHUNK_SIZE;
        let mut start = 0;

        loop {
            if all_papers.len() >= limit_val {
                break;
            }

            let remaining = limit_val.saturating_sub(all_papers.len());
            let max_results = remaining.min(chunk_size);

            let url = self.build_search_url(query, start, max_results, &after, &before);

            let response = self.client.get(&url).send().await?;
            if !response.status().is_success() {
                return Err(anyhow::anyhow!(
                    "API request failed with status: {}",
                    response.status()
                ));
            }

            let text = response.text().await?;
            let feed: ArxivFeed = from_str(&text).with_context(|| {
                format!("Failed to parse arXiv API response: {}", &text[..200.min(text.len())])
            })?;

            if feed.entries.is_empty() {
                break;
            }

            let papers: Vec<Paper> = feed.entries.into_iter().map(|e| e.into()).collect();

            if papers.is_empty() {
                break;
            }

            all_papers.extend(papers);

            // If we got less results than requested, we've reached the end
            if all_papers.len() < start + max_results {
                break;
            }

            start += chunk_size;

            // Rate limiting: arXiv recommends 1 request per 3 seconds
            tokio::time::sleep(Duration::from_secs(3)).await;
        }

        if let Some(n) = limit {
            all_papers.truncate(n);
        }

        Ok(all_papers)
    }

    /// Fetch a single paper by ID using the id_list parameter
    pub async fn fetch(&self, id: &str) -> Result<Paper> {
        let paper_id = self.extract_paper_id(id);
        let url = format!("{}?id_list={}", self.base_url, paper_id);

        let response = self.client.get(&url).send().await?;
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("API request failed with status: {}", response.status()));
        }

        let text = response.text().await?;
        let feed: ArxivFeed = from_str(&text).with_context(|| {
            format!("Failed to parse arXiv API response: {}", &text[..200.min(text.len())])
        })?;

        if feed.entries.is_empty() {
            return Err(anyhow::anyhow!("Paper not found: {}", id));
        }

        let mut paper: Paper = feed.entries.into_iter().next().unwrap().into();

        // Fetch PDF and extract text
        if !paper.pdf_url.is_empty() {
            let pdf_text = self.extract_pdf_text(&paper.pdf_url).await?;
            paper.description_paragraphs = pdf_text;
        }

        Ok(paper)
    }

    /// Fetch PDF bytes for a paper
    pub async fn fetch_pdf(&self, id: &str) -> Result<Vec<u8>> {
        let paper = self.fetch(id).await?;
        let response = self.client.get(&paper.pdf_url).send().await?;
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Failed to download PDF: status {}", response.status()));
        }
        let bytes = response.bytes().await?;
        Ok(bytes.to_vec())
    }

    /// Extract text from PDF URL
    async fn extract_pdf_text(&self, pdf_url: &str) -> Result<Option<Vec<Paragraph>>> {
        let pdf_url = pdf_url.to_string();

        let result = tokio::task::spawn_blocking(move || match reqwest::blocking::get(&pdf_url) {
            Ok(response) => {
                if response.status().is_success() {
                    let bytes = response.bytes().ok()?;
                    let mut temp_file = tempfile::NamedTempFile::new().ok()?;
                    use std::io::Write;
                    temp_file.write_all(&bytes).ok()?;

                    match pdf_extract::extract_text(temp_file.path()) {
                        Ok(text) => {
                            let paragraphs: Vec<Paragraph> = text
                                .split("\n\n")
                                .map(|s| s.trim().to_string())
                                .filter(|s| !s.is_empty())
                                .enumerate()
                                .map(|(i, s)| Paragraph {
                                    number: format!("{:04}", i + 1),
                                    id: String::new(),
                                    text: s,
                                })
                                .collect();
                            Some(paragraphs)
                        }
                        Err(e) => {
                            eprintln!("Failed to extract text from PDF: {}", e);
                            None
                        }
                    }
                } else {
                    eprintln!("Failed to download PDF: Status {}", response.status());
                    None
                }
            }
            Err(e) => {
                eprintln!("Failed to download PDF: {}", e);
                None
            }
        })
        .await?;

        Ok(result)
    }

    /// Build search URL for arXiv API
    fn build_search_url(
        &self,
        query: &str,
        start: usize,
        max_results: usize,
        after: &Option<String>,
        before: &Option<String>,
    ) -> String {
        let search_query = if after.is_some() || before.is_some() {
            // arXiv API supports date filtering with the format:
            // search_query=all:QUERY+AND+submittedDate:[YYYYMMDD+TO+YYYYMMDD]
            let from_date = after.as_deref().unwrap_or("1900-01-01");
            let to_date = before.as_deref().unwrap_or("2099-12-31");

            // Format dates for arXiv API (YYYYMMDD)
            let from_formatted = from_date.replace("-", "");
            let to_formatted = to_date.replace("-", "");

            format!("all:{}+AND+submittedDate:[{}+TO+{}]", query, from_formatted, to_formatted)
        } else {
            format!("all:{}", query)
        };

        format!(
            "{}?search_query={}&start={}&max_results={}",
            self.base_url, search_query, start, max_results
        )
    }

    /// Extract paper ID from various formats
    fn extract_paper_id(&self, id: &str) -> String {
        if id.starts_with("http") {
            // Extract ID from URL like https://arxiv.org/abs/2301.07041
            if let Some(part) = id.rsplit('/').next() {
                return part.to_string();
            }
        }
        id.to_string()
    }
}

impl Default for ArxivClient {
    fn default() -> Self {
        Self::new()
    }
}

// ============ ArXiv API Response Models ============

/// Root feed element from arXiv Atom API
#[derive(Debug, Deserialize)]
struct ArxivFeed {
    #[serde(rename = "entry", default)]
    entries: Vec<ArxivEntry>,
}

/// Single paper entry from arXiv Atom API
#[derive(Debug, Deserialize)]
struct ArxivEntry {
    #[serde(rename = "id")]
    id: String,
    #[serde(rename = "title")]
    title: String,
    #[serde(rename = "summary")]
    summary: String,
    #[serde(rename = "published")]
    published: String,
    #[serde(rename = "updated")]
    _updated: String,
    #[serde(rename = "author")]
    authors: Vec<ArxivAuthor>,
    #[serde(rename = "link")]
    links: Vec<ArxivLink>,
    #[serde(rename = "arxiv:primary_category")]
    _primary_category: Option<ArxivCategory>,
    #[serde(rename = "arxiv:comment")]
    _comment: Option<String>,
    #[serde(rename = "arxiv:journal_ref")]
    _journal_ref: Option<String>,
    #[serde(rename = "arxiv:doi")]
    _doi: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ArxivAuthor {
    #[serde(rename = "name")]
    name: String,
}

#[derive(Debug, Deserialize)]
struct ArxivLink {
    #[serde(rename = "@href")]
    href: String,
    #[serde(rename = "@rel")]
    rel: Option<String>,
    #[serde(rename = "@type")]
    content_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ArxivCategory {
    #[serde(rename = "term")]
    _term: String,
}

// Convert ArxivEntry to our Paper model
impl From<ArxivEntry> for Paper {
    fn from(entry: ArxivEntry) -> Self {
        // Extract paper ID from URL (e.g., http://arxiv.org/abs/2301.07041)
        let id = entry.id.rsplit('/').next().unwrap_or(&entry.id).to_string();

        // Clean up title and summary (remove extra whitespace)
        let title = entry.title.trim().to_string();
        let summary = entry.summary.trim().to_string();

        // Extract authors
        let authors = entry.authors.into_iter().map(|a| a.name).collect();

        // Find PDF URL
        let pdf_url = entry
            .links
            .iter()
            .find(|l| {
                l.rel.as_deref() == Some("related")
                    || l.content_type.as_deref() == Some("application/pdf")
            })
            .map(|l| l.href.clone())
            .unwrap_or_else(|| {
                // Construct PDF URL from ID
                format!("https://arxiv.org/pdf/{}.pdf", id)
            });

        // Parse published date
        let published_date =
            entry.published.split('T').next().unwrap_or(&entry.published).to_string();

        Self {
            id,
            title,
            authors,
            summary,
            published_date,
            url: entry.id,
            pdf_url,
            description_paragraphs: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============ extract_paper_id tests ============

    #[test]
    fn test_extract_paper_id_from_url() {
        let client = ArxivClient::new();
        assert_eq!(client.extract_paper_id("https://arxiv.org/abs/2301.07041"), "2301.07041");
    }

    #[test]
    fn test_extract_paper_id_from_id() {
        let client = ArxivClient::new();
        assert_eq!(client.extract_paper_id("2301.07041"), "2301.07041");
    }

    #[test]
    fn test_extract_paper_id_from_http_url() {
        let client = ArxivClient::new();
        assert_eq!(client.extract_paper_id("http://arxiv.org/abs/1234.5678v2"), "1234.5678v2");
    }

    // ============ build_search_url tests ============

    #[test]
    fn test_build_search_url_simple() {
        let client = ArxivClient::new();
        let url = client.build_search_url("machine learning", 0, 50, &None, &None);
        assert!(url.contains("search_query=all:machine learning"));
        assert!(url.contains("start=0"));
        assert!(url.contains("max_results=50"));
    }

    #[test]
    fn test_build_search_url_with_dates() {
        let client = ArxivClient::new();
        let after = Some("2023-01-01".to_string());
        let before = Some("2023-12-31".to_string());
        let url = client.build_search_url("LLM", 0, 50, &after, &before);
        assert!(url.contains("search_query=all:LLM"));
        assert!(url.contains("submittedDate:[20230101+TO+20231231]"));
    }

    #[test]
    fn test_build_search_url_with_after_only() {
        let client = ArxivClient::new();
        let after = Some("2023-06-01".to_string());
        let url = client.build_search_url("AI", 0, 10, &after, &None);
        assert!(url.contains("submittedDate:[20230601+TO+20991231]"));
    }

    #[test]
    fn test_build_search_url_with_before_only() {
        let client = ArxivClient::new();
        let before = Some("2023-06-01".to_string());
        let url = client.build_search_url("AI", 0, 10, &None, &before);
        assert!(url.contains("submittedDate:[19000101+TO+20230601]"));
    }

    #[test]
    fn test_build_search_url_with_start_offset() {
        let client = ArxivClient::new();
        let url = client.build_search_url("test", 100, 25, &None, &None);
        assert!(url.contains("start=100"));
        assert!(url.contains("max_results=25"));
    }

    // ============ XML deserialization tests ============

    fn sample_arxiv_xml() -> &'static str {
        r#"<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns="http://www.w3.org/2005/Atom"
      xmlns:arxiv="http://arxiv.org/schemas/atom">
  <title>ArXiv Query</title>
  <entry>
    <id>http://arxiv.org/abs/2301.07041v1</id>
    <title>  Test Paper Title  </title>
    <summary>  This is a test summary.  </summary>
    <published>2023-01-17T18:59:59Z</published>
    <updated>2023-01-17T18:59:59Z</updated>
    <author><name>Alice Smith</name></author>
    <author><name>Bob Jones</name></author>
    <link href="http://arxiv.org/abs/2301.07041v1" rel="alternate" type="text/html"/>
    <link href="http://arxiv.org/pdf/2301.07041v1" rel="related" type="application/pdf" title="pdf"/>
    <arxiv:primary_category term="cs.AI"/>
    <arxiv:comment>10 pages, 5 figures</arxiv:comment>
  </entry>
</feed>"#
    }

    #[test]
    fn test_parse_arxiv_feed_xml() {
        let feed: ArxivFeed = from_str(sample_arxiv_xml()).unwrap();
        assert_eq!(feed.entries.len(), 1);

        let entry = &feed.entries[0];
        assert_eq!(entry.id, "http://arxiv.org/abs/2301.07041v1");
        assert!(entry.title.contains("Test Paper Title"));
        assert!(entry.summary.contains("test summary"));
        assert_eq!(entry.authors.len(), 2);
        assert_eq!(entry.authors[0].name, "Alice Smith");
        assert_eq!(entry.authors[1].name, "Bob Jones");
        assert_eq!(entry.published, "2023-01-17T18:59:59Z");
        assert_eq!(entry.links.len(), 2);
    }

    #[test]
    fn test_parse_arxiv_feed_empty_entries() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns="http://www.w3.org/2005/Atom"
      xmlns:arxiv="http://arxiv.org/schemas/atom">
  <title>ArXiv Query</title>
</feed>"#;
        let feed: ArxivFeed = from_str(xml).unwrap();
        assert!(feed.entries.is_empty());
    }

    #[test]
    fn test_parse_arxiv_feed_multiple_entries() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns="http://www.w3.org/2005/Atom"
      xmlns:arxiv="http://arxiv.org/schemas/atom">
  <title>ArXiv Query</title>
  <entry>
    <id>http://arxiv.org/abs/0001.0001v1</id>
    <title>Paper One</title>
    <summary>Summary one</summary>
    <published>2023-01-01T00:00:00Z</published>
    <updated>2023-01-01T00:00:00Z</updated>
    <author><name>Author A</name></author>
    <link href="http://arxiv.org/abs/0001.0001v1" rel="alternate" type="text/html"/>
  </entry>
  <entry>
    <id>http://arxiv.org/abs/0002.0002v1</id>
    <title>Paper Two</title>
    <summary>Summary two</summary>
    <published>2023-02-01T00:00:00Z</published>
    <updated>2023-02-01T00:00:00Z</updated>
    <author><name>Author B</name></author>
    <link href="http://arxiv.org/abs/0002.0002v1" rel="alternate" type="text/html"/>
  </entry>
</feed>"#;
        let feed: ArxivFeed = from_str(xml).unwrap();
        assert_eq!(feed.entries.len(), 2);
    }

    // ============ ArxivEntry -> Paper conversion tests ============

    #[test]
    fn test_entry_to_paper_conversion() {
        let feed: ArxivFeed = from_str(sample_arxiv_xml()).unwrap();
        let paper: Paper = feed.entries.into_iter().next().unwrap().into();

        assert_eq!(paper.id, "2301.07041v1");
        assert_eq!(paper.title, "Test Paper Title");
        assert_eq!(paper.summary, "This is a test summary.");
        assert_eq!(paper.authors, vec!["Alice Smith", "Bob Jones"]);
        assert_eq!(paper.published_date, "2023-01-17");
        assert_eq!(paper.url, "http://arxiv.org/abs/2301.07041v1");
        assert_eq!(paper.pdf_url, "http://arxiv.org/pdf/2301.07041v1");
        assert!(paper.description_paragraphs.is_none());
    }

    #[test]
    fn test_entry_to_paper_pdf_url_fallback() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns="http://www.w3.org/2005/Atom"
      xmlns:arxiv="http://arxiv.org/schemas/atom">
  <title>ArXiv Query</title>
  <entry>
    <id>http://arxiv.org/abs/9999.1234v1</id>
    <title>No PDF Link Paper</title>
    <summary>Summary</summary>
    <published>2023-05-01T00:00:00Z</published>
    <updated>2023-05-01T00:00:00Z</updated>
    <author><name>Test Author</name></author>
    <link href="http://arxiv.org/abs/9999.1234v1" rel="alternate" type="text/html"/>
  </entry>
</feed>"#;
        let feed: ArxivFeed = from_str(xml).unwrap();
        let paper: Paper = feed.entries.into_iter().next().unwrap().into();

        // Should fallback to constructed PDF URL
        assert_eq!(paper.pdf_url, "https://arxiv.org/pdf/9999.1234v1.pdf");
    }

    #[test]
    fn test_entry_to_paper_title_trimmed() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns="http://www.w3.org/2005/Atom"
      xmlns:arxiv="http://arxiv.org/schemas/atom">
  <title>ArXiv Query</title>
  <entry>
    <id>http://arxiv.org/abs/1111.2222v1</id>
    <title>
      Whitespace Title
    </title>
    <summary>
      Whitespace Summary
    </summary>
    <published>2023-03-15T12:00:00Z</published>
    <updated>2023-03-15T12:00:00Z</updated>
    <author><name>Author</name></author>
    <link href="http://arxiv.org/abs/1111.2222v1" rel="alternate" type="text/html"/>
  </entry>
</feed>"#;
        let feed: ArxivFeed = from_str(xml).unwrap();
        let paper: Paper = feed.entries.into_iter().next().unwrap().into();

        assert_eq!(paper.title, "Whitespace Title");
        assert_eq!(paper.summary, "Whitespace Summary");
    }

    #[test]
    fn test_entry_to_paper_date_parsing() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns="http://www.w3.org/2005/Atom"
      xmlns:arxiv="http://arxiv.org/schemas/atom">
  <title>ArXiv Query</title>
  <entry>
    <id>http://arxiv.org/abs/3333.4444v1</id>
    <title>Date Test</title>
    <summary>Summary</summary>
    <published>2024-12-25T08:30:00Z</published>
    <updated>2024-12-25T08:30:00Z</updated>
    <author><name>Author</name></author>
    <link href="http://arxiv.org/abs/3333.4444v1" rel="alternate" type="text/html"/>
  </entry>
</feed>"#;
        let feed: ArxivFeed = from_str(xml).unwrap();
        let paper: Paper = feed.entries.into_iter().next().unwrap().into();

        assert_eq!(paper.published_date, "2024-12-25");
    }

    // ============ Default impl test ============

    #[test]
    fn test_arxiv_client_default() {
        let client = ArxivClient::default();
        assert_eq!(client.base_url, "https://export.arxiv.org/api/query");
    }
}
