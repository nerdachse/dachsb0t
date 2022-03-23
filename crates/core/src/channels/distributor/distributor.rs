const NERDHUBRAUM_WEBSOCKET_PATH: &str = "ws://localhost:13337/ws";

const MEME_TEST_URL: &str = "https://ih1.redbubble.net/image.875111905.4798/flat,750x,075,f-pad,750x1000,f8f8f8.jpg";

#[derive(Serialize, Deserialize)]
struct NerdHubRaumMsg {
    r#type: String,
    url: String,
}

