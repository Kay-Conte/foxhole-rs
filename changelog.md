# [unreleased]

### Added
- Various new `Resolve` and `Response` types
  - `&Vec<u8>` is now resolve, representing the raw body of the request 
  - `&str` is now resolve, representing the str representation of the request
  - `Raw` a response with the `application/x-binary` content type hint
  - `Css` a response with the `text/css` content type hint
  - `Js` a response with the `text/javascript` content type hint

### Changed
- Renamed `MaybeIntoResponse` to `Action`
- Refactored various items into modules `resolve` and `action` exports to related items have changed

# [0.3.0]

### Added
- Body reading example `body.rs`

### Removed
- Cargo.lock is no longer synced

### Changed 
- `Vegemite` is now `Foxhole`
- `Resolve` trait now takes a lifetime and is capable of returning refernces
  to the state
- `Lazy` is now usable through an immutable reference and by extension
  `RequestState` is now immutable
