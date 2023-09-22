use std::collections::HashMap;

use crate::systems::DynSystem;

/// A Node in the Router tree.
pub struct Route {
    children: HashMap<String, Route>,
    systems: Vec<DynSystem>,
}

impl Route {
    /// Construct a new `Route`
    #[must_use]
    pub fn new(systems: Vec<DynSystem>) -> Self {
        Self {
            children: HashMap::new(),
            systems,
        }
    }

    /// Construct an empty `Route`
    #[must_use]
    pub fn empty() -> Self {
        Route::new(vec![])
    }

    /// Add a `Route` as a child of this node
    #[must_use]
    pub fn route(mut self, path: impl Into<String>, node: Route) -> Self {
        self.children.insert(path.into(), node);

        self
    }

    /// Access the list of systems associated with this node
    #[must_use]
    pub fn systems(&self) -> &[DynSystem] {
        &self.systems
    }
    
    /// Route to a child of this node by path
    #[must_use]
    pub fn get_child<'a>(&'a self, path: &str) -> Option<&'a Route> {
        self.children.get(path)
    }
}
