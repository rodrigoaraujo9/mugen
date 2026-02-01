mod key;
mod play;
mod config;
mod state;
mod audio_source;
mod audio_capture;
mod display;
mod visualizer;

use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::time::Duration;
use tokio::sync::Notify;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;

    let _ = std::thread::spawn(|| { // audio handle
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            if let Err(e) = play::run_audio().await {
                eprintln!("Audio error: {:?}", e);
            }
        });
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut visualizer = visualizer::VisualizerApp::new();

    let mut quit = false;
    while !quit {
        let audio_data = if let Some(capture) = state::get_audio_capture().await {
            capture.get_data()
        } else {
            None
        };

        if let Err(e) = visualizer.draw(&mut terminal, audio_data) {
            eprintln!("Draw error: {:?}", e);
            break;
        }

        if let Ok(should_quit) = visualizer.handle_events() {
            quit = should_quit;
        }

        tokio::time::sleep(Duration::from_millis(16)).await; // ~60fps
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
