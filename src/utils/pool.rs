use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub struct Pool<T: Default> {
    inner: Arc<Mutex<Vec<T>>>,
}

impl<T: Default> Pool<T> {
    pub fn from_vec(vec: Vec<T>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(vec)),
        }
    }

    fn pop(&self) -> Option<T> {
        self.inner.lock().unwrap().pop()
    }

    pub fn take(&self) -> Pooled<T> {
        Pooled {
            object: self.pop().unwrap_or_else(|| T::default()),
            pool: Self {
                inner: self.inner.clone(),
            },
        }
    }

    fn insert(&self, object: T) {
        self.inner.lock().unwrap().push(object);
    }
}

#[derive(Debug)]
pub struct Pooled<T: Default> {
    object: T,
    pool: Pool<T>,
}

impl<T: Default> Drop for Pooled<T> {
    fn drop(&mut self) {
        let obj = std::mem::take(&mut self.object);
        self.pool.insert(obj);
    }
}

impl<T: Default> std::ops::Deref for Pooled<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.object
    }
}

impl<T: Default> std::ops::DerefMut for Pooled<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.object
    }
}
