use foxhole::{Route, systems::{Upgrade, WebsocketHandler}, sys, run, websocket::Frame};

fn upgrade(ws: Upgrade) -> WebsocketHandler {
    ws.upgrade(|ws| { 
        for frame in ws {
            let text = match frame {
                Frame::Text(c) => String::from_utf8(c).unwrap(),
                _ => continue,
            };

            println!("{}", text);
        }
    })
}

fn main() {
    let router = Route::new(sys![upgrade]);

    run("0.0.0.0:8080", router);
}
