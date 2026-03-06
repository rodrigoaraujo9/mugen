use std::{f32::consts::TAU, time::Duration};

use rodio::Source;

use crate::audio_patch::{Node, SynthSource};

#[derive(Debug, Clone, Copy)]
pub struct LowPass {
    pub cutoff_hz: f32,
}

impl LowPass {
    pub fn new(cutoff_hz: f32) -> Self {
        Self { cutoff_hz }
    }

    fn calc_alpha(sample_rate: f32, cutoff_hz: f32) -> f32 {
        let cutoff_hz = cutoff_hz.clamp(1.0, sample_rate * 0.45);
        let dt = 1.0 / sample_rate;
        let tau = 1.0 / (TAU * cutoff_hz);
        dt / (tau + dt)
    }
}

impl Node for LowPass {
    fn apply(&self, input: SynthSource) -> SynthSource {
        let sr = input.sample_rate() as f32;
        let cutoff_hz = self.cutoff_hz.clamp(1.0, sr * 0.45);
        let alpha = Self::calc_alpha(sr, cutoff_hz);

        Box::new(LowPassSource {
            input,
            cutoff_hz,
            alpha,
            prev_y: 0.0,
        })
    }

    fn name(&self) -> &'static str {
        "Low Pass"
    }
}

struct LowPassSource {
    input: SynthSource,
    cutoff_hz: f32,
    alpha: f32,
    prev_y: f32,
}

impl LowPassSource {
    fn calc_alpha(sample_rate: f32, cutoff_hz: f32) -> f32 {
        let cutoff_hz = cutoff_hz.clamp(1.0, sample_rate * 0.45);
        let dt = 1.0 / sample_rate;
        let tau = 1.0 / (TAU * cutoff_hz);
        dt / (tau + dt)
    }
}

impl Iterator for LowPassSource {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        let x = self.input.next()?;

        let sr = self.input.sample_rate() as f32;
        self.alpha = Self::calc_alpha(sr, self.cutoff_hz);

        let y = self.alpha * x + (1.0 - self.alpha) * self.prev_y;
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
