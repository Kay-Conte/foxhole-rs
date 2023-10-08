use vegemite::{RequestState, Resolve, ResolveGuard, Route, sys, run, PathIter, Post};

use std::cell::Ref;

struct Body<'a>(Ref<'a, Vec<u8>>);

impl<'a, 'b> Resolve<'b> for Body<'a> {
    type Output = Body<'b>;

    fn resolve(ctx: &'b RequestState, _path_iter: &mut PathIter) -> ResolveGuard<Self::Output> {
        ResolveGuard::Value(Body(ctx.request.body().get()))
    }
}

fn post(_post: Post, body: Body) -> u16{
    200
}

fn main() {
    let route = Route::new(sys![post]);

    run("127.0.0.1:8080", route); 
}
