use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use rand::{seq::SliceRandom, thread_rng};
use serde::{Deserialize, Serialize};
use serenity::{
    all::{CommandInteraction, CommandOptionType},
    builder::{CreateCommand, CreateCommandOption, CreateEmbed, CreateEmbedFooter},
};

use super::{
    common::CommandResponse,
    movie::{OmdbErrorResponse, OmdbRating},
    CommandArguments, DiscordCommand,
};
use crate::{Error, Result};
pub const COMMAND: Series = Series;

pub struct Series;

#[async_trait]
impl DiscordCommand for Series {
    fn register(&self) -> CreateCommand {
        let options = vec![
            CreateCommandOption::new(CommandOptionType::String, "title", "The title of the series")
                .required(true),
            CreateCommandOption::new(
                CommandOptionType::Integer,
                "year",
                "The Year in which the TV series started",
            )
            .required(false),
            CreateCommandOption::new(
                CommandOptionType::Boolean,
                "full",
                "Respond with a fuller description of the plot (false by default)",
            ),
            CreateCommandOption::new(
                CommandOptionType::Boolean,
                "episode",
                "Find for a specific episode",
            )
            .required(false),
        ];

        CreateCommand::new(self.name())
            .description("Get information about a TV series")
            .dm_permission(true)
            .set_options(options)
    }

    async fn run(&self, command: Arc<CommandInteraction>, args: CommandArguments) -> Result<CommandResponse> {
        let (title, year, full_plot, episode) = {
            // Get the role argument
            let mut title = "";
            let mut year = None;
            let mut full_plot = false;
            let mut episode = false;
            for option in &command.data.options {
                match &*option.name {
                    "title" => title = option.value.as_str().ok_or(Error::InternalLogic)?,
                    "year" => year = option.value.as_i64().map(|int| int.to_string()),
                    "full" => full_plot = option.value.as_bool().unwrap_or_default(),
                    "episode" => episode = option.value.as_bool().unwrap_or_default(),
                    _ => return Err(Error::InternalLogic),
                }
            }

            (title, year, full_plot, episode)
        };

        let api_key = {
            let choose_default_key = || *super::movie::OMDB_API_KEYS.choose(&mut thread_rng()).unwrap();

            args.config
                .secrets
                .omdb_api_key
                .as_ref()
                .map_or_else(choose_default_key, String::as_str)
        };

        let series = OmdbSeries::series(api_key, title, year, full_plot, episode).await?;
        let embed: CreateEmbed = series.into();

        Ok(CommandResponse::Embed(Box::new(embed)))
    }

    fn name(&self) -> &'static str { "series" }
}

/// Movie metadata from `OMDb`
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct OmdbSeries {
    #[serde(rename = "Title")]
    title:         String,
    #[serde(rename = "Year")]
    year:          String,
    #[serde(rename = "Rated")]
    rated:         String,
    #[serde(rename = "Released")]
    released:      String,
    #[serde(rename = "Runtime")]
    runtime:       String,
    #[serde(rename = "Genre")]
    genre:         String,
    #[serde(rename = "Director")]
    director:      String,
    #[serde(rename = "Writer")]
    writer:        String,
    #[serde(rename = "Actors")]
    actors:        String,
    #[serde(rename = "Plot")]
    plot:          String,
    #[serde(rename = "Language")]
    language:      String,
    #[serde(rename = "Country")]
    country:       String,
    #[serde(rename = "Awards")]
    awards:        String,
    #[serde(rename = "Poster")]
    poster:        String,
    #[serde(rename = "Ratings")]
    ratings:       Vec<OmdbRating>,
    #[serde(rename = "Metascore")]
    metascore:     String,
    imdb_rating:   String,
    imdb_votes:    String,
    #[serde(rename = "imdbID")]
    imdb_id:       String,
    #[serde(rename = "Type")]
    type_field:    String,
    total_seasons: String,
    // #[serde(rename = "DVD")]
    // dvd:         Option<String>,
    // #[serde(rename = "BoxOffice")]
    // box_office:  Option<String>,
    // #[serde(rename = "Production")]
    // production:  String,
    // #[serde(rename = "Website")]
    // website:     String,
    // #[serde(rename = "Response")]
    // response:    String,
}

impl From<OmdbSeries> for CreateEmbed {
    fn from(value: OmdbSeries) -> Self {
        let description = value.plot.to_string();
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
            .field("TV Rating", value.rated, true)
            .field("Director", value.director, true)
            .field("Writer", value.writer, true)
            .field("Starring", value.actors, true)
            .field("Genre", value.genre, true)
            .field("Runtime", runtime, true)
            .field("Seasons", value.total_seasons, true)
            .field("Ratings", format!("IMDb:{}", value.imdb_rating), true)
            .footer(CreateEmbedFooter::new(format!("IMDb ID: {}", value.imdb_id)))
    }
}


impl OmdbSeries {
    pub async fn series(
        omdb_api_key: &str,
        title: &str,
        year: Option<String>,
        full_plot: bool,
        episode: bool,
    ) -> Result<Self> {
        let year = year.map_or_else(String::new, |year| format!("&y={year}"));
        let plot = if full_plot { "&plot=full" } else { "" };
        let kind = if episode { "episode" } else { "series" };
        let url = format!(
            "http://www.omdbapi.com/?t={}{year}{plot}&apikey={omdb_api_key}&type={kind}",
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
