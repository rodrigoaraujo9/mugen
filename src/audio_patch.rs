use rodio::Source;

/// boxed Rodio source producing mono `f32` samples, `Send` so it can live across threads
pub type SynthSource = Box<dyn Source<Item = f32> + Send>;

/// something that can produce audio for a given frequency (oscillator, sampler, full patch, etc)
pub trait AudioSource: Send + Sync {
    fn create_source(&self, frequency: f32) -> SynthSource;
    fn name(&self) -> &'static str;
}

/// an effect/processor that transforms one source into another (filter, gain, ADSR, etc)
/// nodes may own a target node they drive internally (e.g. AdsrNode wrapping Gain)
pub trait Node: Send + Sync {
    fn apply(&self, input: SynthSource) -> SynthSource;
    fn name(&self) -> &'static str;
}

/// a root source factory for a patch (oscillator/noise generator/etc), before nodes run
pub trait Generator: Send + Sync {
    fn create(&self, frequency: f32) -> SynthSource;
    fn name(&self) -> &'static str;
}

/// a patch = one generator feeding a chain of nodes (generator → node1 → node2 → ...)
pub struct PatchSource {
    generator: Box<dyn Generator>,
    nodes: Vec<Box<dyn Node>>,
}

impl PatchSource {
    pub fn new(generator: Box<dyn Generator>) -> Self {
        Self { generator, nodes: vec![] }
    }

    pub fn push_node(mut self, node: Box<dyn Node>) -> Self {
        self.nodes.push(node);
        self
    }
}

impl AudioSource for PatchSource {
    fn create_source(&self, frequency: f32) -> SynthSource {
        let mut src = self.generator.create(frequency);
        for n in &self.nodes {
            src = n.apply(src);
        }
        src
    }
    fn name(&self) -> &'static str {
        self.generator.name()
    }
}
