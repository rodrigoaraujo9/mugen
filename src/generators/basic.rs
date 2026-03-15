use crate::config::{AMP_DEFAULT, SAMPLE_RATE};
use crate::patch::{Generator, SynthSource};
use rodio::Source;
use std::f32::consts::TAU;
use std::sync::{Arc, RwLock};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BasicKind {
    Sine,
    Saw,
    Square,
    Triangle,
    Noise,
}

impl BasicKind {
    #[inline]
    pub fn toggle(self) -> Self {
        match self {
            Self::Sine => Self::Saw,
            Self::Saw => Self::Square,
            Self::Square => Self::Triangle,
            Self::Triangle => Self::Noise,
            Self::Noise => Self::Sine,
        }
    }

    #[inline]
    pub fn name(self) -> &'static str {
        match self {
            Self::Sine => "Sine",
            Self::Saw => "Saw",
            Self::Square => "Square",
            Self::Triangle => "Triangle",
            Self::Noise => "Noise",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct BasicGeneratorParams {
    pub kind: BasicKind,
    pub amplitude: f32,
    pub sample_rate: u32,
}

impl Default for BasicGeneratorParams {
    fn default() -> Self {
        Self {
            kind: BasicKind::Sine,
            amplitude: AMP_DEFAULT,
            sample_rate: SAMPLE_RATE,
        }
    }
}

type SharedParams = Arc<RwLock<BasicGeneratorParams>>;

pub fn basic_generator(kind: BasicKind) -> Arc<BasicGenerator> {
    Arc::new(BasicGenerator::new(kind))
}

pub struct BasicGenerator {
    params: SharedParams,
}

impl BasicGenerator {
    pub fn new(kind: BasicKind) -> Self {
        Self::from_params(BasicGeneratorParams {
            kind,
            ..BasicGeneratorParams::default()
        })
    }

    pub fn from_params(params: BasicGeneratorParams) -> Self {
        Self {
            params: Arc::new(RwLock::new(BasicGeneratorParams {
                kind: params.kind,
                amplitude: params.amplitude.max(0.0),
                sample_rate: params.sample_rate.max(1),
            })),
        }
    }

    #[inline]
    pub fn params(&self) -> BasicGeneratorParams {
        *self.params.read().unwrap()
    }

    pub fn set_kind(&self, kind: BasicKind) {
        self.params.write().unwrap().kind = kind;
    }

    pub fn set_amplitude(&self, amplitude: f32) {
        self.params.write().unwrap().amplitude = amplitude.max(0.0);
    }

    pub fn set_sample_rate(&self, sample_rate: u32) {
        self.params.write().unwrap().sample_rate = sample_rate.max(1);
    }
}

impl Generator for BasicGenerator {
    fn create(&self, frequency: f32) -> SynthSource {
        Box::new(BasicSource::new(self.params.clone(), frequency))
    }

    fn name(&self) -> &'static str {
        self.params().kind.name()
    }
}

struct BasicSource {
    params: SharedParams,
    frequency: f32,
    phase: f32,
    rng: u64,
}

impl BasicSource {
    fn new(params: SharedParams, frequency: f32) -> Self {
        Self {
            params,
            frequency: frequency.max(0.0),
            phase: 0.0,
            rng: 0x1234_5678_9ABC_DEF0,
        }
    }

    fn live_sample_rate(&self) -> u32 {
        self.params.read().unwrap().sample_rate.max(1)
    }

    fn step_phase(&mut self) -> f32 {
        let phase = self.phase;
        self.phase += self.frequency / self.live_sample_rate() as f32;

        if self.phase >= 1.0 {
            self.phase -= self.phase.floor();
        }

        phase
    }

    fn next_noise(&mut self) -> f32 {
        let mut x = self.rng;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.rng = x;

        let y = x.wrapping_mul(0x2545_F491_4F6C_DD1D);
        let u = (y >> 40) as u32;
        let f = u as f32 / ((1u32 << 24) as f32);

        2.0 * f - 1.0
    }
}

impl Iterator for BasicSource {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        let params = *self.params.read().unwrap();
        let amp = params.amplitude.max(0.0);

        let y = match params.kind {
            BasicKind::Sine => (TAU * self.step_phase()).sin(),
            BasicKind::Square => {
                if self.step_phase() < 0.5 {
                    1.0
                } else {
                    -1.0
                }
            }
            BasicKind::Triangle => {
                let p = self.step_phase();
                if p < 0.5 {
                    -1.0 + 4.0 * p
                } else {
                    3.0 - 4.0 * p
                }
            }
            BasicKind::Saw => 2.0 * self.step_phase() - 1.0,
            BasicKind::Noise => self.next_noise(),
        };

        Some(y * amp)
    }
}

impl Source for BasicSource {
    fn current_span_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        1
    }

    fn sample_rate(&self) -> u32 {
        self.live_sample_rate()
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}
