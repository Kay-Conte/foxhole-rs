use foxhole::{
    resolve::{Post, Resolve, ResolveGuard},
    sys, App, Http1, PathIter, RequestState, Scope,
};

use std::str;

struct Body<'a>(&'a str);

impl<'a, 'b> Resolve<'b> for Body<'a> {
    type Output = Body<'b>;

    fn resolve(ctx: &'b RequestState, _path_iter: &mut PathIter) -> ResolveGuard<Self::Output> {
        let body = str::from_utf8(ctx.request.body().get_as_slice()).unwrap();

        ResolveGuard::Value(Body(body))
    }
}

fn post(_post: Post, body: Body) -> u16 {
    println!("Body: {}", body.0);

    200
}

fn main() {
    let scope = Scope::new(sys![post]);

    App::builder(scope).run::<Http1>("127.0.0.1:8080");
}
