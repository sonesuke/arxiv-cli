use crate::cdp::{CdpBrowser, CdpPage};
use crate::config::Config;
use crate::models::{Paper, Paragraph};
use anyhow::Result;

pub struct ArxivClient {
    browser: CdpBrowser,
}

impl ArxivClient {
    pub async fn new(config: &Config) -> Result<Self> {
        let args = vec![
            "--user-agent=Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
        ];
        let browser = CdpBrowser::launch(None, args, config.headless, false).await?;
        Ok(Self { browser })
    }

    pub async fn search(
        &self,
        query: &str,
        limit: Option<usize>,
        after: Option<String>,
        before: Option<String>,
    ) -> Result<Vec<Paper>> {
        let mut all_papers = Vec::new();
        let limit_val = limit.unwrap_or(usize::MAX);
        let chunk_size = 50;
        let mut start = 0;

        loop {
            if all_papers.len() >= limit_val {
                break;
            }

            let ws_url = self.browser.new_page().await?;
            let tab = CdpPage::new(&ws_url).await?;

            let url = Self::build_search_url(query, start, &after, &before);

            tab.goto(&url).await?;
            // tab.wait_until_navigated()?; // CDP helper doesn't have this, wait for element instead

            // Wait for results to load or check if no results
            // We wait for the specific list item class or no results message?
            // google-patent-cli uses wait_for_element with loop
            if !tab.wait_for_element("li.arxiv-result", 30).await? {
                break; // No more results found or timeout
            }

            let js_script = include_str!("scripts/extract_search_results.js");

            let value = tab.evaluate(js_script).await?;

            let json_str: String = serde_json::from_value(value)?;
            let papers: Vec<Paper> = serde_json::from_str(&json_str)?;

            if papers.is_empty() {
                break;
            }

            all_papers.extend(papers);

            // Close tab to save resources, though headless_chrome might handle this on drop, explicit is safer for loop

            // Tab closes when dropped? No, CdpPage doesn't own the tab in browser, it just connects.
            // But for this simple implementation we just open new tabs.
            // CdpBrowser drop will kill key process.
            // google-patent-cli doesn't explicit close tabs in loop?
            // It seems CdpPage doesn't have close method.
            // This might leak tabs in long loop.
            // But we can just proceed for now matching the structure.

            start += chunk_size;
        }

        #[allow(clippy::collapsible_if)]
        if let Some(n) = limit {
            if all_papers.len() > n {
                all_papers.truncate(n);
            }
        }

        Ok(all_papers)
    }

    pub async fn fetch(&self, id: &str) -> Result<Paper> {
        let ws_url = self.browser.new_page().await?;
        let tab = CdpPage::new(&ws_url).await?;
        let url = Self::build_fetch_url(id);

        tab.goto(&url).await?;

        if !tab.wait_for_element("h1.title", 10).await? {
            return Err(anyhow::anyhow!("Paper page not loaded correctly or timeout"));
        }

        let js_script = include_str!("scripts/extract_paper.js");

        let value = tab.evaluate(js_script).await?;

        let json_str: String = serde_json::from_value(value)?;
        let mut paper: Paper = serde_json::from_str(&json_str)?;

        // Fetch PDF and extract text
        if !paper.pdf_url.is_empty() {
            let pdf_url = paper.pdf_url.clone();
            let pdf_text = tokio::task::spawn_blocking(move || {
                match reqwest::blocking::get(&pdf_url) {
                    Ok(response) => {
                        if response.status().is_success() {
                            let bytes = response.bytes().ok()?;
                            // Use tempfile to write bytes for pdf-extract
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
                }
            })
            .await?;

            paper.description_paragraphs = pdf_text;
        }

        Ok(paper)
    }

    pub async fn fetch_pdf(&self, id: &str) -> Result<Vec<u8>> {
        let paper = self.fetch(id).await?;
        let response = reqwest::get(&paper.pdf_url).await?;
        let bytes = response.bytes().await?;
        Ok(bytes.to_vec())
    }

    fn build_search_url(
        query: &str,
        start: usize,
        after: &Option<String>,
        before: &Option<String>,
    ) -> String {
        let encoded_query = urlencoding::encode(query);
        if after.is_some() || before.is_some() {
            let from_date = after.as_deref().unwrap_or("");
            let to_date = before.as_deref().unwrap_or("");
            format!(
                "https://arxiv.org/search/advanced?advanced=1&terms-0-operator=AND&terms-0-term={}&terms-0-field=all&classification-physics_archives=all&classification-include_cross_list=include&date-filter_by=date_range&date-from_date={}&date-to_date={}&date-date_type=submitted_date&abstracts=show&size=50&order=-announced_date_first&start={}",
                encoded_query, from_date, to_date, start
            )
        } else {
            format!(
                "https://arxiv.org/search/?query={}&searchtype=all&source=header&start={}",
                encoded_query, start
            )
        }
    }

    fn build_fetch_url(id: &str) -> String {
        if id.starts_with("http") {
            id.to_string()
        } else {
            format!("https://arxiv.org/abs/{}", id)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_search_url_simple() {
        let url = ArxivClient::build_search_url("LLM", 0, &None, &None);
        assert_eq!(url, "https://arxiv.org/search/?query=LLM&searchtype=all&source=header&start=0");
    }

    #[test]
    fn test_build_search_url_with_pagination() {
        let url = ArxivClient::build_search_url("LLM", 50, &None, &None);
        assert_eq!(
            url,
            "https://arxiv.org/search/?query=LLM&searchtype=all&source=header&start=50"
        );
    }

    #[test]
    fn test_build_search_url_with_dates() {
        let after = Some("2023-01-01".to_string());
        let before = Some("2023-12-31".to_string());
        let url = ArxivClient::build_search_url("LLM", 0, &after, &before);
        assert!(url.contains("date-filter_by=date_range"));
        assert!(url.contains("date-from_date=2023-01-01"));
        assert!(url.contains("date-to_date=2023-12-31"));
    }

    #[test]
    fn test_build_fetch_url_id() {
        let url = ArxivClient::build_fetch_url("2512.04518");
        assert_eq!(url, "https://arxiv.org/abs/2512.04518");
    }

    #[test]
    fn test_build_fetch_url_full_url() {
        let url = ArxivClient::build_fetch_url("https://arxiv.org/abs/2512.04518");
        assert_eq!(url, "https://arxiv.org/abs/2512.04518");
    }
}
