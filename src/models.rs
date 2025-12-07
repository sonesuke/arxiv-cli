use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Paper {
    pub id: String,
    pub title: String,
    pub authors: Vec<String>,
    pub summary: String,
    pub published_date: String,
    pub url: String,
    pub pdf_url: String,
    pub description_paragraphs: Option<Vec<Paragraph>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Paragraph {
    pub number: String,
    pub id: String,
    pub text: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paper_serialization() {
        let paper = Paper {
            id: "1".to_string(),
            title: "Test".to_string(),
            authors: vec!["Author".to_string()],
            summary: "Summary".to_string(),
            published_date: "2024".to_string(),
            url: "http://url".to_string(),
            pdf_url: "http://pdf".to_string(),
            description_paragraphs: Some(vec![Paragraph {
                number: "0001".to_string(),
                id: "".to_string(),
                text: "Text".to_string(),
            }]),
        };
        let json = serde_json::to_string(&paper).unwrap();
        assert!(json.contains("Test"));
        assert!(json.contains("description_paragraphs"));
    }
}
