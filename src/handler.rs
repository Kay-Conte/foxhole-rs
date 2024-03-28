use std::collections::HashMap;

use crate::systems::{DynSystem, IntoDynSystem};

pub enum Method<T> {
    Get(T),
    Post(T),
}

impl<T> Method<T> {
    pub(crate) fn inner(self) -> (http::Method, T) {
        match self {
            Method::Get(v) => (http::Method::GET, v),
            Method::Post(v) => (http::Method::POST, v),
        }
    }
}

pub struct Handler {
    methods: HashMap<http::Method, DynSystem>,
}

impl Handler {
    pub(crate) fn new() -> Self {
        Self {
            methods: HashMap::new(),
        }
    }

    pub(crate) fn insert(&mut self, method: http::Method, system: DynSystem) {
        self.methods.insert(method, system);
    }

    pub(crate) fn get(&self, method: &http::Method) -> Option<&DynSystem> {
        self.methods.get(method)
    }
}

pub trait InsertHandler<A> {
    fn insert_to_handler(self, handler: &mut Handler);
}

macro_rules! handler {
    ($($x:ident, $y:ident),* $(,)?) => {
        #[allow(unused_parens)]
        impl<$($x, $y),*> InsertHandler<($($y),*)> for ($(Method<$x>),*) where
            $($x: IntoDynSystem<$y>,)*
        {
            fn insert_to_handler(self, handler: &mut Handler) {
                #[allow(non_snake_case)]
                let ($($x),*) = self;

                $(
                    let (method, system) = $x.inner();

                    handler.insert(method, system.into_dyn_system());
                )*
            }
        }
    }
}

macro_rules! handler_all {
    () => { };

    ($first:ident, $second:ident, $($x:ident),*$(,)?)  => {
        handler! { $first, $second, $($x,)* }

        handler_all! { $($x,)*}
    }
}

handler_all! { A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z }
