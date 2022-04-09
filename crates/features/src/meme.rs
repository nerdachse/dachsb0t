use async_trait::async_trait;
use dachsb0t_plugin::{Action, BroadcastMsg, Event, Plugin, RegisterAction};
use std::time::{Duration, Instant};

pub struct MemePlugin {
    last_called: Instant,
    allowed_interval: Duration,
}

impl MemePlugin {
    pub fn new(allowed_interval: Duration) -> Self {
        Self {
            last_called: Instant::now()
                .checked_sub(allowed_interval)
                .unwrap_or(Instant::now()),
            allowed_interval,
        }
    }
}
const MEME_TEST_URL: &str =
    "https://ih1.redbubble.net/image.875111905.4798/flat,750x,075,f-pad,750x1000,f8f8f8.jpg";

#[async_trait]
impl Plugin for MemePlugin {
    fn name(&self) -> String {
        "MemePlugin".to_owned()
    }

    async fn handle_event(&mut self, e: Event) -> Option<Action> {
        let now = Instant::now();
        match e {
            Event::ChatMessage(message) => {
                if now.duration_since(self.last_called) > self.allowed_interval {
                    self.last_called = now;
                    if message.msg.starts_with("!meme") {
                        return Some(Action::Broadcast(BroadcastMsg::new(
                            "ShowMeme",
                            MEME_TEST_URL,
                        )));
                    }
                    return None;
                };
            }
        }
        None
    }

    fn register_action(&self) -> RegisterAction {
        RegisterAction::None
    }
}
