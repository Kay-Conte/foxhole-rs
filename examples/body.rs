use vegemite::{RequestState, Resolve, ResolveGuard, Route, sys, run, PathIter, Post};

use std::str;

struct Body<'a>(&'a str);

impl<'a, 'b> Resolve<'b> for Body<'a> {
    type Output = Body<'b>;

    fn resolve(ctx: &'b RequestState, _path_iter: &mut PathIter) -> ResolveGuard<Self::Output> {
        let body = str::from_utf8(ctx.request.body().get().as_slice()).unwrap();

        ResolveGuard::Value(Body(body))
    }
}

fn post(_post: Post, body: Body) -> u16{
    println!("Body: {}", body.0);

    200
}

fn main() {
    let route = Route::new(sys![post]);

    run("127.0.0.1:8080", route); 
}
