use std::{borrow::BorrowMut, collections::HashMap};

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

struct Node {
    handler: Option<DynSystem>,
    children: Vec<(Pattern, Node)>,
}

impl Node {
    fn new() -> Self {
        Self {
            handler: None,
            children: Vec::new(),
        }
    }
}

enum Pattern {
    Exact(String),
    Capture,
    Collect,
}

impl Pattern {
    fn new(s: &str) -> Self {
        if s.starts_with(":") {
            Pattern::Capture
        } else if s == "*" {
            Pattern::Collect
        } else {
            Pattern::Exact(s.to_string())
        }
    }

    fn exact(&self, rhs: &str) -> bool {
        match self {
            Pattern::Exact(s) => s == rhs,
            Pattern::Capture if rhs.starts_with(":") => true,
            Pattern::Collect if rhs == "*" => true,
            _ => false,
        }
    }
}

pub struct Router {
    root: Node,
}

impl Router {
    pub fn new() -> Self {
        Self { root: Node::new() }
    }

    pub fn add_route(mut self, path: &str, handler: DynSystem) -> Self {
        let mut cursor = &mut self.root;
        for segment in path.split("/") {
            if segment.is_empty() {
                continue;
            }

            if let Some(idx) = cursor.children.iter().position(|i| i.0.exact(segment)) {
                cursor = &mut cursor.children[idx].1;
            } else {
                cursor.children.push((Pattern::new(segment), Node::new()));

                println!("{}", cursor.children.len());

                cursor = cursor.children.last_mut().unwrap().1.borrow_mut();
            }
        }

        cursor.handler = Some(handler);

        self
    }

    pub fn route(&self, path: &str) -> Option<(&DynSystem, Vec<String>)> {
        let mut captured = vec![];

        let mut cursor = &self.root;
        let mut iter = path.split("/");

        'outer: while let Some(segment) = iter.next() {
            if segment.is_empty() {
                continue;
            }

            for (pattern, node) in cursor.children.iter() {
                match pattern {
                    Pattern::Exact(s) if s == segment => {
                        cursor = &node;
                        
                        break;
                    }
                    Pattern::Capture => {
                        captured.push(segment.to_string());

                        cursor = node;
                        break;
                    }
                    Pattern::Collect => {
                        captured.push(segment.to_string());
                        captured.extend(iter.map(String::from));

                        cursor = node;

                        break 'outer;
                    }
                    _ => {}
                }
            }
        }

        cursor.handler.as_ref().map(|i| (i, captured))
    }
}
