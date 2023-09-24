use vegemite::{routing::Route, sys, framework::run};

fn middleware() {
    
}

fn main() {

    // ! systems are run from left to right until a response is received from a system
    let router = Route::new(sys![middleware]);

    run("127.0.0.1:5000", router)
}
