//! Thread-safe live parameter storage

use std::sync::{Arc, RwLock};

#[derive(Debug)]
pub struct Shared<T> {
    inner: Arc<RwLock<T>>,
}

impl<T> Shared<T> {
    #[inline]
    pub fn new(value: T) -> Self {
        Self {
            inner: Arc::new(RwLock::new(value)),
        }
    }

    #[inline]
    pub fn get(&self) -> T
    where
        T: Clone,
    {
        self.inner.read().expect("shared read poisoned").clone()
    }

    #[inline]
    pub fn set(&self, value: T) {
        *self.inner.write().expect("shared write poisoned") = value;
    }

    #[inline]
    pub fn update(&self, f: impl FnOnce(&mut T)) {
        let mut guard = self.inner.write().expect("shared write poisoned");
        f(&mut guard);
    }
}

impl<T> Clone for Shared<T> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}
