use std::sync::{Arc, RwLock};

#[derive(Clone)]
pub struct Shared<T>(Arc<RwLock<T>>);

impl<T> Shared<T> {
    pub fn new(value: T) -> Self {
        Self(Arc::new(RwLock::new(value)))
    }

    pub fn get(&self) -> T
    where
        T: Copy,
    {
        *self.0.read().unwrap()
    }

    pub fn set(&self, value: T) {
        *self.0.write().unwrap() = value;
    }

    pub fn update(&self, f: impl FnOnce(&mut T)) {
        f(&mut self.0.write().unwrap());
    }
}
