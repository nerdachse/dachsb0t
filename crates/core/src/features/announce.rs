use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Mutex;
use tokio::time;

use crate::channels::websocket::WebSocketWrite;

use crate::channels::TwitchChatSender;
use tracing::error;

pub async fn announce(interval: Duration, msg: &str, write: Arc<Mutex<WebSocketWrite>>) {
    let mut interval = time::interval(interval);
    loop {
        interval.tick().await;
        if let Ok(mut write) = write.try_lock() {
            if let Err(_) = (&mut write).send_priv_msg(msg).await {
                error!("Couldn't announce: {}", msg);
            }
        }
    }
}
