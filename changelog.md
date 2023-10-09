# [0.3.0]

### Added
- Body reading example `body.rs`

### Removed
- Cargo.lock is no longer synced

### Changes 
- `Vegemite` is now `Foxhole`
- `Resolve` trait now takes a lifetime and is capable of returning refernces
  to the state
- `Lazy` is now usable through an immutable reference and by extension
  `RequestState` is now immutable
