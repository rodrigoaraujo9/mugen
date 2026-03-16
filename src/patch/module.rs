use std::sync::Arc;

pub type ModuleApply<H> = fn(SynthSource, H) -> SynthSource;

pub trait PatchModule: Send + Sync {
    fn name(&self) -> &'static str;
    fn apply(&self, input: SynthSource) -> SynthSource;
}

pub type SharedPatchModule = Arc<dyn PatchModule>;

pub struct EffectModule<H>
where
    H: Clone + Send + Sync + 'static,
{
    name: &'static str,
    handle: H,
    apply_fn: ModuleApply<H>,
}

impl<H> EffectModule<H>
where
    H: Clone + Send + Sync + 'static,
{
    pub fn new(name: &'static str, handle: H, apply_fn: ModuleApply<H>) -> Self {
        Self {
            name,
            handle,
            apply_fn,
        }
    }
}

impl<H> PatchModule for EffectModule<H>
where
    H: Clone + Send + Sync + 'static,
{
    fn name(&self) -> &'static str {
        self.name
    }

    fn apply(&self, input: SynthSource) -> SynthSource {
        (self.apply_fn)(input, self.handle.clone())
    }
}

pub fn module<H>(name: &'static str, handle: H, apply_fn: ModuleApply<H>) -> SharedPatchModule
where
    H: Clone + Send + Sync + 'static,
{
    Arc::new(EffectModule::new(name, handle, apply_fn))
}
