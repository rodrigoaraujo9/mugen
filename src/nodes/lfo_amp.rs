use crate::generators::basic::Wave;
use crate::nodes::lfo::LfoOsc;
use crate::patch::SynthSource;
use crate::shared::Shared;
use rodio::Source;
use std::time::Duration;

#[derive(Debug, Clone, Copy)]
pub struct LfoAmpParams {
    pub kind: Wave,
    pub rate_hz: f32,
    pub depth: f32,
    pub base_gain: f32,
}

pub type LfoAmpHandle = Shared<LfoAmpParams>;

#[inline]
pub fn lfo_amp_handle(params: LfoAmpParams) -> LfoAmpHandle {
    Shared::new(LfoAmpParams {
        kind: params.kind,
        rate_hz: params.rate_hz.max(0.0),
        depth: params.depth.clamp(0.0, 1.0),
        base_gain: params.base_gain.max(0.0),
    })
}

#[inline]
pub fn lfo_amp(input: SynthSource, params: LfoAmpHandle) -> SynthSource {
    let p = params.get();
    let sr = input.sample_rate().max(1);

    Box::new(LfoAmpSource {
        input,
        params,
        lfo: LfoOsc::new(p.kind, p.rate_hz, sr),
    })
}

struct LfoAmpSource {
    input: SynthSource,
    params: LfoAmpHandle,
    lfo: LfoOsc,
}

impl Iterator for LfoAmpSource {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        let x = self.input.next()?;
        let p = self.params.get();

        self.lfo.sync_sample_rate(self.input.sample_rate());
        self.lfo.set_kind(p.kind);
        self.lfo.set_rate_hz(p.rate_hz);

        let gain = p.base_gain.max(0.0) * (1.0 + p.depth.clamp(0.0, 1.0) * self.lfo.next_value());
        Some(x * gain)
    }
}

impl Source for LfoAmpSource {
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
