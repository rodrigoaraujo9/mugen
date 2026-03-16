//! Internal mutable state owned by the audio engine

use crate::audio::Snapshot;
use crate::generators::basic::{Wave, osc_params};
use crate::nodes::adsr::{Adsr, adsr_handle};
use crate::nodes::lfo_amp::{LfoAmpParams, lfo_amp_handle};
use crate::nodes::lowpass::{LowPassParams, lowpass_handle};
use crate::patch::Patch;
use device_query::Keycode;
use std::collections::HashSet;

pub struct State {
    pub volume: f32,
    pub muted: bool,
    pub octave: i32,
    pub held_keys: HashSet<Keycode>,
    pub patch: Patch,
}

impl State {
    pub fn from_snapshot(snapshot: Snapshot) -> Self {
        let osc = osc_params(snapshot.wave);
        let adsr = adsr_handle(snapshot.adsr);
        let lfo = lfo_amp_handle(snapshot.lfo);
        let lowpass = lowpass_handle(snapshot.lowpass);

        Self {
            volume: snapshot.volume,
            muted: snapshot.muted,
            octave: 0,
            held_keys: HashSet::new(),
            patch: Patch::new(osc, adsr, lfo, lowpass),
        }
    }

    #[inline]
    pub fn wave(&self) -> Wave {
        self.patch.wave()
    }

    #[inline]
    pub fn adsr(&self) -> Adsr {
        self.patch.adsr()
    }

    #[inline]
    pub fn lfo(&self) -> LfoAmpParams {
        self.patch.lfo()
    }

    #[inline]
    pub fn lowpass(&self) -> LowPassParams {
        self.patch.lowpass()
    }

    #[inline]
    pub fn set_wave(&self, wave: Wave) {
        self.patch.set_wave(wave);
    }

    #[inline]
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            volume: self.volume,
            muted: self.muted,
            wave: self.wave(),
            patch_name: self.patch.name(),
            adsr: self.adsr(),
            lfo: self.lfo(),
            lowpass: self.lowpass(),
            octave: self.octave,
        }
    }
}
