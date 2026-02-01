mod key;
mod play;
mod config;
mod state;
mod audio_source;
use play::run;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    run().await
}
