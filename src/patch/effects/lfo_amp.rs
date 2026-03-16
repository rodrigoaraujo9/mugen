//! Applies low-frequency modulation to signal gain

use crate::patch::effects::lfo::LfoOsc;
use crate::patch::oscilators::basic::Wave;
use crate::patch::{Effect, PatchSource};
use crate::shared::Shared;

#[derive(Debug, Clone, Copy)]
pub struct LfoAmp {
    pub wave: Wave,
    pub rate_hz: f32,
    pub depth: f32,
    pub base_gain: f32,
}

pub type LfoAmpHandle = Shared<LfoAmp>;

#[inline]
pub fn make_lfo_amp(lfo: LfoAmp) -> LfoAmpHandle {
    Shared::new(LfoAmp {
        wave: lfo.wave,
        rate_hz: lfo.rate_hz.max(0.0),
        depth: lfo.depth.clamp(0.0, 1.0),
        base_gain: lfo.base_gain.max(0.0),
    })
}

struct LfoAmpSource {
    input: PatchSource,
    lfo_amp: LfoAmpHandle,
    lfo: LfoOsc,
}

impl Iterator for LfoAmpSource {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        let x = self.input.next()?;
        let cfg = self.lfo_amp.get();

        self.lfo.sync_sample_rate(self.input.sample_rate());
        self.lfo.set_wave(cfg.wave);
        self.lfo.set_rate_hz(cfg.rate_hz);

        let gain =
            cfg.base_gain.max(0.0) * (1.0 + cfg.depth.clamp(0.0, 1.0) * self.lfo.next_value());

        Some(x * gain)
    }
}

crate::impl_source_passthrough!(LfoAmpSource, input);

impl Effect for Shared<LfoAmp> {
    fn name(&self) -> &'static str {
        "LFO Amp"
    }

    fn apply(&self, input: PatchSource) -> PatchSource {
        let cfg = self.get();
        let sr = input.sample_rate().max(1);

        Box::new(LfoAmpSource {
            input,
            lfo_amp: self.clone(),
            lfo: LfoOsc::new(cfg.wave, cfg.rate_hz, sr),
        })
    }
}
