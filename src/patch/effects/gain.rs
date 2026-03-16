//! Scales signal amplitude using shared live control

use crate::patch::{Effect, PatchSource};
use crate::shared::Shared;

#[derive(Debug, Clone, Copy)]
pub struct Gain {
    pub amount: f32,
}

pub type GainHandle = Shared<Gain>;

#[inline]
pub fn make_gain(amount: f32) -> GainHandle {
    Shared::new(Gain {
        amount: amount.max(0.0),
    })
}

struct GainSource {
    input: PatchSource,
    gain: GainHandle,
}

impl Iterator for GainSource {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        let x = self.input.next()?;
        let g = self.gain.get().amount.max(0.0);
        Some(x * g)
    }
}

crate::impl_source_passthrough!(GainSource, input);

impl Effect for Shared<Gain> {
    fn name(&self) -> &'static str {
        "Gain"
    }

    fn apply(&self, input: PatchSource) -> PatchSource {
        Box::new(GainSource {
            input,
            gain: self.clone(),
        })
    }
}
