use std::error::Error;
use std::sync::Arc;
use std::time::Duration;
use tokio::{sync::Mutex, task};

mod channels;
mod features;
use channels::start as start_twitch;

use dachsb0t_features::AnnouncePlugin;
use dachsb0t_features::MemePlugin;
use dachsb0t_plugin::Plugin;

#[derive(Clone)]
pub struct SendablePlugin {
    pub inner: Arc<Mutex<dyn Plugin>>,
}

impl SendablePlugin {
    pub fn new<P>(plugin: P) -> Self
    where
        P: Plugin + Sized,
    {
        Self {
            inner: Arc::new(Mutex::new(plugin)),
        }
    }
}

#[derive(Clone)]
pub struct Plugins(Vec<SendablePlugin>);

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::init_timed();

    let announce_discord = SendablePlugin::new(AnnouncePlugin::new(
        Duration::from_secs(600),
        "Wusstest du schon, es gibt auch einen Discord?! Nein? Jetzt aber! https://discord.gg/Yf9MUJv3mr",
    ));
    let announce_github = SendablePlugin::new(AnnouncePlugin::new(
        Duration::from_secs(1200),
        "Interesse an den Programmierstreams? Dann schau mal auf github vorbei: https://github.com/nerdachse",
    ));
    let meme_fun = SendablePlugin::new(MemePlugin::new(Duration::from_secs(1)));
    let plugins: Vec<SendablePlugin> = vec![announce_discord, announce_github, meme_fun];
    let plugins = Plugins(plugins);

    let twitch_bot = task::spawn(start_twitch(plugins));
    let _twitch = twitch_bot.await?;

    Ok(())
}
