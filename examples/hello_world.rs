use vegemite::{run, sys, Get, Route, Post};

// The get parameter of this function ensures that it is only run when the request is a get
// request. Note that parameters are evaluated from left to right. If a parameter returns
// `ResolveGuard::None` or `ResolveGuard::Respond` early, subsequent parameters will not be
// evaluated.
fn get(_get: Get) -> u16 {
    // u16 implements `MaybeIntoResponse` such that the framework can resolve a response from it
    200
}


// This one only runs on POST
fn post(_post: Post) -> u16 {
    200
}

fn main() {
    // systems are executed from the root to the endpoint and from left to right of each node until
    // a response is formed. 
    let router = Route::new(sys![get, post]);

    run("0.0.0.0:5000", router);
}
