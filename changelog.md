# [unreleased]

### Added
- Body reading example `body.rs

### Changes 
- `Resolve` trait now takes a lifetime and is capable of returning refernces
  to the state
- `Lazy` is now usable through an immutable reference and by extension
  `RequestState` is now immutable
