use crate::{
    action::{Action, IntoAction},
    resolve::{Resolve, ResolveGuard},
    tasks::RequestState,
};

use std::collections::VecDeque;

#[doc(hidden)]
pub trait System<'a, T> {
    fn run(self, ctx: &'a RequestState, captures: VecDeque<String>) -> Action;
}

#[doc(hidden)]
pub struct DynSystem {
    #[allow(clippy::type_complexity)]
    inner: Box<dyn Fn(&RequestState, VecDeque<String>) -> Action + 'static + Send + Sync>,
}

impl DynSystem {
    pub fn new<A>(system: impl for<'a> System<'a, A> + 'static + Send + Sync + Copy) -> Self {
        DynSystem {
            inner: Box::new(move |ctx, captures| system.run(ctx, captures)),
        }
    }

    pub fn call(&self, ctx: &RequestState, captures: VecDeque<String>) -> Action {
        (self.inner)(ctx, captures)
    }
}

#[doc(hidden)]
pub trait IntoDynSystem<T> {
    fn into_dyn_system(self) -> DynSystem;
}

impl<T, A> IntoDynSystem<A> for T
where
    T: for<'a> System<'a, A> + 'static + Send + Sync + Copy,
{
    fn into_dyn_system(self) -> DynSystem {
        DynSystem::new(self)
    }
}

macro_rules! system {
    ($($x:ident),* $(,)?) => {
        impl<'a, RESPONSE, $($x,)* BASE> System<'a, (RESPONSE, $($x,)*)> for BASE
        where
            BASE: Fn($($x,)*) -> RESPONSE + Fn($($x::Output<'a>,)*) -> RESPONSE,
            $($x: Resolve,)*
            RESPONSE: IntoAction,
        {
            #[allow(unused)]
            fn run(self, mut ctx: &'a RequestState, mut captures: VecDeque<String>) -> Action {


                $(
                #[allow(non_snake_case)]
                let $x = match $x::resolve(&ctx, &mut captures) {
                    ResolveGuard::Value(v) => v,
                    ResolveGuard::Err(e) => return Action::Err(e),
                    ResolveGuard::Respond(r) => return Action::Respond(r), };)*

                let r = self($($x,)*);

                r.action()
            }
        }
    }
}

macro_rules! system_all {
    () => {
        system! { }
    };

    ($first:ident, $($x:ident),*$(,)?)  => {
        system! { $first, $($x,)* }

        system_all! { $($x,)*}
    }
}

system_all! { A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z }
