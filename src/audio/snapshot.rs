use crate::generators::basic::Wave;
use crate::nodes::adsr::Adsr;
use crate::nodes::gain::Gain;
use crate::nodes::lfo_amp::LfoAmp;
use crate::nodes::lowpass::LowPass;

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
