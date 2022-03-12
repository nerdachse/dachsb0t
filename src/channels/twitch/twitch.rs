use futures_util::{
    SinkExt, StreamExt,
};
use std::{env, time::Duration};

use tokio_tungstenite::{
    connect_async, tungstenite::protocol::Message, tungstenite::Error as TError
};

use tracing::{error, info, warn};
use url::Url;

use async_trait::async_trait;
use once_cell::sync::Lazy;
use regex::Regex;

use tokio::sync::Mutex;
use tokio::task;

use std::sync::Arc;

use crate::channels::websocket::{WebSocketRead, WebSocketWrite};
use crate::features::announce;

const TWITCH_CB_TOKEN: &'static str = "twitch_chat_bot_auth_token";
const TWITCH_CHAT_URL: &'static str = "wss://irc-ws.chat.twitch.tv:443";
const TWITCH_CHANNEL: &'static str = "nerdachse";
const TWITCH_BOT_NAME: &'static str = "dachsb0t";

static IRC_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"([\S]*) ([\S]*) ([\S]*) (.*)").expect("failed to compile regex"));

pub async fn start() -> () {
    let token = env::var(TWITCH_CB_TOKEN).expect("No twitch_chat_bot_auth_token provided in env");
    let url = Url::parse(TWITCH_CHAT_URL).expect("Couldn't parse TWITCH_CHAT_URL");
    let (ws_stream, _) = connect_async(url)
        .await
        .expect("Failed to connect to TWITCH_CHAT_URL");
    let (mut write, mut read) = ws_stream.split();

    // Only temporary join channel before
    auth(&token, &mut write).await;

    let write = Arc::new(Mutex::new(write));
    let announce_discord = task::spawn(announce(Duration::from_secs(1800), "Wusstest du schon, es gibt auch einen Discord?! Nein? Jetzt aber! https://discord.gg/Yf9MUJv3mr", write.clone()));
    let announce_github = task::spawn(announce(Duration::from_secs(3600), "Interesse an den Programmierstreams? Dann schau mal auf github vorbei: https://github.com/nerdachse", write.clone()));
    let handle_messages = task::spawn(async move {handle_requests(&mut read, write).await});
    let _ = announce_discord.await;
    let _ = announce_github.await;
    let _ = handle_messages.await;
}

async fn handle_twitch_irc_command(code: &str, write: &mut WebSocketWrite) {
    match code {
        "001" => {
            write
                .send(Message::Text(format!("CAP REQ :twitch.tv/tags")))
                .await
                .expect("Failed to request capabilities");
            write
                .send(Message::Text(format!("JOIN #{TWITCH_CHANNEL}")))
                .await
                .expect("Failed to connect to channel");
        }
        "CAP" => info!("Got capabilities, yeah!"),
        code => warn!("Unhandled code: {code}"),
    }
}

#[async_trait]
pub trait TwitchChatSender {
    async fn send_priv_msg(&mut self, msg: &str) -> Result<(), TError>;
}

#[async_trait]
impl TwitchChatSender for WebSocketWrite {
    async fn send_priv_msg(&mut self, msg: &str) -> Result<(), TError> {
        self.send(Message::Text(format!("PRIVMSG #{TWITCH_CHANNEL} :{msg}")))
            .await
    }
}

async fn handle_twitch_message(msg: &str, write: &mut WebSocketWrite) {
    let text = msg.split(":").last();
    if let Some(text) = text {
        if text.starts_with("!hallo") {
            if let Err(_) = write.send_priv_msg("Howdy").await {
                error!("Failed to send response for !hallo");
            }
        }
    }
}

async fn auth(token: &str, write: &mut WebSocketWrite) {
    write
        .send(Message::Text(format!("PASS oauth:{token}")))
        .await
        .expect("Failed to send oauth token");
    write
        .send(Message::Text(format!("NICK {TWITCH_BOT_NAME}")))
        .await
        .expect("Failed to send nick");
}

