//! Snapshot of the current audio engine state exposed to subscribers

use crate::generators::basic::Wave;
use crate::nodes::adsr::Adsr;
use crate::nodes::lfo_amp::LfoAmpParams;
use crate::nodes::lowpass::LowPassParams;

#[derive(Debug, Clone)]
pub struct Snapshot {
    pub volume: f32,
    pub muted: bool,
    pub wave: Wave,
    pub octave: i32,
    pub patch_name: String,
    pub adsr: Adsr,
    pub lfo: LfoAmpParams,
    pub lowpass: LowPassParams,
}
