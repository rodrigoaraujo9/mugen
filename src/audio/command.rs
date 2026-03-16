use crate::generators::basic::Wave;
use crate::nodes::adsr::Adsr;
use crate::nodes::gain::Gain;
use crate::nodes::lfo_amp::LfoAmp;
use crate::nodes::lowpass::LowPass;

#[derive(Debug, Clone, Copy)]
pub enum Command {
    SetVolume(f32),
    SetMuted(bool),
    SetWave(Wave),
    SetAdsr(Adsr),
    SetGain(Gain),
    SetLfoAmp(LfoAmp),
    SetLowPass(LowPass),
    SetOctave(i32),
}
