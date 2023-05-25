use std::sync::Arc;

use log::info;
use serde::{Deserialize, Serialize};

use crate::{Error, Result};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RPage {
    pub pageid:  i64,
    pub ns:      i64,
    pub title:   String,
    pub extract: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Query {
    pub pages: Vec<RPage>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SummaryResponse {
    pub batchcomplete: bool,
    pub query:         Query,
}

#[derive(Clone, PartialEq, PartialOrd, Ord, Eq, Debug)]
/// The result of a search operation.
pub struct Page {
    /// Title of the page
    pub title: Arc<str>,

    /// The URL of the page
    pub url: Arc<str>,
}

impl Page {
    /// Create a new `Page`
    pub fn new(title: String, url: String) -> Self {
        Self {
            title: Arc::from(title),
            url:   Arc::from(url),
        }
    }

    /// Search for a page on Wikipedia and return a `Page`
    pub async fn search(search_term: &str) -> Result<Self> {
        type SearchResult = (String, Vec<String>, Vec<String>, Vec<String>);

        // Replace spaces with %20 for the url
        let title = search_term.replace(' ', "%20");

        let request_url = format!(
            "https://en.wikipedia.org/w/api.php?action=opensearch&search={}&limit=1&namespace=0&format=json",
            title.trim()
        );

        // Make the API call, parse the json to a `Page`.
        let page = match {
            match reqwest::get(&request_url).await {
                Ok(x) => {
                    info!("Requested '{}'", request_url);
                    x
                }
                Err(e) => return Err(Error::HttpRequest(e)),
            }
            .json::<SearchResult>()
            .await
        } {
            Ok(resp) => {
                let t = match resp.1.get(0) {
                    Some(x) => x.to_string(),
                    None => return Err(Error::WikipedaSearch(search_term.to_string())),
                };

                let u = match resp.3.get(0) {
                    Some(x) => x.to_string(),
                    None => return Err(Error::WikipedaSearch(search_term.to_string())),
                };

                Self::new(t, u)
            }
            Err(e) => return Err(Error::JsonParse(e.to_string())),
        };


        Ok(page)
    }

    pub async fn get_summary(self) -> Result<String> {
        let request_url =
        format!(
            "https://en.wikipedia.org/w/api.php?action=query&format=json&prop=extracts&titles={}&formatversion=2&exchars=1000&explaintext=1&redirects=1",
            self.title
        );

        // Make the API call, parse the json to a `Page`.
        let resp = match {
            match reqwest::get(&request_url).await {
                Ok(x) => {
                    info!("Requested '{}'", request_url);
                    x
                }
                Err(e) => return Err(Error::HttpRequest(e)),
            }
            .json::<SummaryResponse>()
            .await
        } {
            Ok(x) => x,
            Err(e) => return Err(Error::JsonParse(e.to_string())),
        };

        let summary_text = resp.query.pages.get(0).unwrap().extract.to_owned();

        Ok(summary_text)
    }
}

#[cfg(test)]

pub mod tests {
    use super::Page;

    #[tokio::test]
    async fn test_search_page() {
        let expected_page = Page::new(
            "Albert Einstein".to_string(),
            "https://en.wikipedia.org/wiki/Albert_Einstein".to_string(),
        );
        let page = Page::search("Albert Einstein").await.unwrap();
        assert_eq!(page, expected_page);
    }

    #[tokio::test]
    async fn test_search_page_misspelled() {
        let expected_page = Page::new(
            "Programming language".to_string(),
            "https://en.wikipedia.org/wiki/Programming_language".to_string(),
        );
        let page = Page::search("progrmming lang").await.unwrap();
        assert_eq!(page, expected_page);
    }

    #[tokio::test]
    async fn test_get_page_summary() {
        let page = Page::search("Albert Einstein").await.unwrap();
        let r = page.get_summary().await;
        assert!(r.is_ok());
    }
}
