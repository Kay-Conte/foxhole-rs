use std::{collections::HashMap, any::{TypeId, Any}, sync::{Arc, RwLock}};

type Value = Box<dyn Any + Sync + Send>;

pub type SharedTypeCache = Arc<RwLock<TypeCache>>;

pub struct TypeCache {
    inner: HashMap<TypeId, Value>,
}

impl TypeCache {
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }

    pub fn get<T: Any>(&self) -> Option<&T> {
        self.inner.get(&TypeId::of::<T>()).map(|f| f.downcast_ref().unwrap())
    }

    pub fn insert<T: Any + Send + Sync>(&mut self, value: T) -> Option<Box<T>> {
        self.inner.insert(TypeId::of::<T>(), Box::new(value)).map(|f| f.downcast().unwrap())
    } 

    pub fn remove<T: Any + Send + Sync>(&mut self) -> Option<Box<T>> {
        self.inner.remove(&TypeId::of::<T>()).map(|f| f.downcast().unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_get() {
        let mut cache = TypeCache::new();
        
        assert!(cache.get::<u32>().is_none());

        assert!(cache.insert::<u32>(0).is_none());

        assert!(cache.get::<u32>().is_some());

        assert!(cache.insert::<u32>(0).is_some())
    }
}
