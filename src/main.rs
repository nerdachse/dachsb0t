use std::error::Error;
use tokio::task;

mod features;
mod channels;
use channels::start as start_twitch;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::init_timed();

    let twitch_bot = task::spawn(start_twitch());
    let _twitch = twitch_bot.await?;

    Ok(())
}
