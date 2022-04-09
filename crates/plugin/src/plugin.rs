use crate::{Action, Event};
use async_trait::async_trait;
use std::time::Duration;

#[async_trait]
pub trait Plugin
where
    Self: 'static + Send + Sync,
{
    fn name(&self) -> String;
    async fn handle_event(&mut self, e: Event) -> Option<Action>;
    fn register_action(&self) -> RegisterAction;
}

pub enum RegisterAction {
    None,
    Interval(RegisterInterval),
}

pub struct RegisterInterval {
    pub action: Action,
    pub interval: Duration,
}
