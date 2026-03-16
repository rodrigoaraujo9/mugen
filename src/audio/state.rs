//! Stores live engine parameters and patch handles

use crate::audio::Snapshot;
use crate::patch::effects::adsr::{Adsr, AdsrHandle, make_adsr};
use crate::patch::effects::gain::{Gain, GainHandle, make_gain};
use crate::patch::effects::lfo_amp::{LfoAmp, LfoAmpHandle, make_lfo_amp};
use crate::patch::effects::lowpass::{LowPass, LowPassHandle, make_lowpass};
use crate::patch::oscilators::basic::{OscHandle, Wave, make_osc};
use crate::patch::{Patch, SharedEffect};
use device_query::Keycode;
use std::collections::HashSet;
use std::sync::Arc;

pub struct State {
    pub volume: f32,
    pub muted: bool,
    pub octave: i32,
    pub held_keys: HashSet<Keycode>,

    pub osc: OscHandle,
    pub adsr: AdsrHandle,
    pub gain: GainHandle,
    pub lfo_amp: LfoAmpHandle,
    pub lowpass: LowPassHandle,

    pub patch: Patch,
}

impl State {
    pub fn from_snapshot(snapshot: Snapshot) -> Self {
        let osc = make_osc(snapshot.wave);
        let adsr = make_adsr(snapshot.adsr);
        let gain = make_gain(snapshot.gain.amount);
        let lfo_amp = make_lfo_amp(snapshot.lfo_amp);
        let lowpass = make_lowpass(snapshot.lowpass);

        let effects: Vec<SharedEffect> = vec![
            Arc::new(gain.clone()),
            Arc::new(lfo_amp.clone()),
            Arc::new(lowpass.clone()),
        ];

        let patch = Patch::new(osc.clone(), adsr.clone(), effects);

        Self {
            volume: snapshot.volume,
            muted: snapshot.muted,
            octave: snapshot.octave,
            held_keys: HashSet::new(),
            osc,
            adsr,
            gain,
            lfo_amp,
            lowpass,
            patch,
        }
    }

    #[inline]
    pub fn wave(&self) -> Wave {
        self.osc.get().wave
    }

    #[inline]
    pub fn set_wave(&self, wave: Wave) {
        self.osc.update(|osc| osc.wave = wave);
    }

    #[inline]
    pub fn toggle_wave(&self) {
        self.osc.update(|osc| osc.wave = osc.wave.toggle());
    }

    #[inline]
    pub fn adsr(&self) -> Adsr {
        self.adsr.get()
    }

    #[inline]
    pub fn set_adsr(&self, adsr: Adsr) {
        self.adsr.set(adsr);
    }

    #[inline]
    pub fn gain(&self) -> Gain {
        self.gain.get()
    }

    #[inline]
    pub fn set_gain(&self, gain: Gain) {
        self.gain.set(gain);
    }

    #[inline]
    pub fn lfo_amp(&self) -> LfoAmp {
        self.lfo_amp.get()
    }

    #[inline]
    pub fn set_lfo_amp(&self, lfo_amp: LfoAmp) {
        self.lfo_amp.set(lfo_amp);
    }

    #[inline]
    pub fn lowpass(&self) -> LowPass {
        self.lowpass.get()
    }

    #[inline]
    pub fn set_lowpass(&self, lowpass: LowPass) {
        self.lowpass.set(lowpass);
    }

    #[inline]
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            volume: self.volume,
            muted: self.muted,
            wave: self.wave(),
            octave: self.octave,
            patch_name: self.patch.name(),
            adsr: self.adsr(),
            gain: self.gain(),
            lfo_amp: self.lfo_amp(),
            lowpass: self.lowpass(),
        }
    }
}
