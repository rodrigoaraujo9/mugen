use rodio::Source;
use crate::patch::Node;

pub struct Gain {
    gain: f32,
}

impl Gain {
    pub fn new(gain:f32) -> Gain {
        Gain { gain }
    }
}

impl Node for  Gain{
    fn apply(&self, input: crate::patch::SynthSource) -> crate::patch::SynthSource {
        Box::new(input.amplify(self.gain))
    }
    fn name(&self) -> &'static str {
        "Gain"
    }
}
