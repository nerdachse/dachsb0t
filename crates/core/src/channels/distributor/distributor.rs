use serde::{Deserialize, Serialize};

// FIXME there is no need for this to be public
pub const NERDHUBRAUM_WEBSOCKET_PATH: &str = "ws://localhost:13337/ws";

#[derive(Serialize, Deserialize)]
pub struct NerdHubRaumMsg {
    pub r#type: String,
    pub url: String,
}

impl NerdHubRaumMsg {
    pub fn new<S>(r#type: S, url: S) -> Self
    where
        S: Into<String>,
    {
        Self {
            r#type: r#type.into(),
            url: url.into(),
        }
    }
}
