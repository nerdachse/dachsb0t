use async_trait::async_trait;
use dachsb0t_plugin::{Action, BroadcastMsg, Event, Plugin, RegisterAction, RegisterInterval};
use std::time::{Duration, Instant};
use tokio::task::JoinHandle;

pub struct AnnouncePlugin {
    interval: Duration,
    msg: String,
}

impl AnnouncePlugin {
    pub fn new<Msg: Into<String>>(interval: Duration, msg: Msg) -> Self {
        Self {
            interval,
            msg: msg.into(),
        }
    }
}

#[async_trait]
impl Plugin for AnnouncePlugin {
    fn name(&self) -> String {
        format!("AnnouncePlugin for: {}", self.msg)
    }

    async fn handle_event(&mut self, _: Event) -> Option<Action> {
        None
    }

    fn register_action(&self) -> RegisterAction {
        RegisterAction::Interval(RegisterInterval {
            interval: self.interval.clone(),
            action: Action::Respond(self.msg.clone()),
        })
    }
}
