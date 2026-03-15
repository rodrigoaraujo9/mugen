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
    pub(crate) held_keys_rx: watch::Receiver<HashSet<Keycode>>,
}

impl AudioHandle {
    #[inline]
    fn send(&self, cmd: AudioCommand) {
        let _ = self.tx.send(cmd);
    }

    pub fn set_volume(&self, volume: f32) {
        self.send(AudioCommand::SetVolume(volume));
    }

    pub fn set_muted(&self, muted: bool) {
        self.send(AudioCommand::SetMuted(muted));
    }

    pub fn set_generator_kind(&self, kind: BasicKind) {
        self.send(AudioCommand::SetGeneratorKind(kind));
    }

    pub fn set_adsr(&self, adsr: Adsr) {
        self.send(AudioCommand::SetAdsr(adsr));
    }

    pub fn set_octave(&self, octave: i32) {
        self.send(AudioCommand::SetOctave(octave));
    }

    pub fn set_lfo(&self, params: LfoAmpParams) {
        self.send(AudioCommand::SetLfo(params));
    }

    pub fn set_lowpass(&self, params: LowPassParams) {
        self.send(AudioCommand::SetLowPass(params));
    }

    pub fn subscribe(&self) -> watch::Receiver<AudioSnapshot> {
        self.snapshot_rx.clone()
    }

    pub fn subscribe_held_keys(&self) -> watch::Receiver<HashSet<Keycode>> {
        self.held_keys_rx.clone()
    }
}
