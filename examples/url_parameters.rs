use foxhole::{
    action::Html,
    resolve::{ArgMap, UrlCollect, UrlPart},
    App, Http1,
    Method::Get,
    Router,
};

fn user(UrlPart(part): UrlPart) -> Html {
    Html(part)
}

fn collect(UrlCollect(after): UrlCollect) -> Html {
    Html(after.join(""))
}

// /query?q=hello
fn query(ArgMap(map): ArgMap) -> Option<Html> {
    map.get("q").map(|i| Html(i.to_string()))
}

fn main() {
    let router = Router::new()
        .add_route("/user/:username", Get(user))
        .add_route("/collect/*", Get(collect))
        .add_route("/query", Get(query));

    println!("Try connecting on a browser at 'http://localhost:8080/user/USERNAME'");

    let res = App::builder(router).run::<Http1>("0.0.0.0:8080");

    if let Err(e) = res {
        println!("{e:?}");
    };
}
