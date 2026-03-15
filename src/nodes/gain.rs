use crate::patch::{Node, SynthSource};
use rodio::Source;

pub struct Gain {
    gain: f32,
}

impl Gain {
    pub fn new(gain: f32) -> Self {
        Self { gain }
    }
}

impl Node for Gain {
    fn apply(&self, input: SynthSource) -> SynthSource {
        Box::new(input.amplify(self.gain))
    }

    fn name(&self) -> &'static str {
        "Gain"
    }
}
