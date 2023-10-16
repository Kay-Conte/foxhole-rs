use crate::{
    action::Action,
    action::RawResponse,
    resolve::{Resolve, ResolveGuard},
    tasks::{PathIter, RequestState},
};

#[doc(hidden)]
pub trait System<'a, T> {
    fn run(self, ctx: &'a RequestState, path_iter: &mut PathIter) -> Option<RawResponse>;
}

#[doc(hidden)]
pub struct DynSystem {
    inner: Box<dyn Fn(&RequestState, &mut PathIter) -> Option<RawResponse> + 'static + Send + Sync>,
}

impl DynSystem {
    pub fn new<A>(system: impl for<'a> System<'a, A> + 'static + Send + Sync + Copy) -> Self {
        DynSystem {
            inner: Box::new(move |ctx, path_iter| system.run(ctx, path_iter)),
        }
    }

    pub fn call(&self, ctx: &RequestState, path_iter: &mut PathIter) -> Option<RawResponse> {
        (self.inner)(ctx, path_iter)
    }
}

macro_rules! system {
    ($($x:ident),* $(,)?) => {
        impl<'a, RESPONSE, $($x,)* BASE> System<'a, (RESPONSE, $($x,)*)> for BASE
        where
            BASE: Fn($($x,)*) -> RESPONSE + Fn($($x::Output,)*) -> RESPONSE,
            $($x: Resolve<'a>,)*
            RESPONSE: Action,
        {
            #[allow(unused)]
            fn run(self, ctx: &'a RequestState, path_iter: &mut PathIter) -> Option<RawResponse> {


                $(
                #[allow(non_snake_case)]
                let $x = match $x::resolve(ctx, path_iter) {
                    ResolveGuard::Value(v) => v,
                    ResolveGuard::None => return None,
                    ResolveGuard::Respond(r) => return Some(r), };)*

                let r = self($($x,)*);

                r.action()
            }
        }
    }
}

macro_rules! all {
    () => {
        system! { }
    };

    ($first:ident, $($x:ident),*$(,)?)  => {
        system! { $first, $($x,)* }

        all! { $($x,)*}
    }
}

all! { A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z }
