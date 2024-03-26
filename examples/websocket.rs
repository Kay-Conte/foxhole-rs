use foxhole::{
    sys,
    websocket::{Upgrade, Websocket},
    App, Http1, Scope,
};

fn get(upgrade: Upgrade) -> Websocket {
    upgrade.handle(|mut ws| {
        while let Ok(frame) = ws.next_frame() {
            println!("{:?}", frame);
        }
    })
}

fn main() {
    let scope = Scope::new(sys![get]);

    println!("Running on '127.0.0.1:8080'");

    App::builder(scope).run::<Http1>("127.0.0.1:8080");
}
