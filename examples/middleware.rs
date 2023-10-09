use vegemite::{
    framework::run, routing::Route, sys, systems::Html, Get, MaybeIntoResponse, Resolve,
    ResolveGuard, PathIter,
};

struct Auth {
    #[allow(unused)]
    token: String,
}

impl<'a> Resolve<'a> for Auth {
    type Output = Self;

    fn resolve(ctx: &'a vegemite::RequestState, _path_iter: &mut PathIter) -> ResolveGuard<Self> {
        match ctx.request.headers().get("Authorization") {
            Some(v) => ResolveGuard::Value(Auth {
                token: v.to_str().unwrap().to_string(),
            }),
            None => ResolveGuard::Respond(401u16.maybe_response().unwrap()),
        }
    }
}

fn middleware(_auth: Auth) {
    // We could additionally do some extra work here or just use `Auth` on the endpoints like
    // `get_page.
}

fn get_page(_get: Get) -> Html {
    Html("<h1> This page is for authorized personnel only </h1>".to_string())
}

fn main() {
    // ! systems are run from left to right until a response is received from a system
    let router = Route::new(sys![middleware]).route("page", sys![get_page]);

    println!("Try connecting on a browser at 'http://localhost:8080/page'");

    run("127.0.0.1:5000", router)
}
