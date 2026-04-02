//! Attenuates high frequencies with shared cutoff control

use crate::patch::shared::Shared;
use crate::patch::{Effect, PatchSource};
use std::f32::consts::TAU;

#[derive(Debug, Clone)]
pub struct LowPass {
    pub cutoff_hz: f32,
}

pub type LowPassHandle = Shared<LowPass>;

#[inline]
#[must_use] 
pub fn make_lowpass(lowpass: &LowPass) -> LowPassHandle {
    Shared::new(LowPass {
        cutoff_hz: lowpass.cutoff_hz.max(1.0),
    })
}

fn alpha(sample_rate: f32, cutoff_hz: f32) -> f32 {
    let cutoff_hz = cutoff_hz.clamp(1.0, sample_rate * 0.45);
    let dt = 1.0 / sample_rate;
    let tau = 1.0 / (TAU * cutoff_hz);
    dt / (tau + dt)
}

struct LowPassSource {
    input: PatchSource,
    lowpass: LowPassHandle,
    prev_y: f32,
}

impl Iterator for LowPassSource {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        let x = self.input.next()?;
        let sr = self.input.sample_rate() as f32;
        let cutoff = self.lowpass.get().cutoff_hz;
        let a = alpha(sr, cutoff);

        let y = a * x + (1.0 - a) * self.prev_y;
        self.prev_y = y;

        Some(y)
    }
}

crate::impl_source_passthrough!(LowPassSource, input);

impl Effect for Shared<LowPass> {
    fn name(&self) -> &'static str {
        "LowPass"
    }

    fn apply(&self, input: PatchSource) -> PatchSource {
        Box::new(LowPassSource {
            input,
            lowpass: self.clone(),
            prev_y: 0.0,
        })
    }
}
