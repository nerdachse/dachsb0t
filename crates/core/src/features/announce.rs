use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Mutex;
use tokio::time;
use tokio::time::sleep;

use crate::channels::websocket::WebSocketWrite;

use crate::channels::TwitchChatSender;
use tracing::error;

pub async fn announce(interval: Duration, msg: String, write: Arc<Mutex<WebSocketWrite>>) {
    let mut interval = time::interval(interval);
    loop {
        interval.tick().await;
        // Not the prettiest code I've ever written but, well...
        'retry: loop {
            sleep(Duration::from_millis(100)).await;
            if let Ok(mut write) = write.try_lock() {
                if let Err(e) = (&mut write).send_priv_msg(&msg).await {
                    error!("Couldn't announce: {e}, msg: {}", msg);
                }
                break 'retry;
            }
        }
    }
}
