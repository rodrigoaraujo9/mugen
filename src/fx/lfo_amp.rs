use std::time::Duration;

use rodio::Source;

use crate::audio_patch::{Node, SynthSource};
use crate::patches::basic::BasicKind;
use crate::fx::lfo::LfoOsc;

pub struct LfoAmpNode {
    pub kind: BasicKind,
    pub rate_hz: f32,  // LFO frequency
    pub depth: f32,
    pub base_gain: f32
}

impl LfoAmpNode {
    pub fn new(kind: BasicKind, rate_hz: f32, depth: f32) -> Self {
        Self {
            kind,
            rate_hz,
            depth,
            base_gain: 1.0,
        }
    }
}

impl Node for LfoAmpNode {
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

        let l = self.lfo.next();

        // gain in [base*(1-depth), base*(1+depth)]
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
