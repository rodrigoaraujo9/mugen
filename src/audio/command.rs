//! Commands sent to the audio engine

use crate::effects::adsr::Adsr;
use crate::effects::gain::Gain;
use crate::effects::lfo_amp::LfoAmp;
use crate::effects::lowpass::LowPass;
use crate::oscilators::basic::Wave;

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
