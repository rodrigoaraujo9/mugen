//! Internal mutable state owned by the audio engine

use crate::audio::AudioSnapshot;
use crate::generators::basic::{Wave, osc_params};
use crate::nodes::adsr::{Adsr, adsr_handle};
use crate::nodes::lfo_amp::{LfoAmpParams, lfo_amp_handle};
use crate::nodes::lowpass::{LowPassParams, lowpass_handle};
use crate::patch::Patch;
use device_query::Keycode;
use std::collections::HashSet;

pub struct AudioState {
    pub volume: f32,
    pub muted: bool,
    pub octave_offset: i32,
    pub held_keys: HashSet<Keycode>,
    pub patch: Patch,
}

impl AudioState {
    pub fn new(snapshot: AudioSnapshot) -> Self {
        let osc = osc_params(snapshot.wave_kind);
        let adsr = adsr_handle(snapshot.adsr);
        let lfo = lfo_amp_handle(snapshot.lfo);
        let lowpass = lowpass_handle(snapshot.lowpass);

        Self {
            volume: snapshot.volume,
            muted: snapshot.muted,
            octave_offset: 0,
            held_keys: HashSet::new(),
            patch: Patch::new(osc, adsr, lfo, lowpass),
        }
    }

    #[inline]
    pub fn osc_kind(&self) -> Wave {
        self.patch.osc.get().kind
    }

    #[inline]
    pub fn adsr(&self) -> Adsr {
        self.patch.adsr.get()
    }

    #[inline]
    pub fn lfo(&self) -> LfoAmpParams {
        self.patch.lfo.get()
    }

    #[inline]
    pub fn lowpass(&self) -> LowPassParams {
        self.patch.lowpass.get()
    }

    #[inline]
    pub fn snapshot(&self) -> AudioSnapshot {
        AudioSnapshot {
            volume: self.volume,
            muted: self.muted,
            wave_kind: self.osc_kind(),
            patch_name: self.patch.name(),
            adsr: self.adsr(),
            lfo: self.lfo(),
            lowpass: self.lowpass(),
        }
    }
}
