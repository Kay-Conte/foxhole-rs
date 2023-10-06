use vegemite::{RequestState, Resolve, ResolveGuard, Post, Route, sys, run, Get};

struct Body(String);

impl Resolve for Body {
    fn resolve(ctx: &mut RequestState) -> ResolveGuard<Self> {
        ResolveGuard::Value(Body(
            String::from_utf8(ctx.request.body_mut().get().clone()).unwrap(),
        ))
    }
}

fn post(_post: Post, body: Body) -> u16{
    println!("Received body {}", body.0);

    200
}

fn get(_g: Get) -> u16 {
    200
}

fn main() {
    let route = Route::new(sys![post, get]);

    run("127.0.0.1:8080", route); 
}
