use turnip_http::{
    framework::run,
    routing::Route,
    systems::Get, sys,
};

fn get(_get: Get) -> u16 {
    200
}

fn main() {
    let router = Route::new(sys![get]);

    run("0.0.0.0:5000", router);
}
