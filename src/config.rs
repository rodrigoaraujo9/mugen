use tokio::time::Duration;

use crate::{audio_patch::{Generator, PatchSource}, fx::adsr::SynthSource, patches::basic::BasicKind};

//play.rs
pub const TICK: u64 = 10;

//key.rs
pub const BASE_FREQ: f32 = 440.0;
pub const A4_SEMITONES: i32 = 57;
pub const SEMITONES_PER_OCTAVE: i32 = 12;
pub const KEYBOARD_BASE_OCTAVE: i32 = 4;

//audio_source.rs
pub const AMP_DEFAULT:f32 = 0.1;

//patches
pub const SAMPLE_RATE: u32 = 48_000;
pub const ENDLESS: Duration = Duration::from_secs(3600);

// ADSR defaults
pub const ADSR_ATTACK_S: f32  = 0.5; //sec
pub const ADSR_DECAY_S: f32   = 0.5; //sec
pub const ADSR_SUSTAIN: f32   = 0.4; //0..1
pub const ADSR_RELEASE_S: f32 = 1.0; //sec

// LFO defaults
pub const LFO_KIND: BasicKind = BasicKind::Sine;
pub const LFO_RATE_HZ: f32 = 10.0;
pub const LFO_DEPTH: f32 = 1.0;
