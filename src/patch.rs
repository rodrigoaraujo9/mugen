use rodio::Source;
use std::sync::Arc;
use std::sync::RwLock;
use std::sync::atomic::AtomicBool;

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

    pub fn generator(&self) -> &SharedGenerator {
        &self.generator
    }

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
        if index >= nodes.len() {
            return false;
        }
        nodes[index] = node;
        true
    }

    pub fn clear_nodes(&self) {
        self.nodes.write().unwrap().clear();
    }

    pub fn create(&self, frequency: f32) -> SynthSource {
        let mut src = self.generator.create(frequency);
        let nodes = self.nodes.read().unwrap().clone();
        for node in nodes {
            src = node.apply(src);
        }
        src
    }

    pub fn name(&self) -> &'static str {
        self.generator.name()
    }
}
