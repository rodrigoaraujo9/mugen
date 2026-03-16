use crate::generators::basic::Wave;
use crate::nodes::adsr::Adsr;
use crate::nodes::lfo_amp::LfoAmpParams;
use crate::nodes::lowpass::LowPassParams;

#[derive(Debug, Clone)]
pub struct AudioSnapshot {
    pub volume: f32,
    pub muted: bool,
    pub wave_kind: Wave,
    pub patch_name: String,
    pub adsr: Adsr,
    pub lfo: LfoAmpParams,
    pub lowpass: LowPassParams,
}

pub enum AudioCommand {
    SetVolume(f32),
    SetMuted(bool),
    SetGeneratorKind(Wave),
    SetAdsr(Adsr),
    SetOctave(i32),
    SetLfo(LfoAmpParams),
    SetLowPass(LowPassParams),
}
