use vegemite::{
    framework::run, routing::Route, sys, Get, IntoResponse, MaybeIntoResponse, Resolve,
    ResolveGuard, Response,
};

struct Html {
    value: String,
}

impl IntoResponse for Html {
    fn response(self) -> Response<Vec<u8>> {
        let bytes = self.value.into_bytes();

        Response::builder()
            .status(200)
            .header("Content-Type", "text/html; charset=utf-8")
            .header("Content-Length", format!("{}", bytes.len()))
            .body(bytes)
            .unwrap()
    }
}

struct Auth {
    #[allow(unused)]
    token: String,
}

impl Resolve for Auth {
    type Output = ResolveGuard<Self>;

    fn resolve(ctx: &mut vegemite::RequestState) -> Self::Output {
        let token = ctx.request.headers().iter().find_map(|h| {
            if h.0 == "Authorization" {
                Some(h.1.to_str().unwrap().to_string())
            } else {
                None
            }
        });

        match token {
            Some(v) => ResolveGuard::Value(Auth { token: v }),
            None => ResolveGuard::Respond(401u16.maybe_response().unwrap()),
        }
    }
}

fn middleware(_auth: Auth) {
    // We could additionally do some extra work here or just use `Auth` on the endpoints like
    // `get_page.
}

fn get_page(_get: Get) -> Html {
    Html {
        value: "<h1> This page is for authorized personnel only </h1>".to_string(),
    }
}

fn main() {
    // ! systems are run from left to right until a response is received from a system
    let router = Route::new(sys![middleware]).route("page", sys![get_page]);

    run("127.0.0.1:5000", router)
}
