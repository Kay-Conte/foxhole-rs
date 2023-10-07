use vegemite::{RequestState, Resolve, ResolveGuard, Route, sys, run, PathIter};

use std::str;

struct Body<'a>(&'a str);

impl<'a, 'b> Resolve<'b> for Body<'a> {
    type Output = Body<'b>;

    fn resolve(ctx: &'b RequestState, _path_iter: &mut PathIter) -> ResolveGuard<Self::Output> {
        ResolveGuard::Value(Body(str::from_utf8(ctx.request.body().get()).unwrap()))
    }
}

fn post(body: Body) -> u16{
    println!("Received body {}", body.0);

    200
}

fn main() {
    let route = Route::new(sys![post]);

    run("127.0.0.1:8080", route); 
}
