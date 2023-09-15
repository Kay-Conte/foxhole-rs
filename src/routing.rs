use std::collections::HashMap;

use crate::systems::DynSystem;

pub struct Node {
    children: HashMap<String, Node>,
    systems: Vec<DynSystem>,
}

impl Node {
    pub fn new(systems: Vec<DynSystem>) -> Self {
        Self {
            children: HashMap::new(),
            systems,
        }
    }

    pub fn insert(mut self, path: impl Into<String>, node: Node) -> Self {
        self.children.insert(path.into(), node);

        self
    }
}
