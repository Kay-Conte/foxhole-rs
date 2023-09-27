use std::collections::HashMap;

use crate::systems::DynSystem;

/// A Node in the Router tree.
pub struct Route {
    children: HashMap<String, Route>,
    systems: Vec<DynSystem>,
}

impl Route {
    /// Construct a new `Route`
    pub fn new(systems: Vec<DynSystem>) -> Self {
        Self {
            children: HashMap::new(),
            systems,
        }
    }

    /// Construct an empty `Route`
    pub fn empty() -> Self {
        Route::new(vec![])
    }

    /// Add a `Route` as a child of this node
    pub fn route(mut self, path: impl Into<String>, node: Route) -> Self {
        self.children.insert(path.into(), node);

        self
    }

    /// Access the list of systems associated with this node
    pub fn systems(&self) -> &[DynSystem] {
        &self.systems
    }
    
    /// Route to a child of this node by path
    pub fn get_child<'a>(&'a self, path: &str) -> Option<&'a Route> {
        self.children.get(path)
    }
}

#[cfg(feature = "shuttle")]
#[async_trait::async_trait]
impl shuttle_service::Service for Route {
    async fn bind(self, addr: std::net::SocketAddr) -> Result<(), shuttle_service::Error> {
        crate::run(addr, self);
        Ok(())
    }
}
