use rodio::Source;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, RwLock};

pub type SynthSource = Box<dyn Source<Item = f32> + Send>;
pub type Gate = Arc<AtomicBool>;
pub type SharedGenerator = Arc<dyn Generator>;
pub type SharedNode = Arc<dyn Node>;

pub trait Node: Send + Sync {
    fn apply(&self, input: SynthSource) -> SynthSource;
    fn name(&self) -> &'static str;
}

pub trait Generator: Send + Sync {
    fn create(&self, frequency: f32) -> SynthSource;
    fn name(&self) -> &'static str;
}

pub struct Patch {
    generator: SharedGenerator,
    nodes: RwLock<Vec<SharedNode>>,
}

impl Patch {
    pub fn new(generator: SharedGenerator) -> Self {
        Self {
            generator,
            nodes: RwLock::new(Vec::new()),
        }
    }

    #[inline]
    pub fn generator(&self) -> SharedGenerator {
        self.generator.clone()
    }

    #[inline]
    pub fn nodes(&self) -> Vec<SharedNode> {
        self.nodes.read().unwrap().clone()
    }

    pub fn set_nodes(&self, nodes: Vec<SharedNode>) {
        *self.nodes.write().unwrap() = nodes;
    }

    pub fn push_node(&self, node: SharedNode) {
        self.nodes.write().unwrap().push(node);
    }

    pub fn replace_node(&self, index: usize, node: SharedNode) -> bool {
        let mut nodes = self.nodes.write().unwrap();
        if let Some(slot) = nodes.get_mut(index) {
            *slot = node;
            true
        } else {
            false
        }
    }

    pub fn clear_nodes(&self) {
        self.nodes.write().unwrap().clear();
    }

    pub fn create(&self, frequency: f32) -> SynthSource {
        let mut src = self.generator.create(frequency);
        for node in self.nodes.read().unwrap().iter().cloned() {
            src = node.apply(src);
        }
        src
    }

    #[inline]
    pub fn name(&self) -> &'static str {
        self.generator.name()
    }
}
