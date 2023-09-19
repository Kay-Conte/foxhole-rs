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

    pub fn empty() -> Self {
        Node::new(vec![])
    }

    pub fn route(mut self, path: impl Into<String>, node: Node) -> Self {
        self.children.insert(path.into(), node);

        self
    }

    pub fn systems(&self) -> &[DynSystem] {
        &self.systems
    }

    pub fn get_child<'a, 'b>(&'a self, path: &'b str) -> Option<&'a Node> {
        self.children.get(path)
    }
}
