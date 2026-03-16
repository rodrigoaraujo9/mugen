use crate::patch::SynthSource;
use crate::shared::Shared;
use rodio::Source;
use std::time::Duration;

#[derive(Debug, Clone, Copy)]
pub struct GainParams {
    pub amount: f32,
}

pub type GainHandle = Shared<GainParams>;

#[inline]
pub fn gain_handle(amount: f32) -> GainHandle {
    Shared::new(GainParams {
        amount: amount.max(0.0),
    })
}

#[inline]
pub fn gain(input: SynthSource, params: GainHandle) -> SynthSource {
    Box::new(GainSource { input, params })
}

struct GainSource {
    input: SynthSource,
    params: GainHandle,
}

impl Iterator for GainSource {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        let x = self.input.next()?;
        let g = self.params.get().amount.max(0.0);
        Some(x * g)
    }
}

impl Source for GainSource {
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
