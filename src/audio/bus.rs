//! Shared audio bus that owns the engine channels and singleton client access

use crate::Effects::adsr::Adsr;
use crate::Effects::gain::Gain;
use crate::Effects::lfo_amp::LfoAmp;
use crate::Effects::lowpass::LowPass;
use crate::Oscilators::basic::Wave;
use crate::audio::{Client, Command, Snapshot};
use crate::config::{
    ADSR_ATTACK_S, ADSR_DECAY_S, ADSR_RELEASE_S, ADSR_SUSTAIN, CUTOFF, LFO_DEPTH, LFO_KIND,
    LFO_RATE_HZ,
};
use device_query::Keycode;
use std::collections::HashSet;
use tokio::sync::{Mutex, OnceCell, mpsc, watch};

pub struct Bus {
    client: Client,
    commands: Mutex<Option<mpsc::UnboundedReceiver<Command>>>,
    snapshot_tx: watch::Sender<Snapshot>,
    held_keys_tx: watch::Sender<HashSet<Keycode>>,
}

static AUDIO: OnceCell<Bus> = OnceCell::const_new();

fn initial_snapshot() -> Snapshot {
    Snapshot {
        volume: 1.0,
        muted: false,
        wave: Wave::Sine,
        octave: 0,
        patch_name: Wave::Sine.name().to_string(),
        adsr: Adsr::new(ADSR_ATTACK_S, ADSR_DECAY_S, ADSR_SUSTAIN, ADSR_RELEASE_S),
        gain: Gain { amount: 1.0 },
        lfo_amp: LfoAmp {
            wave: LFO_KIND,
            rate_hz: LFO_RATE_HZ,
            depth: LFO_DEPTH,
            base_gain: 1.0,
        },
        lowpass: LowPass { cutoff_hz: CUTOFF },
    }
}

pub async fn client() -> &'static Client {
    &AUDIO
        .get_or_init(|| async {
            let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();

            let snapshot = initial_snapshot();
            let (snapshot_tx, snapshot_rx) = watch::channel(snapshot);
            let (held_keys_tx, held_keys_rx) = watch::channel(HashSet::<Keycode>::new());

            Bus {
                client: Client::new(cmd_tx, snapshot_rx, held_keys_rx),
                commands: Mutex::new(Some(cmd_rx)),
                snapshot_tx,
                held_keys_tx,
            }
        })
        .await
        .client
}

pub async fn take_engine_channels() -> (
    mpsc::UnboundedReceiver<Command>,
    watch::Sender<Snapshot>,
    watch::Sender<HashSet<Keycode>>,
    Snapshot,
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
