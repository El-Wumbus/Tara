use serde::{Deserialize, Serialize};

use crate::{commands::CommandResponse, Error, Result};

pub async fn random() -> Result<CommandResponse> { Ok(Quote::random().await?.to_string().into()) }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Quote {
    Random(Random),
}

impl Quote {
    async fn random() -> Result<Self> { Ok(Self::Random(Random::fetch().await?)) }
}

impl std::fmt::Display for Quote {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Random(quote) => quote,
            }
        )
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Random {
    #[serde(rename = "_id")]
    pub id:            String,
    pub content:       String,
    pub author:        String,
    pub tags:          Vec<String>,
    pub author_slug:   String,
    pub length:        i64,
    pub date_added:    String,
    pub date_modified: String,
}

impl Random {
    pub async fn fetch() -> Result<Self> {
        // Construct request URL
        const URL: &str = "https://api.quotable.io/random";

        // Get the response
        let resp = match {
            match reqwest::get(URL).await {
                Ok(x) => x,
                Err(e) => return Err(Error::HttpRequest(e)),
            }
        }
        .json::<Self>()
        .await
        {
            Ok(x) => x,
            Err(e) => return Err(Error::JsonParse(e.to_string())),
        };

        Ok(resp)
    }
}

impl std::fmt::Display for Random {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "*{}*\n\t—{}", self.content.trim(), self.author)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_random_quote_display() {
        let quote = Random {
            content: "This is a quote".to_string(),
            author: "Rust Test".to_string(),
            tags: Vec::new(),
            length: 17,
            ..Default::default()
        };

        assert_eq!(quote.to_string(), "*This is a quote*\n\t—Rust Test");
    }

    #[test]
    fn test_quote_display() {
        let quote = Quote::Random(Random {
            content: "This is a quote".to_string(),
            author: "Rust Test".to_string(),
            tags: Vec::new(),
            length: 17,
            ..Default::default()
        });

        assert_eq!(quote.to_string(), "*This is a quote*\n\t—Rust Test");
    }

    #[tokio::test]
    async fn fetch_random_quote() {
        let quote = Quote::random().await.unwrap();
        let Quote::Random(quote) = quote;

        assert_eq!(
            quote.to_string(),
            format!("*{}*\n\t—{}", quote.content, quote.author)
        );
    }
}
