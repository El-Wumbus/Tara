use std::collections::HashSet;

use rustrict::Type;
use scraper::{Html, Selector};

use crate::{Error, Result};

#[derive(Clone, Debug, Eq)]
pub struct SearchResult {
    title:   String,
    snippet: String,
}

impl std::hash::Hash for SearchResult {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) { self.title.hash(state); }
}

impl std::cmp::PartialEq for SearchResult {
    fn eq(&self, other: &Self) -> bool { self.title == other.title }
}

impl std::cmp::PartialOrd for SearchResult {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> { Some(self.cmp(other)) }
}

impl std::cmp::Ord for SearchResult {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering { self.title.cmp(&other.title) }
}

impl std::fmt::Display for SearchResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "***{}***\n\t{}\n", self.title, self.snippet)
    }
}

pub async fn scrape(search_term: &str, result_count: usize) -> Result<(Vec<SearchResult>, String)> {
    use rustrict::Censor;

    // If the search term is sexual or profane, we stop here.
    if Censor::from_str(search_term)
        .analyze()
        .is(Type::SEXUAL | Type::PROFANE)
    {
        return Err(Error::InappropriateSearch(search_term.to_string()));
    }

    let search_term = urlencoding::encode(search_term);
    let url = format!("https://duckduckgo.com/html?q={search_term}");

    let client = reqwest::Client::new();
    let resp = client.get(&url).send().await.map_err(Error::HttpRequest)?;
    let document = Html::parse_document(&resp.text().await.map_err(Error::HttpRequest)?);
    let result_selector = Selector::parse(".web-result").unwrap();
    let result_title_selector = Selector::parse(".result__a").unwrap();
    let result_snippet_selector = Selector::parse(".result__snippet").unwrap();

    // We make a hashset of the results' titles. This is done to more efficiantly
    // ensure unique titles
    let mut results_hash = HashSet::new();

    let results = document
        .select(&result_selector)
        .filter_map(|result| {
            // Get the title
            let result_title = result.select(&result_title_selector).next().unwrap();
            let title = Censor::from_str(&result_title.text().collect::<String>())
                .with_censor_replacement('#')
                .censor_and_analyze();

            // If we've seen this title before, or the title is sexual or profane, we skip
            // this result.
            if results_hash.contains(&title.0) || title.1.is(Type::SEXUAL | Type::PROFANE) {
                return None;
            }

            let result_snippet = result.select(&result_snippet_selector).next().unwrap();
            let snippet = Censor::from_str(&result_snippet.text().collect::<String>())
                .with_censor_replacement('#')
                .censor_and_analyze();

            // If the snippet is sexual or profane, we skip this result
            if snippet.1.is(Type::SEXUAL | Type::PROFANE) {
                return None;
            }

            // Add the result to the list
            results_hash.insert(title.0.clone());
            Some(SearchResult {
                title:   title.0,
                snippet: snippet.0,
            })
        })
        .enumerate()
        .filter_map(|(i, x)| if i < result_count { Some(x) } else { None })
        .collect();

    Ok((results, url))
}
