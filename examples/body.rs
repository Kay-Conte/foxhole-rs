use foxhole::{
    resolve::{Resolve, ResolveGuard}, App, Captures, Http1, Method::Post, RequestState, Router
};

use std::str;

struct Body<'a>(&'a str);

impl<'a, 'b> Resolve<'b> for Body<'a> {
    type Output = Body<'b>;

    fn resolve(ctx: &'b RequestState, _path_iter: &mut Captures) -> ResolveGuard<Self::Output> {
        let body = str::from_utf8(ctx.request.body().get_as_slice()).unwrap();

        ResolveGuard::Value(Body(body))
    }
}

fn post(body: Body) -> u16 {
    println!("Body: {}", body.0);

    200
}

fn main() {
    let router = Router::new().add_route("/", Post(post));

    App::builder(router).run::<Http1>("127.0.0.1:8080");
}
