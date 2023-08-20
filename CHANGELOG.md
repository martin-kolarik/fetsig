# Changelog

All notable changes to this project will be documented in this file. The
format is based on [Keep a Changelog], and this project aims to follow
[Semantic Versioning].

## [0.1.4] - 2023-08-20

### Added

- Signals to `Messages`.
- Iterating for `FileList`.
- `Display` for `MediaType`.

### Changed

- `tracing` feature removed, `log` left. `log` can easily be chained with `tracing`.
- `futures-timeout` removed as it depends on `async_io` which does not suppport `wasm`, own `timeout` and `TimeoutFuture` added as a replacement.
