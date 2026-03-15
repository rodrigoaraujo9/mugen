use super::handle::AudioHandle;
use super::types::{AudioCommand, AudioSnapshot};
use crate::config::{
    ADSR_ATTACK_S, ADSR_DECAY_S, ADSR_RELEASE_S, ADSR_SUSTAIN, CUTOFF, LFO_DEPTH, LFO_KIND,
    LFO_RATE_HZ,
};
use crate::generators::basic::BasicKind;
use crate::nodes::adsr::Adsr;
use crate::nodes::lfo_amp::LfoAmp;
use crate::nodes::lowpass::LowPass;
use device_query::Keycode;
use std::collections::HashSet;
use tokio::sync::{Mutex, OnceCell, mpsc, watch};

pub struct AudioSystem {
    pub handle: AudioHandle,
    pub cmd_rx: Mutex<Option<mpsc::UnboundedReceiver<AudioCommand>>>,
    pub snapshot_tx: watch::Sender<AudioSnapshot>,
    pub held_keys_tx: watch::Sender<HashSet<Keycode>>,
}

pub static AUDIO: OnceCell<AudioSystem> = OnceCell::const_new();

pub async fn get_handle() -> &'static AudioHandle {
    &AUDIO
        .get_or_init(|| async {
            let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();

            let adsr = Adsr::new(ADSR_ATTACK_S, ADSR_DECAY_S, ADSR_SUSTAIN, ADSR_RELEASE_S);
            let lfo = LfoAmp::new(LFO_KIND, LFO_RATE_HZ, LFO_DEPTH);
            let lowpass = LowPass::new(CUTOFF);

            let snapshot = AudioSnapshot {
                volume: 1.0,
                muted: false,
                generator_kind: BasicKind::Sine,
                patch_name: BasicKind::Sine.name().to_string(),
                adsr,
                lfo: lfo.params(),
                lowpass: lowpass.params(),
            };

            let (snapshot_tx, snapshot_rx) = watch::channel(snapshot);
            let (held_keys_tx, held_keys_rx) = watch::channel(HashSet::new());

            AudioSystem {
                handle: AudioHandle {
                    tx: cmd_tx,
                    snapshot_rx,
                    held_keys_rx,
                },
                cmd_rx: Mutex::new(Some(cmd_rx)),
                snapshot_tx,
                held_keys_tx,
            }
        })
        .await
        .handle
}

pub async fn take_runtime_io() -> (
    mpsc::UnboundedReceiver<AudioCommand>,
    watch::Sender<AudioSnapshot>,
    watch::Sender<HashSet<Keycode>>,
    AudioSnapshot,
) {
    let sys = AUDIO
        .get_or_init(|| async { unreachable!("call get_handle() first") })
        .await;

    let mut guard = sys.cmd_rx.lock().await;
    let cmd_rx = guard.take().expect("audio runtime already taken");
    let snapshot = sys.snapshot_tx.borrow().clone();

    (
        cmd_rx,
        sys.snapshot_tx.clone(),
        sys.held_keys_tx.clone(),
        snapshot,
    )
}
