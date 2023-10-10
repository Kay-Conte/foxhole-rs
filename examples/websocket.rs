use foxhole::{Route, systems::{ConnectionUpgrade, AcceptConnectionUpgrade}, sys, run};

fn upgrade(ws: ConnectionUpgrade) -> AcceptConnectionUpgrade {
    ws.upgrade(|_ws| { 
        println!("Websocket connected successfully");
    })
}

fn main() {
    let router = Route::new(sys![upgrade]);

    run("0.0.0.0:8080", router);
}
