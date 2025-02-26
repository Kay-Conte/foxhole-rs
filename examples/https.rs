#[cfg(feature = "tls")]
use std::io::BufReader;

#[cfg(feature = "tls")]
use foxhole::{action::Html, App, Http1, Method::Get, Router};

#[cfg(feature = "tls")]
use rustls::ServerConfig;

// ! These are dummy files. Replace them with real cert and key.
#[cfg(feature = "tls")]
const CERT_FILE: &[u8] = include_bytes!("./auth/cert.pem");

#[cfg(feature = "tls")]
const KEY_FILE: &[u8] = include_bytes!("./auth/key.pem");

#[cfg(feature = "tls")]
fn get() -> Html {
    Html("<h1> Foxhole! </h1>".to_string())
}

#[cfg(feature = "tls")]
fn main() {
    let router = Router::new().add_route("/", Get(get));

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

    App::builder(router)
        .tls_config(config)
        .run::<Http1>("127.0.0.1:8080");
}

#[cfg(not(feature = "tls"))]
fn main() {
    println!("Run with `--features \"tls\"`");
}
