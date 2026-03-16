//! Client API for sending commands to the audio engine and subscribing to state

use crate::audio::{Command, Snapshot};
use crate::effects::adsr::Adsr;
use crate::effects::gain::Gain;
use crate::effects::lfo_amp::LfoAmp;
use crate::effects::lowpass::LowPass;
use crate::oscilators::basic::Wave;
use device_query::Keycode;
use std::collections::HashSet;
use tokio::sync::{mpsc, watch};

#[derive(Clone)]
pub struct Client {
    tx: mpsc::UnboundedSender<Command>,
    snapshot_rx: watch::Receiver<Snapshot>,
    held_keys_rx: watch::Receiver<HashSet<Keycode>>,
}

impl Client {
    pub(crate) fn new(
        tx: mpsc::UnboundedSender<Command>,
        snapshot_rx: watch::Receiver<Snapshot>,
        held_keys_rx: watch::Receiver<HashSet<Keycode>>,
    ) -> Self {
        Self {
            tx,
            snapshot_rx,
            held_keys_rx,
        }
    }

    #[inline]
    fn send(&self, cmd: Command) {
        let _ = self.tx.send(cmd);
    }

    pub fn set_volume(&self, volume: f32) {
        self.send(Command::SetVolume(volume));
    }

    pub fn set_muted(&self, muted: bool) {
        self.send(Command::SetMuted(muted));
    }

    pub fn set_wave(&self, wave: Wave) {
        self.send(Command::SetWave(wave));
    }

    pub fn set_adsr(&self, adsr: Adsr) {
        self.send(Command::SetAdsr(adsr));
    }

    pub fn set_gain(&self, gain: Gain) {
        self.send(Command::SetGain(gain));
    }

    pub fn set_lfo_amp(&self, lfo_amp: LfoAmp) {
        self.send(Command::SetLfoAmp(lfo_amp));
    }

    pub fn set_lowpass(&self, lowpass: LowPass) {
        self.send(Command::SetLowPass(lowpass));
    }

    pub fn set_octave(&self, octave: i32) {
        self.send(Command::SetOctave(octave));
    }

    pub fn subscribe(&self) -> watch::Receiver<Snapshot> {
        self.snapshot_rx.clone()
    }

    pub fn subscribe_held_keys(&self) -> watch::Receiver<HashSet<Keycode>> {
        self.held_keys_rx.clone()
    }
}
