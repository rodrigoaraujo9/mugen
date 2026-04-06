//! Snapshot of the current audio engine state exposed to subscribers

use crate::config::{
    ADSR_ATTACK_S, ADSR_DECAY_S, ADSR_RELEASE_S, ADSR_SUSTAIN, CUTOFF, LFO_DEPTH, LFO_KIND,
    LFO_RATE_HZ,
};
use crate::patch::effects::adsr::Adsr;
use crate::patch::effects::gain::Gain;
use crate::patch::effects::lfo_amp::LfoAmp;
use crate::patch::effects::lowpass::LowPass;
use crate::patch::oscilators::basic::Wave;
use crate::presets::Preset;

#[derive(Debug, Clone)]
pub struct Snapshot {
    pub volume: f32,
    pub muted: bool,
    pub wave: Wave,
    pub octave: i32,
    pub patch_name: String,
    pub adsr: Adsr,
    pub gain: Gain,
    pub lfo_amp: LfoAmp,
    pub lowpass: LowPass,
}

impl Snapshot {
    #[must_use]
    pub fn default() -> Self {
        Self {
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
    pub fn from_preset(preset: Preset) -> Self {
        Self {
            volume: 1.0,
            muted: false,
            wave: preset.wave,
            octave: preset.octave_shift,
            patch_name: preset.name,
            adsr: Adsr::new(preset.attack, preset.decay, preset.sustain, preset.release),
            gain: Gain { amount: 1.0 },
            lfo_amp: LfoAmp {
                wave: preset.lfo_wave,
                rate_hz: preset.lfo_rate,
                depth: preset.lfo_depth,
                base_gain: 1.0,
            },
            lowpass: LowPass {
                cutoff_hz: preset.cutoff,
            },
        }
    }
}
