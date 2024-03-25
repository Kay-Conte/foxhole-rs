use std::collections::HashMap;

use crate::systems::DynSystem;

/// A Node in the Router tree.
pub struct Scope {
    children: HashMap<String, Scope>,
    systems: Vec<DynSystem>,
}

impl Scope {
    /// Construct a new `Scope`
    pub fn new(systems: Vec<DynSystem>) -> Self {
        Self {
            children: HashMap::new(),
            systems,
        }
    }

    /// Construct an empty `Scope`
    pub fn empty() -> Self {
        Scope::new(vec![])
    }

    /// Add a `Scope` as a child of this node
    pub fn route(mut self, path: impl Into<String>, route: impl Into<Scope>) -> Self {
        self.children.insert(path.into(), route.into());

        self
    }

    /// Access the list of systems associated with this node
    pub fn systems(&self) -> &[DynSystem] {
        &self.systems
    }

    /// Scope to a child of this node by path
    pub fn get_child<'a>(&'a self, path: &str) -> Option<&'a Scope> {
        self.children.get(path)
    }
}

impl From<Vec<DynSystem>> for Scope {
    fn from(value: Vec<DynSystem>) -> Self {
        Scope::new(value)
    }
}
