use turnip_http::{
    framework::run,
    routing::Node,
    systems::Get, sys,
};


fn get(_get: Get) -> u16 {
    println!("Got request");

    200
}

fn main() {
    let router = Node::empty().route(
        "hello_world", 
        Node::new(sys![get]) 
    );

    run("0.0.0.0:5000", router);
}
