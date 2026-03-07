use std::time::Duration;
use rodio::Source;
use crate::patch::{Node, SynthSource};
use crate::generators::basic::BasicKind;
use crate::nodes::lfo::LfoOsc;

#[derive(Debug, Clone, Copy)]
pub struct LfoAmp {
    pub kind: BasicKind,
    pub rate_hz: f32,  // LFO frequency
    pub depth: f32,
    pub base_gain: f32
}

impl LfoAmp {
    pub fn new(kind: BasicKind, rate_hz: f32, depth: f32) -> Self {
        Self {
            kind,
            rate_hz,
            depth,
            base_gain: 1.0,
        }
    }
}

impl Node for LfoAmp {
    fn apply(&self, input: SynthSource) -> SynthSource {
        let sr = input.sample_rate();
        let kind = self.kind;
        let rate_hz = self.rate_hz;
        let depth = self.depth.clamp(0.0, 1.0);
        let base_gain = self.base_gain.max(0.0);

        Box::new(LfoAmpSource {
            input,
            lfo: LfoOsc::new(kind, rate_hz, sr),
            depth,
            base_gain,
        })
    }

    fn name(&self) -> &'static str {
        "LFO Amp"
    }
}

struct LfoAmpSource {
    input: SynthSource,
    lfo: LfoOsc,
    depth: f32,
    base_gain: f32,
}

impl Iterator for LfoAmpSource {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        let x = self.input.next()?;

        self.lfo.sync_sr(self.input.sample_rate());

        let l = self.lfo.next();
        let gain = self.base_gain * (1.0 + self.depth * l);
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
