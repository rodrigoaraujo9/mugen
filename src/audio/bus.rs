//! Shared audio bus that owns engine command channel, published snapshots, and singleton client access

use crate::audio::{AudioClient, AudioCommand, AudioSnapshot};
use crate::config::{
    ADSR_ATTACK_S, ADSR_DECAY_S, ADSR_RELEASE_S, ADSR_SUSTAIN, CUTOFF, LFO_DEPTH, LFO_KIND,
    LFO_RATE_HZ,
};
use crate::generators::basic::Wave;
use crate::nodes::adsr::Adsr;
use crate::nodes::lfo_amp::LfoAmpParams;
use crate::nodes::lowpass::LowPassParams;
use device_query::Keycode;
use std::collections::HashSet;
use tokio::sync::{Mutex, OnceCell, mpsc, watch};

pub struct AudioBus {
    pub client: AudioClient,
    pub commands: Mutex<Option<mpsc::UnboundedReceiver<AudioCommand>>>,
    pub snapshot_tx: watch::Sender<AudioSnapshot>,
    pub held_keys_tx: watch::Sender<HashSet<Keycode>>,
}

static AUDIO: OnceCell<AudioBus> = OnceCell::const_new();

pub async fn client() -> &'static AudioClient {
    &AUDIO
        .get_or_init(|| async {
            let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();

            let snapshot = AudioSnapshot {
                volume: 1.0,
                muted: false,
                wave_kind: Wave::Sine,
                patch_name: Wave::Sine.name().to_string(),
                adsr: Adsr::new(ADSR_ATTACK_S, ADSR_DECAY_S, ADSR_SUSTAIN, ADSR_RELEASE_S),
                lfo: LfoAmpParams {
                    kind: LFO_KIND,
                    rate_hz: LFO_RATE_HZ,
                    depth: LFO_DEPTH,
                    base_gain: 1.0,
                },
                lowpass: LowPassParams { cutoff_hz: CUTOFF },
            };

            let (snapshot_tx, snapshot_rx) = watch::channel(snapshot);
            let (held_keys_tx, held_keys_rx) = watch::channel(HashSet::new());

            AudioBus {
                client: AudioClient {
                    tx: cmd_tx,
                    snapshot_rx,
                    held_keys_rx,
                },
                commands: Mutex::new(Some(cmd_rx)),
                snapshot_tx,
                held_keys_tx,
            }
        })
        .await
        .client
}

pub async fn take_engine_io() -> (
    mpsc::UnboundedReceiver<AudioCommand>,
    watch::Sender<AudioSnapshot>,
    watch::Sender<HashSet<Keycode>>,
    AudioSnapshot,
) {
    let bus = AUDIO
        .get_or_init(|| async { unreachable!("call audio::client() first") })
        .await;

    let mut commands = bus.commands.lock().await;
    let cmd_rx = commands.take().expect("audio engine already taken");
    let snapshot = bus.snapshot_tx.borrow().clone();

    (
        cmd_rx,
        bus.snapshot_tx.clone(),
        bus.held_keys_tx.clone(),
        snapshot,
    )
}