async fn handle_requests(read: &mut WebSocketRead, write: Arc<Mutex<WebSocketWrite>>) -> () {
    // Result<(), Box<dyn Error>> {
    while let Some(msg) = read.next().await {
        match msg {
            Ok(msg) => {
                if msg.is_text() {
                    if let Ok(text) = msg.into_text() {
                        handle_text_msg(text, write.clone()).await;
                    }
                } else if msg.is_ping() {
                    info!("I got a websocket ping, OMG!");
                } else {
                    warn!("Unhandled msg: {msg}");
                }
            },
            Err(e) => warn!("Failed to get msg: {e}"),
        }
    }
}

async fn handle_text_msg(text: String, write: Arc<Mutex<WebSocketWrite>>) {
    // One websocket message can incorporate multiple lines, but IRC is a
    // line-based protocol
    let mut lines = text.lines();
    while let Some(text) = lines.next() {
        match text.chars().nth(0) {
            // Normal IRC message
            Some(':') => {
                //info!("irc msg: {:?}", text);
                if let Some(caps) = IRC_REGEX.captures(&text) {
                    let _sender = caps.get(1);
                    let code = caps.get(2).expect("no code");
                    let _channel = caps.get(3);
                    let _text = caps.get(4);
                    if let Ok(mut write) = write.try_lock() {
                        handle_twitch_irc_command(code.as_str(), &mut write).await;
                    }
                }
            }
            // This is special for twitch when we requested
            // additional capabilities
            Some('@') => {
                info!("msg was {text}");
                if let Ok(mut write) = write.try_lock() {
                    handle_twitch_message(text, &mut write).await;
                }
            }
            _other => warn!("Unhandled msg: {:?}", text),
        }
    }
}

#[test]
fn test_regex() {
    let msg = ":tmi.twitch.tv 001 nerdachse :Welcome, GLHF!\r\n";

    let caps = IRC_REGEX.captures(&msg);
    //assert_eq!(caps, Some(x));

    let caps = caps.expect("no capture groups");

    let sender = caps.get(1);
    let code = caps.get(2);
    let channel = caps.get(3);
    let text = caps.get(4);

    assert_eq!(sender.unwrap().as_str(), ":tmi.twitch.tv");
    assert_eq!(code.unwrap().as_str(), "001");
    assert_eq!(channel.unwrap().as_str(), "nerdachse");
    assert_eq!(text.unwrap().as_str(), ":Welcome, GLHF!\r");
}

#[test]
fn websocket_to_irc_line_split() {
    let msg = ":tmi.twitch.tv 001 nerdachse :Welcome, GLHF!\r\n:tmi.twitch.tv 002 nerdachse :Your host is tmi.twitch.tv\r\n:tmi.twitch.tv 003 nerdachse :This server is rather new\r\n:tmi.twitch.tv 004 nerdachse :-\r\n:tmi.twitch.tv 375 nerdachse :-\r\n:tmi.twitch.tv 372 nerdachse :You are in a maze of twisty passages, all alike.\r\n:tmi.twitch.tv 376 nerdachse :>\r\n";

    let mut lines = msg.lines();

    assert_eq!(
        lines.next(),
        Some(":tmi.twitch.tv 001 nerdachse :Welcome, GLHF!")
    );
    assert_eq!(
        lines.next(),
        Some(":tmi.twitch.tv 002 nerdachse :Your host is tmi.twitch.tv")
    );
    assert_eq!(
        lines.next(),
        Some(":tmi.twitch.tv 003 nerdachse :This server is rather new")
    );
    assert_eq!(lines.next(), Some(":tmi.twitch.tv 004 nerdachse :-"));
    assert_eq!(lines.next(), Some(":tmi.twitch.tv 375 nerdachse :-"));
    assert_eq!(
        lines.next(),
        Some(":tmi.twitch.tv 372 nerdachse :You are in a maze of twisty passages, all alike.")
    );
    assert_eq!(lines.next(), Some(":tmi.twitch.tv 376 nerdachse :>"));
}
