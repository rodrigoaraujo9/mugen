//! Snapshot of the current audio engine state exposed to subscribers

use crate::Effects::adsr::Adsr;
use crate::Effects::gain::Gain;
use crate::Effects::lfo_amp::LfoAmp;
use crate::Effects::lowpass::LowPass;
use crate::Oscilators::basic::Wave;

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
