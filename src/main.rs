mod key;
mod play;
mod config;
mod state;
mod audio_source;
mod ui;
use play::run_audio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    run_audio().await
}
