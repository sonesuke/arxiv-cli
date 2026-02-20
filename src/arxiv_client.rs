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
        let client = Client::builder().timeout(Duration::from_secs(30)).build().unwrap();

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
    #[serde(rename = "entry")]
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
    fn test_build_search_url_simple() {
        let client = ArxivClient::new();
        let url = client.build_search_url("machine learning", 0, 50, &None, &None);
        assert!(url.contains("search_query=machine+learning"));
        assert!(url.contains("start=0"));
        assert!(url.contains("max_results=50"));
    }

    #[test]
    fn test_build_search_url_with_dates() {
        let client = ArxivClient::new();
        let after = Some("2023-01-01".to_string());
        let before = Some("2023-12-31".to_string());
        let url = client.build_search_url("LLM", 0, 50, &after, &before);
        assert!(url.contains("search_query=LLM"));
        assert!(url.contains("submittedDate:[20230101-20231231]"));
    }
}
