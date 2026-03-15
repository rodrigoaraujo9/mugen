use crate::generators::basic::BasicKind;
use crate::nodes::lfo::LfoOsc;
use crate::patch::{Node, SynthSource};
use rodio::Source;
use std::sync::Arc;
use std::sync::RwLock;
use std::time::Duration;

#[derive(Debug, Clone, Copy)]
pub struct LfoAmpParams {
    pub kind: BasicKind,
    pub rate_hz: f32,
    pub depth: f32,
    pub base_gain: f32,
}

#[derive(Clone)]
pub struct LfoAmp {
    params: Arc<RwLock<LfoAmpParams>>,
}

impl LfoAmp {
    pub fn new(kind: BasicKind, rate_hz: f32, depth: f32) -> Self {
        Self {
            params: Arc::new(RwLock::new(LfoAmpParams {
                kind,
                rate_hz,
                depth,
                base_gain: 1.0,
            })),
        }
    }

    pub fn from_params(params: LfoAmpParams) -> Self {
        Self {
            params: Arc::new(RwLock::new(params)),
        }
    }

    pub fn params(&self) -> LfoAmpParams {
        *self.params.read().unwrap()
    }

    pub fn set_kind(&self, kind: BasicKind) {
        self.params.write().unwrap().kind = kind;
    }

    pub fn set_rate_hz(&self, rate_hz: f32) {
        self.params.write().unwrap().rate_hz = rate_hz.max(0.0);
    }

    pub fn set_depth(&self, depth: f32) {
        self.params.write().unwrap().depth = depth.clamp(0.0, 1.0);
    }

    pub fn set_base_gain(&self, base_gain: f32) {
        self.params.write().unwrap().base_gain = base_gain.max(0.0);
    }

    pub fn set_all(&self, params: LfoAmpParams) {
        *self.params.write().unwrap() = LfoAmpParams {
            kind: params.kind,
            rate_hz: params.rate_hz.max(0.0),
            depth: params.depth.clamp(0.0, 1.0),
            base_gain: params.base_gain.max(0.0),
        };
    }
}

impl Node for LfoAmp {
    fn apply(&self, input: SynthSource) -> SynthSource {
        let sr = input.sample_rate().max(1);
        let initial = self.params();

        Box::new(LfoAmpSource {
            input,
            lfo: LfoOsc::new(initial.kind, initial.rate_hz, sr),
            params: self.params.clone(),
        })
    }

    fn name(&self) -> &'static str {
        "LFO Amp"
    }
}

struct LfoAmpSource {
    input: SynthSource,
    lfo: LfoOsc,
    params: Arc<RwLock<LfoAmpParams>>,
}

impl Iterator for LfoAmpSource {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        let x = self.input.next()?;
        self.lfo.sync_sr(self.input.sample_rate());

        let params = *self.params.read().unwrap();
        self.lfo.set_kind(params.kind);
        self.lfo.set_rate_hz(params.rate_hz);

        let gain = params.base_gain.max(0.0)
            * (1.0 + params.depth.clamp(0.0, 1.0) * self.lfo.next_value());

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
