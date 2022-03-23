use crate::{Action, Event};
use async_trait::async_trait;

#[async_trait]
pub trait Plugin
where
    Self: 'static + Send + Sync,
{
    async fn handle_event(&mut self, e: Event) -> Option<Action>;
}
