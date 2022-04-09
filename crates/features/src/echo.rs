use async_trait::async_trait;
use dachsb0t_plugin::{Action, Event, Plugin};
use std::time::{Duration, Instant};

pub struct EchoPlugin {
    last_called: Instant,
    allowed_interval: Duration,
}

impl EchoPlugin {
    pub fn new(allowed_interval: Duration) -> Self {
        Self {
            last_called: Instant::now()
                .checked_sub(allowed_interval)
                .unwrap_or(Instant::now()),
            allowed_interval,
        }
    }
}

#[async_trait]
impl Plugin for EchoPlugin {
    type RegisterTaskOutput = ();

    fn name(&self) -> String {
        "EchoPlugin".to_owned()
    }

    async fn handle_event(&mut self, e: Event) -> Option<Action> {
        let now = Instant::now();
        match e {
            Event::ChatMessage(message) => {
                if now.duration_since(self.last_called) > self.allowed_interval {
                    self.last_called = now;
                    return Some(Action::Respond(format!("{} wrote \"{}\", how wise of them!", message.user_name, message.msg)));
                };
            }
        }
        None
    }

    fn register_async_task(&self) -> Self::RegisterTaskOutput {
        ()
    }

}
