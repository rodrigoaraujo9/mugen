//! Commands sent to the audio engine

use crate::Effects::adsr::Adsr;
use crate::Effects::gain::Gain;
use crate::Effects::lfo_amp::LfoAmp;
use crate::Effects::lowpass::LowPass;
use crate::Oscilators::basic::Wave;

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
