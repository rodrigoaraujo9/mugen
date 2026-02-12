use tokio::sync::{mpsc, watch, OnceCell, Mutex};

use crate::patches::basic::BasicKind;

#[derive(Debug, Clone, Copy)]
pub struct AudioSnapshot {
    pub volume: f32,
    pub muted: bool,
    pub kind: BasicKind,
}

#[derive(Debug)]
pub enum AudioCommand {
    SetVolume(f32),
    SetMuted(bool),
    ToggleSource,
    SetSource(BasicKind),
}

#[derive(Clone)]
pub struct AudioHandle {
    tx: mpsc::UnboundedSender<AudioCommand>,
    snapshot_rx: watch::Receiver<AudioSnapshot>,
}

impl AudioHandle {
    pub fn set_volume(&self, v: f32) {
        let _ = self.tx.send(AudioCommand::SetVolume(v));
    }

    pub fn set_muted(&self, m: bool) {
        let _ = self.tx.send(AudioCommand::SetMuted(m));
    }

    pub fn rotate_source(&self) {
        let _ = self.tx.send(AudioCommand::ToggleSource);
    }

    pub fn set_source(&self, kind: BasicKind) {
        let _ = self.tx.send(AudioCommand::SetSource(kind));
    }

    pub fn subscribe(&self) -> watch::Receiver<AudioSnapshot> {
        self.snapshot_rx.clone()
    }
}

struct AudioSystem {
    handle: AudioHandle,
    cmd_rx: Mutex<Option<mpsc::UnboundedReceiver<AudioCommand>>>,
    snapshot_tx: watch::Sender<AudioSnapshot>,
}

static AUDIO: OnceCell<AudioSystem> = OnceCell::const_new();

pub async fn get_handle() -> &'static AudioHandle {
    &AUDIO
        .get_or_init(|| async {
            let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();

            let initial = AudioSnapshot {
                volume: 1.0,
                muted: false,
                kind: BasicKind::Sine,
            };

            let (snapshot_tx, snapshot_rx) = watch::channel(initial);

            AudioSystem {
                handle: AudioHandle { tx: cmd_tx, snapshot_rx },
                cmd_rx: Mutex::new(Some(cmd_rx)),
                snapshot_tx,
            }
        })
        .await
        .handle
}

pub async fn take_runtime_channels(
) -> (mpsc::UnboundedReceiver<AudioCommand>, watch::Sender<AudioSnapshot>, AudioSnapshot) {
    let sys = AUDIO.get_or_init(|| async { unreachable!("call get_handle() first") }).await;

    let mut guard = sys.cmd_rx.lock().await;
    let rx = guard.take().expect("Audio runtime already taken");

    let initial = *sys.snapshot_tx.borrow();
    (rx, sys.snapshot_tx.clone(), initial)
}
