use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Music {
    /// Is music playback through YouTube enabled?
    pub enabled: bool,
}

impl Default for Music {
    fn default() -> Self { Self { enabled: true } }
}
