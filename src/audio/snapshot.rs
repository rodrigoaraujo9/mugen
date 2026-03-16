//! Snapshot of the current audio engine state exposed to subscribers

use crate::patch::effects::adsr::Adsr;
use crate::patch::effects::gain::Gain;
use crate::patch::effects::lfo_amp::LfoAmp;
use crate::patch::effects::lowpass::LowPass;
use crate::patch::oscilators::basic::Wave;

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
