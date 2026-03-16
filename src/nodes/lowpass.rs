use crate::patch::SynthSource;
use crate::shared::Shared;
use rodio::Source;
use std::f32::consts::TAU;
use std::time::Duration;

#[derive(Debug, Clone, Copy)]
pub struct LowPassParams {
    pub cutoff_hz: f32,
}

pub type LowPassHandle = Shared<LowPassParams>;

#[inline]
pub fn lowpass_handle(params: LowPassParams) -> LowPassHandle {
    Shared::new(LowPassParams {
        cutoff_hz: params.cutoff_hz.max(1.0),
    })
}

#[inline]
pub fn lowpass(input: SynthSource, params: LowPassHandle) -> SynthSource {
    Box::new(LowPassSource {
        input,
        params,
        prev_y: 0.0,
    })
}

fn alpha(sample_rate: f32, cutoff_hz: f32) -> f32 {
    let cutoff_hz = cutoff_hz.clamp(1.0, sample_rate * 0.45);
    let dt = 1.0 / sample_rate;
    let tau = 1.0 / (TAU * cutoff_hz);
    dt / (tau + dt)
}

struct LowPassSource {
    input: SynthSource,
    params: LowPassHandle,
    prev_y: f32,
}

impl Iterator for LowPassSource {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        let x = self.input.next()?;
        let sr = self.input.sample_rate() as f32;
        let cutoff = self.params.get().cutoff_hz;
        let a = alpha(sr, cutoff);

        let y = a * x + (1.0 - a) * self.prev_y;
        self.prev_y = y;

        Some(y)
    }
}

impl Source for LowPassSource {
    fn current_span_len(&self) -> Option<usize> {
        self.input.current_span_len()
    }

    fn channels(&self) -> u16 {
        self.input.channels()
    }

    fn sample_rate(&self) -> u32 {
        self.input.sample_rate()
    }

    fn total_duration(&self) -> Option<Duration> {
        self.input.total_duration()
    }
}
