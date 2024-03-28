# [unreleased]

### Added
- New `Router` type and implementation
- Fallback handler on `Router`
- Websocket support under feature flag `websocket`

### Removed
- All method guards, use `foxhole::Method` now in combination with the new `Router
- `Scope` in favor of `Router`

# [0.4.0]

### Added
- `Layer` trait and `LayerGroup` structure.
- `SetContentLength` Layer
- `SetDate` Layer
- `DefaultResponseLayer` layer group
- Https/Tls support under feature flag 'tls'
- Http1 connection handler
- Various new `Resolve` and `Response` types
  - `&[u8]` is now resolve, representing the raw body of the request 
  - `&str` is now resolve, representing the str representation of the request
  - `Raw` a response with the `application/x-binary` content type hint
  - `Css` a response with the `text/css` content type hint
  - `Js` a response with the `text/javascript` content type hint

### Changed
- Renamed `Route` to `Scope`
- The `TypeCache` is no longer behind an `RwLock`
- Changed `framework` to a builder pattern
- Renamed `MaybeIntoResponse` to `Action`
- Refactored various items into modules `resolve` and `action` exports to related items have changed
- `run` and `run_with_cache` now take a generic implementing `Connection` used as the tcpstream handler.

### Fixed
- A bug where requests would sometimes not be handled until another request was received.

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
