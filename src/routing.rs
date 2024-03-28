use std::{borrow::BorrowMut, collections::VecDeque};

use crate::{
    fallback::default_fallback,
    handler::{Handler, InsertHandler},
    systems::{DynSystem, IntoDynSystem},
};

pub type Captures = VecDeque<String>;

struct Node {
    handler: Option<Handler>,
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
    fallback: DynSystem,
}

impl Router {
    pub fn new() -> Self {
        Self {
            root: Node::new(),
            fallback: default_fallback.into_dyn_system(),
        }
    }

    pub fn add_route<T>(mut self, path: &str, handler: impl InsertHandler<T>) -> Self {
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

        let mut new = Handler::new();

        handler.insert_to_handler(&mut new);

        cursor.handler = Some(new);

        self
    }

    pub fn fallback<T, A>(mut self, system: T) -> Self
    where
        T: IntoDynSystem<A>,
    {
        self.fallback = system.into_dyn_system();
        self
    }

    pub(crate) fn get_fallback(&self) -> &DynSystem {
        &self.fallback
    }

    pub(crate) fn route(&self, path: &str) -> Option<(&Handler, Captures)> {
        let mut captured = vec![];

        let mut cursor = &self.root;
        let mut iter = path.split("/");

        'outer: while let Some(segment) = iter.next() {
            if segment.is_empty() {
                continue;
            }

            let mut matched = false;

            for (pattern, node) in cursor.children.iter() {
                match pattern {
                    Pattern::Exact(s) if s == segment => {
                        cursor = &node;

                        matched = true;
                        break;
                    }
                    Pattern::Capture => {
                        captured.push(segment.to_string());

                        cursor = node;

                        matched = true;
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

            if !matched {
                return None;
            }
        }

        cursor
            .handler
            .as_ref()
            .map(|i| (i, VecDeque::from(captured)))
    }
}
