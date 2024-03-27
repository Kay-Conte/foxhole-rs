#[cfg(feature = "websocket")]
use foxhole::{
    sys,
    websocket::{Upgrade, Websocket},
    App, Http1, Scope,
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
    let scope = Scope::new(sys![upgrade]);

    println!("Running on '127.0.0.1:8080'");

    App::builder(scope).run::<Http1>("127.0.0.1:8080");
}

#[cfg(not(feature = "websocket"))]
fn main() {
    println!("Run with \"--features websocket\"");
}
