use rodio::Source;
use crate::audio_patch::Node;

pub struct Gain {
    gain: f32,
}

impl Gain {
    fn new(gain:f32) -> Gain {
        return Gain { gain };
    }
}

impl Node for  Gain{
    fn apply(&self, input: crate::audio_patch::SynthSource) -> crate::audio_patch::SynthSource {
        Box::new(input.amplify(self.gain))
    }
    fn name(&self) -> &'static str {
        "Gain"
    }
}
