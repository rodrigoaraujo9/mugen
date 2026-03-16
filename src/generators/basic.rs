use crate::config::{AMP_DEFAULT, SAMPLE_RATE};
use crate::patch::Sample;
use crate::shared::Shared;
use rodio::Source;
use std::f32::consts::TAU;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wave {
    Sine,
    Saw,
    Square,
    Triangle,
    Noise,
}

impl Wave {
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
pub struct OscParams {
    pub kind: Wave,
    pub amplitude: f32,
    pub sample_rate: u32,
}

impl Default for OscParams {
    fn default() -> Self {
        Self {
            kind: Wave::Sine,
            amplitude: AMP_DEFAULT,
            sample_rate: SAMPLE_RATE,
        }
    }
}

pub type OscParamsHandle = Shared<OscParams>;

#[inline]
pub fn osc_params(kind: Wave) -> OscParamsHandle {
    Shared::new(OscParams {
        kind,
        ..OscParams::default()
    })
}

#[inline]
pub fn osc(frequency: f32, params: OscParamsHandle) -> OscSource {
    OscSource::new(frequency, params)
}

pub struct OscSource {
    params: OscParamsHandle,
    frequency: f32,
    phase: f32,
    rng: u64,
}

impl OscSource {
    pub fn new(frequency: f32, params: OscParamsHandle) -> Self {
        Self {
            params,
            frequency: frequency.max(0.0),
            phase: 0.0,
            rng: 0x1234_5678_9ABC_DEF0,
        }
    }

    #[inline]
    fn sample_rate_live(&self) -> u32 {
        self.params.get().sample_rate.max(1)
    }

    fn step_phase(&mut self) -> f32 {
        let p = self.phase;
        self.phase += self.frequency / self.sample_rate_live() as f32;

        if self.phase >= 1.0 {
            self.phase -= self.phase.floor();
        }

        p
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

impl Iterator for OscSource {
    type Item = Sample;

    fn next(&mut self) -> Option<Self::Item> {
        let params = self.params.get();
        let amp = params.amplitude.max(0.0);

        let y = match params.kind {
            Wave::Sine => (TAU * self.step_phase()).sin(),
            Wave::Square => {
                if self.step_phase() < 0.5 {
                    1.0
                } else {
                    -1.0
                }
            }
            Wave::Triangle => {
                let p = self.step_phase();
                if p < 0.5 {
                    -1.0 + 4.0 * p
                } else {
                    3.0 - 4.0 * p
                }
            }
            Wave::Saw => 2.0 * self.step_phase() - 1.0,
            Wave::Noise => self.next_noise(),
        };

        Some(y * amp)
    }
}

impl Source for OscSource {
    fn current_span_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        1
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate_live()
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}
