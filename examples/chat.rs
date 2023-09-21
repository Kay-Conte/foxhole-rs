use http::Response;
use turnip_http::{framework::run, routing::Route, sys, systems::Get, http_utils::ResponseExt};

fn get(_get: Get) -> Response<()> {
    Response::<()>::base(200.try_into().unwrap())
}

fn main() {
    let router = Route::empty().route("api", Route::empty().route("users", Route::new(sys![])));

    run("0.0.0.0:5000", router);
}
