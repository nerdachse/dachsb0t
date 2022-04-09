#[derive(Debug)]
pub enum Action {
    Respond(String),
    Broadcast(BroadcastMsg),
}

#[derive(Debug)]
pub struct BroadcastMsg {
    pub r#type: String,
    pub url: String,
}

impl BroadcastMsg {
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
