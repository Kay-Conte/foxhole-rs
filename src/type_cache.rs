use std::{
    any::{Any, TypeId},
    collections::HashMap,
};

type Value = Box<dyn Any + Sync + Send>;

pub trait TypeCacheKey: 'static {
    type Value: Send + Sync;
}

#[derive(Default)]
pub struct TypeCache {
    inner: HashMap<TypeId, Value>,
}

impl TypeCache {
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }

    pub fn get<K: TypeCacheKey>(&self) -> Option<&K::Value> {
        self.inner
            .get(&TypeId::of::<K>())
            .map(|f| f.downcast_ref().unwrap())
    }

    pub fn insert<K: TypeCacheKey>(&mut self, value: K::Value) -> Option<Box<K::Value>> {
        self.inner
            .insert(TypeId::of::<K>(), Box::new(value))
            .map(|f| f.downcast().unwrap())
    }

    pub fn remove<K: TypeCacheKey>(&mut self) -> Option<Box<K::Value>> {
        self.inner
            .remove(&TypeId::of::<K>())
            .map(|f| f.downcast().unwrap())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    pub struct UserId;

    impl TypeCacheKey for UserId {
        type Value = Arc<u32>;
    }

    #[test]
    fn type_map() {
        let mut cache = TypeCache::new();

        assert!(cache.insert::<UserId>(Arc::new(0)).is_none());

        assert!(cache.get::<UserId>().is_some());

        assert!(cache.insert::<UserId>(Arc::new(0)).is_some());

        assert!(cache.remove::<UserId>().is_some());
    }
}
