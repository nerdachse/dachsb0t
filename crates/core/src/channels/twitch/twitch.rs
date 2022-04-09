use std::env;
use std::error::Error;

use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::{
    connect_async, tungstenite::protocol::Message, tungstenite::Error as TError,
};

use tracing::{error, info, warn};
use url::Url;

use async_trait::async_trait;
use once_cell::sync::Lazy;
use regex::Regex;

use tokio::sync::Mutex;
use tokio::task;

use std::sync::Arc;

use crate::channels::websocket::{WebSocketRead, WebSocketStream, WebSocketWrite};
use crate::features::announce;

use crate::channels::distributor::NerdHubRaumMsg;
use crate::Plugins;

use crate::channels::distributor::NERDHUBRAUM_WEBSOCKET_PATH;
use dachsb0t_plugin::{Action, ChatMessage, Event};

const TWITCH_CB_TOKEN: &'static str = "twitch_chat_bot_auth_token";
const TWITCH_CHAT_URL: &'static str = "wss://irc-ws.chat.twitch.tv:443";
const TWITCH_CHANNEL: &'static str = "nerdachse";
const TWITCH_BOT_NAME: &'static str = "dachsb0t";

static IRC_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"([\S]*) ([\S]*) ([\S]*) (.*)").expect("failed to compile regex"));

/*
async fn connect_to_nerdhubraum() -> WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>> {
    let (ws_stream, _) = connect_async(NERDHUBRAUM_WEBSOCKET_PATH)
        .await
        .expect("Failed to connect to nerdhubraum");
    ws_stream
}
*/

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

pub struct TwitchChatBuilder {
    ws_stream: WebSocketStream,
}

impl TwitchChatBuilder {
    pub fn new(ws_stream: WebSocketStream) -> Self {
        Self { ws_stream }
    }

    pub async fn auth(mut self, token: &str) -> Result<TwitchChat, Box<dyn Error>> {
        self.ws_stream
            .send(Message::Text(format!("PASS oauth:{token}")))
            .await?;
        self.ws_stream
            .send(Message::Text(format!("NICK {TWITCH_BOT_NAME}")))
            .await?;

        Ok(TwitchChat::new(self.ws_stream))
    }
}

pub struct TwitchChat {
    read: WebSocketRead,
    write: Arc<Mutex<WebSocketWrite>>,
}

impl TwitchChat {
    fn new(ws_stream: WebSocketStream) -> Self {
        let (write, read) = ws_stream.split();
        Self {
            read,
            write: Arc::new(Mutex::new(write)),
        }
    }
}

pub async fn start(plugins: Plugins) -> () {
    let token = env::var(TWITCH_CB_TOKEN).expect("No twitch_chat_bot_auth_token provided in env");
    let url = Url::parse(TWITCH_CHAT_URL).expect("Couldn't parse TWITCH_CHAT_URL");
    let (ws_stream, _) = connect_async(url)
        .await
        .expect("Failed to connect to TWITCH_CHAT");

    let twitch_chat = TwitchChatBuilder::new(ws_stream);
    let mut twitch_chat = twitch_chat
        .auth(&token)
        .await
        .expect("Failed to authenticate with twitch_chat");

    // Nerdhubraum
    let (ws_stream, _) = connect_async(NERDHUBRAUM_WEBSOCKET_PATH)
        .await
        .expect("Failed to connect to NERDHUBRAUM");
    let (nhr_write, _) = ws_stream.split();
    let nhr_write = Arc::new(Mutex::new(nhr_write));

    plugins.0.iter().for_each(|plugin| {
        if let Ok(plugin) = plugin.inner.try_lock() {
            info!("attempting to register plugin: {}", plugin.name());
            match plugin.register_action() {
                dachsb0t_plugin::RegisterAction::None => {
                    info!("plugin: {} registered no action", plugin.name());
                }
                dachsb0t_plugin::RegisterAction::Interval(i) => match i.action {
                    Action::Respond(msg) => {
                        let _task =
                            task::spawn(announce(i.interval, msg, twitch_chat.write.clone()));
                    }
                    Action::Broadcast(_) => todo!(),
                },
            }
        }
    });

    let handle_messages =
        task::spawn(async move { handle_requests(&mut twitch_chat, nhr_write, plugins).await });
    let _ = handle_messages.await;
    // Note that we never await the registered_actions
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

async fn handle_twitch_message(
    msg: &str,
    write: Arc<Mutex<WebSocketWrite>>,
    nhr_write: Arc<Mutex<WebSocketWrite>>,
    plugins: Plugins,
) {
    let text = msg.split(":").last();
    let user_name = msg.split("display-name=").last();
    let user_name = user_name.unwrap_or("").split(";").next();

    plugins.0.into_iter().for_each(|plugin| {
        let cloned_write = write.clone();
        let cloned_nhr_write = nhr_write.clone();
        let text = text.unwrap_or("").to_owned();
        let user_name = user_name.unwrap_or("").to_owned();
        tokio::spawn(async move {
            if let Ok(mut plugin) = plugin.inner.try_lock() {
                let action = plugin
                    .handle_event(Event::ChatMessage(ChatMessage {
                        user_name,
                        msg: text,
                    }))
                    .await;
                if let Some(action) = action {
                    info!(
                        "plugin: {} responded with action: {:?}",
                        plugin.name(),
                        action
                    );
                    match action {
                        Action::Respond(msg) => {
                            if let Ok(mut write) = cloned_write.try_lock() {
                                if let Err(e) = write.send_priv_msg(&msg).await {
                                    error!("Error sending message to twitch: {e}");
                                }
                            }
                        }
                        Action::Broadcast(msg) => {
                            if let Ok(mut write) = cloned_nhr_write.try_lock() {
                                let msg = NerdHubRaumMsg::new(msg.r#type, msg.url);
                                match serde_json::to_string(&msg) {
                                    Ok(msg) => {
                                        let msg = Message::Text(msg);
                                        if let Err(e) = write.send(msg).await {
                                            error!("Error sending message to nerdhubraum: {e}");
                                        }
                                    }
                                    Err(e) => error!("Error sending message to nerdhubraum: {e}"),
                                }
                            }
                        }
                    }
                }
            }
        });
    })
}

async fn handle_requests(
    chat: &mut TwitchChat,
    nhr_write: Arc<Mutex<WebSocketWrite>>,
    plugins: Plugins,
) -> () {
    while let Some(msg) = chat.read.next().await {
        match msg {
            Ok(msg) => {
                if msg.is_text() {
                    if let Ok(text) = msg.into_text() {
                        handle_text_msg(
                            text,
                            chat.write.clone(),
                            nhr_write.clone(),
                            plugins.clone(),
                        )
                        .await;
                    }
                } else if msg.is_ping() {
                    info!("I got a websocket ping, OMG!");
                } else {
                    warn!("Unhandled msg: {msg}");
                }
            }
            Err(e) => warn!("Failed to get msg: {e}"),
        }
    }
}

async fn handle_text_msg(
    text: String,
    write: Arc<Mutex<WebSocketWrite>>,
    nhr_write: Arc<Mutex<WebSocketWrite>>,
    plugins: Plugins,
) {
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
                handle_twitch_message(text, write.clone(), nhr_write.clone(), plugins.clone())
                    .await;
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
