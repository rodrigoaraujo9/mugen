use crate::nodes::adsr::Adsr;
use crate::nodes::lfo_amp::LfoAmp;
use crate::nodes::lowpass::LowPass;
use crate::patch::Generator;

/// current audio state that the UI can read (volume/mute + which source is active)
#[derive(Debug, Clone)]
pub struct AudioSnapshot {
    pub volume: f32,
    pub muted: bool,
    pub patch_name: String,
    pub adsr: Adsr,
    pub lfo: LfoAmp,
    pub lowpass: LowPass,
}

/// cmds that the UI sends to the audio runtime to change behavior
pub enum AudioCommand {
    SetVolume(f32),
    SetMuted(bool),
    TogglePatch(Vec<Box<dyn Generator>>),
    SetPatch(Box<dyn Generator>),
    SetAdsr(Adsr),
    SetOctave(i32),
    SetLFOAmp(LfoAmp),
    SetLowPass(LowPass),
}
