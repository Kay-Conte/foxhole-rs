#[cfg(feature = "websocket")]
use foxhole::{
    websocket::{Upgrade, Websocket},
    App, Http1,
    Method::Get,
    Router,
};

#[cfg(feature = "websocket")]
fn upgrade(upgrade: Upgrade) -> Websocket {
    use std::io::ErrorKind;

    upgrade.handle(|mut ws| loop {
        match ws.next_frame() {
            Ok(v) => {
                let _ = ws.send(v);
            }
            Err(e) if e.kind() == ErrorKind::WouldBlock => {}
            Err(_) => return,
        }
    })
}

#[cfg(feature = "websocket")]
fn main() {
    let router = Router::new().add_route("/", Get(upgrade));

    println!("Running on '127.0.0.1:8080'");

    App::builder(router).run::<Http1>("127.0.0.1:8080");
}

#[cfg(not(feature = "websocket"))]
fn main() {
    println!("Run with \"--features websocket\"");
}
