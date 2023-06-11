use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use serenity::{
    all::{CommandInteraction, CommandOptionType},
    builder::{CreateCommand, CreateCommandOption, CreateEmbed, CreateEmbedFooter},
};

use super::{common::CommandResponse, CommandArguments, DiscordCommand};
use crate::{Error, Result};
pub const COMMAND: Movie = Movie;

pub struct Movie;

#[async_trait]
impl DiscordCommand for Movie {
    fn register(&self) -> CreateCommand {
        let options = vec![
            CreateCommandOption::new(CommandOptionType::String, "title", "The title of the movie")
                .required(true),
            CreateCommandOption::new(
                CommandOptionType::Integer,
                "year",
                "The Year in which the movie released",
            )
            .required(false),
            CreateCommandOption::new(
                CommandOptionType::Boolean,
                "full",
                "Respond with a fuller description of the plot (false by default)",
            ),
        ];

        CreateCommand::new(self.name())
            .description("Get information about a movie")
            .dm_permission(true)
            .set_options(options)
    }

    async fn run(&self, command: Arc<CommandInteraction>, args: CommandArguments) -> Result<CommandResponse> {
        let (title, year, full_plot) = {
            // Get the role argument
            let mut title = "";
            let mut year = None;
            let mut full_plot = false;
            for option in &command.data.options {
                match &*option.name {
                    "title" => title = option.value.as_str().ok_or(Error::InternalLogic)?,
                    "year" => year = option.value.as_i64().map(|int| int.to_string()),
                    "full" => full_plot = option.value.as_bool().unwrap_or_default(),
                    _ => return Err(Error::InternalLogic),
                }
            }

            (title, year, full_plot)
        };

        let api_key = {
            let choose_default_key = || {
                const OMDB_API_KEYS: &[&str] = &[
                    "4b447405", "eb0c0475", "7776cbde", "ff28f90b", "6c3a2d45", "b07b58c8", "ad04b643",
                    "a95b5205", "777d9323", "2c2c3314", "b5cff164", "89a9f57d", "73a9858a", "efbd8357",
                ];
                *OMDB_API_KEYS.choose(&mut thread_rng()).unwrap()
            };

            args.config
                .secrets
                .omdb_api_key
                .as_ref()
                .map_or_else(choose_default_key, String::as_str)
        };

        let movie = OmdbMovie::from_title(api_key, title, year, full_plot).await?;
        let embed: CreateEmbed = movie.into();

        Ok(CommandResponse::Embed(Box::new(embed)))
    }

    fn name(&self) -> &'static str { "movie" }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OmdbRating {
    #[serde(rename = "Source")]
    pub source: String,
    #[serde(rename = "Value")]
    pub value:  String,
}

/// Movie metadata from `OMDb`
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OmdbMovie {
    #[serde(rename = "Title")]
    title:       String,
    #[serde(rename = "Year")]
    year:        String,
    #[serde(rename = "Rated")]
    rated:       String,
    #[serde(rename = "Released")]
    released:    String,
    #[serde(rename = "Runtime")]
    runtime:     String,
    #[serde(rename = "Genre")]
    genre:       String,
    #[serde(rename = "Director")]
    director:    String,
    #[serde(rename = "Writer")]
    writer:      String,
    #[serde(rename = "Actors")]
    actors:      String,
    #[serde(rename = "Plot")]
    plot:        String,
    #[serde(rename = "Language")]
    language:    String,
    #[serde(rename = "Country")]
    country:     String,
    #[serde(rename = "Awards")]
    awards:      String,
    #[serde(rename = "Poster")]
    poster:      String,
    #[serde(rename = "Ratings")]
    ratings:     Vec<OmdbRating>,
    #[serde(rename = "Metascore")]
    metascore:   String,
    imdb_rating: String,
    imdb_votes:  String,
    #[serde(rename = "imdbID")]
    imdb_id:     String,
    #[serde(rename = "Type")]
    type_field:  String,
    #[serde(rename = "DVD")]
    dvd:         String,
    #[serde(rename = "BoxOffice")]
    box_office:  String,
    #[serde(rename = "Production")]
    production:  String,
    #[serde(rename = "Website")]
    website:     String,
    #[serde(rename = "Response")]
    response:    String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OmdbErrorResponse {
    #[serde(rename = "Response")]
    pub response: String,
    #[serde(rename = "Error")]
    pub error:    String,
}

impl OmdbMovie {
    /// Perform a title request from `OMDb`
    pub async fn from_title(
        omdb_api_key: &str,
        title: &str,
        year: Option<String>,
        full_plot: bool,
    ) -> Result<Self> {
        let year = year.map_or_else(String::new, |year| format!("&y={year}"));
        let plot = if full_plot { "&plot=full" } else { "" };
        let url = format!(
            "http://www.omdbapi.com/?t={}{year}{plot}&apikey={omdb_api_key}",
            urlencoding::encode(title)
        );

        let response = reqwest::get(&url).await?.text().await?;

        let mut movie = match serde_json::from_str::<Self>(&response) {
            Ok(x) => Ok(x),
            Err(e) => {
                let err = serde_json::from_str::<OmdbErrorResponse>(&response)
                    .map_err(|_| Error::JsonParse(e.to_string()))?;
                Err(Error::NoSearchResults(err.error))
            }
        }?;

        if full_plot {
            movie.plot = format!("||{}||", movie.plot);
        }

        Ok(movie)
    }
}

impl From<OmdbMovie> for CreateEmbed {
    fn from(value: OmdbMovie) -> Self {
        let description = format!("{}", value.plot);
        let rotten_tomatoes = {
            let rating = value.ratings.iter().find(|x| x.source == "Rotten Tomatoes");
            rating.map_or("N/A", |rating| &rating.value)
        };
        let runtime = humantime::format_duration(Duration::from_secs(
            60 * value
                .runtime
                .split(' ')
                .next()
                .unwrap_or("0")
                .parse::<u64>()
                .unwrap(),
        ))
        .to_string();

        CreateEmbed::new()
            .title(format!("{} ({})", value.title, value.year))
            .image(value.poster)
            .description(description)
            .field("MPAA Rating", value.rated, true)
            .field("Director", value.director, true)
            .field("Writer", value.writer, true)
            .field("Starring", value.actors, true)
            .field("Genre", value.genre, true)
            .field("Runtime", runtime, true)
            .field(
                "Ratings",
                format!(
                    "Metascore: {}\nIMDb:{}\nRotten Tomatoes: {rotten_tomatoes}",
                    value.metascore, value.imdb_rating
                ),
                false,
            )
            .footer(CreateEmbedFooter::new(format!("IMDb ID: {}", value.imdb_id)))
    }
}
