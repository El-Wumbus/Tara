use serde::{Deserialize, Serialize};

use crate::{Error, Result};
#[derive(Debug, Default, Clone, PartialEq, PartialOrd)]
pub(super) struct Image {
    pub(super) link: String,
}

impl std::fmt::Display for Image {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{}", self.link) }
}

impl From<DogImage> for Image {
    fn from(value: DogImage) -> Self { Self { link: value.message } }
}

impl From<CatImage> for Image {
    fn from(value: CatImage) -> Self { Self { link: value.url } }
}

/// A random image of a dog
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct CatImage {
    id:     String,
    url:    String,
    width:  i64,
    height: i64,
}

impl Default for CatImage {
    fn default() -> Self {
        Self {
            id:     "NOT FOUND".to_string(),
            url:    Default::default(),
            width:  Default::default(),
            height: Default::default(),
        }
    }
}

impl CatImage {
    pub async fn random() -> Result<Self> {
        // Request URL
        const RANDOM_CAT_IMAGE_URL: &str = "https://api.thecatapi.com/v1/images/search";
        let response = reqwest::get(RANDOM_CAT_IMAGE_URL).await?;

        // Parse the response
        let image = response
            .json::<Vec<CatImage>>()
            .await
            .map_err(|e| Error::JsonParse(e.to_string()))?;

        image
            .get(0)
            .cloned()
            .ok_or_else(|| Error::Unexpected("Server returned an empty list of results!"))
    }
}

/// A random image of a dog
#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub(super) struct DogImage {
    status:  String,
    message: String,
}

impl DogImage {
    pub async fn random() -> Result<Self> {
        // Request URL
        const URL: &str = "https://dog.ceo/api/breeds/image/random";

        // Get the response
        let response = match reqwest::get(URL).await {
            Ok(x) => x,
            Err(e) => return Err(Error::HttpRequest(e)),
        };

        // Parse the response
        let image = match response.json::<DogImage>().await {
            Ok(x) => Ok(x),
            Err(e) => Err(Error::JsonParse(e.to_string())),
        }?;

        Ok(image)
    }
}
