use super::*;

#[test]
fn test_req_parsing() {
    let bytes = b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n";

    let req = Request::try_from();

    assert!(req.is_ok());
    let req = req.unwrap();
    assert_eq!(req.method, Method::GET);
    assert_eq!(req.path, "/");
    assert_eq!(req.version, Version::HTTP_1_1);
    assert_eq!(req.headers.len(), 1);
    assert_eq!(req.headers.get("Host"), Some(&"localhost".to_string()));
}
