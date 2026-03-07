use std::collections::HashSet;
use device_query::Keycode;
use tokio::sync::{mpsc, watch};
use super::types::{AudioCommand, AudioSnapshot};
use crate::patch::Generator;
use crate::nodes::adsr::Adsr;
use crate::nodes::lfo_amp::LfoAmp;
use crate::nodes::lowpass::LowPass;

/// handle used by the UI to send commands and subscribe to live snapshots
#[derive(Clone)]
pub struct AudioHandle {
    pub(crate) tx: mpsc::UnboundedSender<AudioCommand>,
    pub(crate) snapshot_rx: watch::Receiver<AudioSnapshot>,
    pub held_keys_rx: watch::Receiver<HashSet<Keycode>>,
}

impl AudioHandle {
    pub fn set_volume(&self, v: f32) { let _ = self.tx.send(AudioCommand::SetVolume(v)); }
    pub fn set_muted(&self, m: bool) { let _ = self.tx.send(AudioCommand::SetMuted(m)); }
    pub fn toggle_patch(&self, patches: Vec<Box<dyn Generator>>) { let _ = self.tx.send(AudioCommand::TogglePatch(patches)); }
    pub fn set_patch(&self, patch: Box<dyn Generator>) { let _ = self.tx.send(AudioCommand::SetPatch(patch)); }
    pub fn set_adsr(&self, adsr: Adsr) { let _ = self.tx.send(AudioCommand::SetAdsr(adsr)); }
    pub fn set_octave(&self, o: i32) { let _ = self.tx.send(AudioCommand::SetOctave(o)); }
    pub fn set_lfoamp(&self, lfo: LfoAmp) { let _ = self.tx.send(AudioCommand::SetLFOAmp(lfo)); }
    pub fn set_lowpass(&self, lowpass: LowPass) {let _ = self.tx.send(AudioCommand::SetLowPass(lowpass));}
    pub fn subscribe(&self) -> watch::Receiver<AudioSnapshot> { self.snapshot_rx.clone() }
}
