use tokio::sync::{mpsc, watch, OnceCell, Mutex};
use crate::audio_patch::AudioSource;
use crate::fx::adsr::Adsr;
use std::collections::HashSet;
use device_query::Keycode;

/// current audio state that the UI can read (volume/mute + which source is active).
#[derive(Debug, Clone)]
pub struct AudioSnapshot {
    pub volume: f32,
    pub muted: bool,
    pub patch_name: String,
}

/// cmds that the UI sends to the audio runtime to change behavior
pub enum AudioCommand {
    SetVolume(f32),
    SetMuted(bool),
    TogglePatch(Vec<Box<dyn AudioSource>>),
    SetPatch(Box<dyn AudioSource>),
    SetAdsr(Adsr),
    SetOctave(i32),
}

/// handle used by the UI: send commands + subscribe to live snapshots
#[derive(Clone)]
pub struct AudioHandle {
    tx: mpsc::UnboundedSender<AudioCommand>,
    snapshot_rx: watch::Receiver<AudioSnapshot>,
    pub held_keys_rx: watch::Receiver<HashSet<Keycode>>,
}

impl AudioHandle {
    pub fn set_volume(&self, v: f32) {
        let _ = self.tx.send(AudioCommand::SetVolume(v));
    }
    pub fn set_muted(&self, m: bool) {
        let _ = self.tx.send(AudioCommand::SetMuted(m));
    }
    pub fn toggle_patch(&self, patches: Vec<Box<dyn AudioSource>>) {
        let _ = self.tx.send(AudioCommand::TogglePatch(patches));
    }
    pub fn set_patch(&self, patch: Box<dyn AudioSource>) {
        let _ = self.tx.send(AudioCommand::SetPatch(patch));
    }
    pub fn set_adsr(&self, adsr: Adsr) {
        let _ = self.tx.send(AudioCommand::SetAdsr(adsr));
    }
    pub fn subscribe(&self) -> watch::Receiver<AudioSnapshot> {
        self.snapshot_rx.clone()
    }
    pub fn set_octave(&self, o: i32) {
        let _ = self.tx.send(AudioCommand::SetOctave(o));
    }
}

/// internal singleton state
struct AudioSystem {
    handle: AudioHandle,
    cmd_rx: Mutex<Option<mpsc::UnboundedReceiver<AudioCommand>>>,
    snapshot_tx: watch::Sender<AudioSnapshot>,
    held_keys_tx: watch::Sender<HashSet<Keycode>>,
}

static AUDIO: OnceCell<AudioSystem> = OnceCell::const_new();

pub async fn get_handle() -> &'static AudioHandle {
    &AUDIO
        .get_or_init(|| async {
            let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
            let initial = AudioSnapshot {
                volume: 1.0,
                muted: false,
                patch_name: "Sine".to_string(),
            };
            let (snapshot_tx, snapshot_rx) = watch::channel(initial);
            let (held_keys_tx, held_keys_rx) = watch::channel(HashSet::new());
            AudioSystem {
                handle: AudioHandle { tx: cmd_tx, snapshot_rx, held_keys_rx },
                cmd_rx: Mutex::new(Some(cmd_rx)),
                snapshot_tx,
                held_keys_tx,
            }
        })
        .await
        .handle
}

pub async fn take_runtime_channels() -> (
    mpsc::UnboundedReceiver<AudioCommand>,
    watch::Sender<AudioSnapshot>,
    watch::Sender<HashSet<Keycode>>,
    AudioSnapshot,
) {
    let sys = AUDIO.get_or_init(|| async { unreachable!("call get_handle() first") }).await;
    let mut guard = sys.cmd_rx.lock().await;
    let rx = guard.take().expect("audio runtime already taken");
    let initial = sys.snapshot_tx.borrow().clone();
    (rx, sys.snapshot_tx.clone(), sys.held_keys_tx.clone(), initial)
}
