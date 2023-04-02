use serde::{Deserialize, Serialize};

use crate::{Error, Result};
#[derive(Debug, Default, Clone, PartialEq, PartialOrd)]
pub struct Image
{
    link: String,
}

impl std::fmt::Display for Image
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{}", self.link) }
}

impl From<DogImage> for Image
{
    fn from(value: DogImage) -> Self { Self { link: value.message } }
}

impl From<CatImage> for Image
{
    fn from(value: CatImage) -> Self { Self { link: value.url } }
}

/// A random image of a dog
#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatImage
{
    pub id:     String,
    pub url:    String,
    pub width:  i64,
    pub height: i64,
}

impl CatImage
{
    async fn random() -> Result<Self>
    {
        // Request URL
        const URL: &str = "https://api.thecatapi.com/v1/images/search";
        pub type CatImages = Vec<CatImage>;

        // Get the response
        let response = match reqwest::get(URL).await {
            Ok(x) => x,
            Err(e) => return Err(Error::HttpRequest(e)),
        };

        // Parse the response
        let image = match response.json::<CatImages>().await {
            Ok(x) => Ok(x),
            Err(e) => Err(Error::JsonParse(e.to_string())),
        }?;

        Ok(image.get(0).unwrap().clone())
    }
}

/// A random image of a dog
#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct DogImage
{
    status:  String,
    message: String,
}

impl DogImage
{
    async fn random() -> Result<Self>
    {
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

pub async fn random_cat() -> Result<String> { Ok(Image::from(CatImage::random().await?).link) }
pub async fn random_dog() -> Result<String> { Ok(Image::from(DogImage::random().await?).link) }
