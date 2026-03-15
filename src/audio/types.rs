use crate::generators::basic::BasicKind;
use crate::nodes::adsr::Adsr;
use crate::nodes::lfo_amp::LfoAmpParams;
use crate::nodes::lowpass::LowPassParams;

#[derive(Debug, Clone)]
pub struct AudioSnapshot {
    pub volume: f32,
    pub muted: bool,
    pub generator_kind: BasicKind,
    pub patch_name: String,
    pub adsr: Adsr,
    pub lfo: LfoAmpParams,
    pub lowpass: LowPassParams,
}

pub enum AudioCommand {
    SetVolume(f32),
    SetMuted(bool),
    SetGeneratorKind(BasicKind),
    SetAdsr(Adsr),
    SetOctave(i32),
    SetLfo(LfoAmpParams),
    SetLowPass(LowPassParams),
}
