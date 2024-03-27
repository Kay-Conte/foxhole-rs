use foxhole::{
    resolve::{Get, UrlPart},
    sys, IntoResponse, Resolve, ResolveGuard, Scope,
};

fn main() {
    // /api/:id/start

    let router = Scope::new(sys![auth]).route("secretstuff", sys![secret]);
}
