use rodio::Source;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

/// boxed Rodio source producing mono `f32` samples, `Send` so it can live across threads
pub type SynthSource = Box<dyn Source<Item = f32> + Send>;

pub type Gate = Arc<AtomicBool>;

/// an effect/processor that transforms one source into another (filter, gain, ADSR, etc)
pub trait Node: Send + Sync {
    fn apply(&self, input: SynthSource) -> SynthSource;
    fn name(&self) -> &'static str;
}

/// a root source factory (oscillator/noise generator/etc), before nodes run
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
        Self {
            generator,
            nodes: vec![],
        }
    }

    pub fn push_node(mut self, node: Box<dyn Node>) -> Self {
        self.nodes.push(node);
        self
    }

    pub fn name(&self) -> &'static str {
        self.generator.name()
    }

    pub fn create(&self, frequency: f32) -> SynthSource {
        let mut src = self.generator.create(frequency);
        for n in &self.nodes {
            src = n.apply(src);
        }
        src
    }
}
