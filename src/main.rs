use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use synth_rs::{play::run_audio, audio_system::get_handle, ui::run_ui};
use tokio::sync::watch;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let handle = get_handle().await.clone();
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    let focused = Arc::new(AtomicBool::new(true));
    focused.store(true, Ordering::Relaxed);

    let ui = {
        let shutdown_tx = shutdown_tx.clone();
        let handle = handle.clone();
        let focused = focused.clone();

        async move {
            let res = run_ui(handle, shutdown_tx.clone(), focused).await;
            let _ = shutdown_tx.send(true);

            res
        }
    };

    let audio = run_audio(shutdown_rx, focused.clone());

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            let _ = shutdown_tx.send(true);
        }
        _ = async { tokio::join!(audio, ui) } => {}
    }

    Ok(())
}
