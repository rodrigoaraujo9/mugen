use super::types::{AudioCommand, AudioSnapshot};
use crate::generators::basic::BasicKind;
use crate::nodes::adsr::Adsr;
use crate::nodes::lfo_amp::LfoAmpParams;
use crate::nodes::lowpass::LowPassParams;
use device_query::Keycode;
use std::collections::HashSet;
use tokio::sync::{mpsc, watch};

#[derive(Clone)]
pub struct AudioHandle {
    pub(crate) tx: mpsc::UnboundedSender<AudioCommand>,
    pub(crate) snapshot_rx: watch::Receiver<AudioSnapshot>,
    pub held_keys_rx: watch::Receiver<HashSet<Keycode>>,
}

impl AudioHandle {
    pub fn set_volume(&self, v: f32) {
        let _ = self.tx.send(AudioCommand::SetVolume(v));
    }

    pub fn set_muted(&self, m: bool) {
        let _ = self.tx.send(AudioCommand::SetMuted(m));
    }

    pub fn set_generator_kind(&self, kind: BasicKind) {
        let _ = self.tx.send(AudioCommand::SetGeneratorKind(kind));
    }

    pub fn set_adsr(&self, adsr: Adsr) {
        let _ = self.tx.send(AudioCommand::SetAdsr(adsr));
    }

    pub fn set_octave(&self, o: i32) {
        let _ = self.tx.send(AudioCommand::SetOctave(o));
    }

    pub fn set_lfo(&self, lfo: LfoAmpParams) {
        let _ = self.tx.send(AudioCommand::SetLfo(lfo));
    }

    pub fn set_lowpass(&self, lowpass: LowPassParams) {
        let _ = self.tx.send(AudioCommand::SetLowPass(lowpass));
    }

    pub fn subscribe(&self) -> watch::Receiver<AudioSnapshot> {
        self.snapshot_rx.clone()
    }
}
