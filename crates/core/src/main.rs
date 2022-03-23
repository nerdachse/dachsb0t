use std::error::Error;
use std::sync::Arc;
use std::time::Duration;
use tokio::task;

mod channels;
mod features;
use channels::start as start_twitch;

use dachsb0t_features::EchoPlugin;
use dachsb0t_plugin::Plugin;

//#[derive(Clone)]
//pub struct Plugins(Arc<tokio::sync::RwLock<Vec<Box<dyn Plugin>>>>);

#[derive(Clone)]
pub struct Plugins(Vec<Arc<tokio::sync::Mutex<dyn Plugin>>>);

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::init_timed();

    let plugins = Plugins(vec![Arc::new(tokio::sync::Mutex::new(EchoPlugin::new(
        Duration::from_secs(5),
    )))]);

    let twitch_bot = task::spawn(start_twitch(plugins));
    let _twitch = twitch_bot.await?;

    Ok(())
}
