use std::{sync::Arc, time::Duration};

use once_cell::sync::Lazy;
use regex::Regex;
use serenity::builder::{CreateEmbed, CreateEmbedAuthor};
use youtubei_rs::{
    query::player,
    types::{client::ClientConfig, query_results::PlayerResult},
};

/// A YouTube video regex that matches on youtube.com/watch and youtu.be links.
///
/// There's two match groups, one for the start of the url and one for the video ID.
pub(super) static YOUTUBE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r#"^(?:https?://)(?:(?:www\.)?youtube\.com/watch\?v=|youtu\.be/)([\w-]{11})(?:[\?&][\w-]+=[\w-]*)*$"#,
    )
    .unwrap()
});

use crate::{Error, Result};

#[derive(Debug, Clone, PartialEq)]
pub(super) struct TrackInfo {
    pub(super) title:         String,
    pub(super) url:           String,
    pub(super) duration:      Duration,
    pub(super) thumbnail_url: Option<String>,
    pub(super) author:        String,
}

impl From<TrackInfo> for CreateEmbed {
    fn from(value: TrackInfo) -> Self {
        let mut embed = CreateEmbed::new()
            .title(format!(
                "{} ({})",
                value.title,
                humantime::format_duration(value.duration)
            ))
            .url(value.url)
            .author(CreateEmbedAuthor::new(value.author));
        if let Some(thumbnail) = value.thumbnail_url {
            embed = embed.image(thumbnail);
        }
        embed
    }
}

impl TrackInfo {
    pub async fn from_youtube_url(client_config: Arc<ClientConfig>, url: &str) -> Result<Self> {
        let video_id = extract_id_from_url(url)
            .ok_or_else(|| Error::CommandMisuse(format!("\"{url}\": Isn't a YouTube video/audio URL")))?;

        let player: PlayerResult = player(String::from(video_id), String::from(""), &client_config)
            .await
            .unwrap();
        let video = player.video_details;

        // Get the thumbnail list and sort it.
        let mut thumbnails = video.thumbnail.thumbnails;
        thumbnails.sort_by(|thumbnail, other| {
            thumbnail
                .height
                .cmp(&other.height)
                .then(thumbnail.width.cmp(&other.width))
        });
        // pick the largest thumbnail that isn't in .webp format
        let thumbnail_url = thumbnails
            .into_iter()
            .filter(|x| !x.url.contains(".webp"))
            .last()
            .map(|x| x.url);

        let track_info = Self {
            title: video.title,
            url: url.to_string(),
            duration: Duration::from_secs(video.length_seconds.parse().unwrap()),
            thumbnail_url,
            author: video.author,
        };

        Ok(track_info)
    }
}


#[inline]
pub(super) fn extract_id_from_url(url: &str) -> Option<&str> {
    Some(YOUTUBE_REGEX.captures(url)?.get(1)?.as_str())
}

#[cfg(test)]

mod tests {
    use super::*;

    #[test]
    fn test_youtube_regex() {
        assert!(YOUTUBE_REGEX.is_match("https://www.youtube.com/watch?v=BbIaaxi9uAY"));
        assert!(YOUTUBE_REGEX.is_match("http://www.youtube.com/watch?v=BbIaaxi9uAY"));
        assert!(YOUTUBE_REGEX.is_match("https://youtube.com/watch?v=BbIaaxi9uAY"));
        assert!(!YOUTUBE_REGEX.is_match("youtube.com/watch?v=BbIaaxi9uAY"));

        assert_eq!(
            YOUTUBE_REGEX
                .captures("https://www.youtube.com/watch?v=BbIaaxi9uAY")
                .unwrap()
                .get(1)
                .unwrap()
                .as_str(),
            "BbIaaxi9uAY"
        );

        assert!(YOUTUBE_REGEX.is_match("https://youtu.be/BbIaaxi9uAY"));
        assert!(YOUTUBE_REGEX.is_match("http://youtu.be/BbIaaxi9uAY"));
        assert!(YOUTUBE_REGEX.is_match("https://youtu.be/BbIaaxi9uAY"));
        assert!(!YOUTUBE_REGEX.is_match("youtu.be/BbIaaxi9uAY"));
        assert!(!YOUTUBE_REGEX.is_match("https://www.youtu.be/BbIaaxi9uAY"));

        assert_eq!(
            YOUTUBE_REGEX
                .captures("https://youtu.be/BbIaaxi9uAY")
                .unwrap()
                .get(1)
                .unwrap()
                .as_str(),
            "BbIaaxi9uAY"
        );

        assert_eq!(
            extract_id_from_url("https://www.youtube.com/watch?v=7YT0rQ2eKkY").unwrap(),
            "7YT0rQ2eKkY"
        );
        assert_eq!(
            extract_id_from_url("https://youtu.be/7YT0rQ2eKkY").unwrap(),
            "7YT0rQ2eKkY"
        );
    }
}
