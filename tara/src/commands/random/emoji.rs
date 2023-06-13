use serde::{Deserialize, Serialize};

use crate::{Error, Result};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Emoji {
    // name:      String,
    // category:  String,
    // group:     String,
    // html_code: Vec<String>,
    unicode:   Vec<String>,
}

pub(super) async fn random_emoji() -> Result<char> {
    let emoji = reqwest::get("https://emojihub.yurace.pro/api/random")
        .await?
        .json::<Emoji>()
        .await
        .map_err(|e| Error::JsonParse(e.to_string()))?;

    let emoji_unicode_str = dbg!(emoji.unicode.get(0).ok_or(Error::InternalLogic)?);
    let Ok(emoji_unicode) = sscanf::sscanf!(emoji_unicode_str, "U+{u32:x}")
        else {return Err(Error::Unexpected("Emoji API returned a different format for their unicode characters than expected!"))};
    char::from_u32(emoji_unicode).ok_or(Error::Unexpected("Emoji API returned invalid unicode!"))
}
