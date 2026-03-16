//! Commands sent to the audio engine

use crate::generators::basic::Wave;
use crate::nodes::adsr::Adsr;
use crate::nodes::lfo_amp::LfoAmpParams;
use crate::nodes::lowpass::LowPassParams;

#[derive(Debug, Clone, Copy)]
pub enum Command {
    SetVolume(f32),
    SetMuted(bool),
    SetWave(Wave),
    SetAdsr(Adsr),
    SetLfo(LfoAmpParams),
    SetLowPass(LowPassParams),
    SetOctave(i32),
}
