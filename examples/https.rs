use std::io::BufReader;

use foxhole::{action::Html, connection::Http1, resolve::Get, run, sys, Scope};
use rustls::ServerConfig;

// ! These are dummy files. Replace them with real cert and key.
const CERT_FILE: &[u8] = include_bytes!("./auth/cert.pem");
const KEY_FILE: &[u8] = include_bytes!("./auth/key.pem");

fn get(_get: Get) -> Html {
    Html("<h1> Foxhole </h1>".to_string())
}

#[cfg(feature = "tls")]
fn main() {
    use foxhole::routing::Router;

    let scope = Scope::new(sys![get]);

    let cert_chain = rustls_pemfile::certs(&mut BufReader::new(CERT_FILE))
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    let private_key = rustls_pemfile::private_key(&mut BufReader::new(KEY_FILE))
        .unwrap()
        .unwrap();

    let config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(cert_chain, private_key)
        .unwrap();

    println!("Running on localhost:8080");

    run::<Http1>("127.0.0.1:8080", Router::new(scope), config);
}

#[cfg(not(feature = "tls"))]
fn main() {
    println!("Run with `--features \"tls\"`");
}
