use std::collections::HashMap;

use crate::systems::DynSystem;

pub struct Route {
    children: HashMap<String, Route>,
    systems: Vec<DynSystem>,
}

impl Route {
    pub fn new(systems: Vec<DynSystem>) -> Self {
        Self {
            children: HashMap::new(),
            systems,
        }
    }

    pub fn empty() -> Self {
        Route::new(vec![])
    }

    pub fn route(mut self, path: impl Into<String>, node: Route) -> Self {
        self.children.insert(path.into(), node);

        self
    }

    pub fn systems(&self) -> &[DynSystem] {
        &self.systems
    }

    pub fn get_child<'a, 'b>(&'a self, path: &'b str) -> Option<&'a Route> {
        self.children.get(path)
    }
}
