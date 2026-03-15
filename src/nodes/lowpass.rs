use crate::patch::{Node, SynthSource};
use rodio::Source;
use std::f32::consts::TAU;
use std::sync::Arc;
use std::sync::RwLock;
use std::time::Duration;

#[derive(Debug, Clone, Copy)]
pub struct LowPassParams {
    pub cutoff_hz: f32,
}

#[derive(Clone)]
pub struct LowPass {
    params: Arc<RwLock<LowPassParams>>,
}

impl LowPass {
    pub fn new(cutoff_hz: f32) -> Self {
        Self {
            params: Arc::new(RwLock::new(LowPassParams { cutoff_hz })),
        }
    }

    pub fn from_params(params: LowPassParams) -> Self {
        Self {
            params: Arc::new(RwLock::new(params)),
        }
    }

    pub fn params(&self) -> LowPassParams {
        *self.params.read().unwrap()
    }

    pub fn set_cutoff_hz(&self, cutoff_hz: f32) {
        self.params.write().unwrap().cutoff_hz = cutoff_hz.max(1.0);
    }

    pub fn set_all(&self, params: LowPassParams) {
        *self.params.write().unwrap() = LowPassParams {
            cutoff_hz: params.cutoff_hz.max(1.0),
        };
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
        Box::new(LowPassSource {
            input,
            params: self.params.clone(),
            prev_y: 0.0,
        })
    }

    fn name(&self) -> &'static str {
        "Low Pass"
    }
}

struct LowPassSource {
    input: SynthSource,
    params: Arc<RwLock<LowPassParams>>,
    prev_y: f32,
}

impl Iterator for LowPassSource {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        let x = self.input.next()?;
        let sr = self.input.sample_rate() as f32;
        let cutoff = self.params.read().unwrap().cutoff_hz;
        let alpha = LowPass::calc_alpha(sr, cutoff);
        let y = alpha * x + (1.0 - alpha) * self.prev_y;
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
