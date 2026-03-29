//! Commands sent to the audio engine

use crate::patch::effects::adsr::Adsr;
use crate::patch::effects::gain::Gain;
use crate::patch::effects::lfo_amp::LfoAmp;
use crate::patch::effects::lowpass::LowPass;
use crate::patch::oscilators::basic::Wave;

#[derive(Debug, Clone)]
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
