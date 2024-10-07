# Changelog

All notable changes to this project will be documented in this file. The
format is based on [Keep a Changelog], and this project aims to follow
[Semantic Versioning].

## [0.3.2] - 2024-10-07

### Changed

- bump up dependencies versions

## [0.3.1] - 2024-08-31

### Changed

- `web_sys::RequestInit` deprecations fixed

## [0.3.0] - 2024-07-20

### Changed

- `Bytes` removed, serialization API changed to use plain `Vec<u8>`

## [0.2.9] - 2024-07-14

### Changed

- bump up versions

## [0.2.7] - 2024-05-08

### Changed

- bump up versions

## [0.2.6] - 2024-04-15

### Changed

- final `CollectionStore::load_merge` `merge_fn` callback behavior: the callback is called only if response
succeeds and the collection is present in the response (if the collection is not present, then the response
is/may be considered incorrect.)

## [0.2.5] - 2024-04-15

### Changed

- addition, `CollectionStore::load_merge` merge function takes `StatusCode` parameter with the result of the call

### Fixed

- storing received collection does not clear existing collection if the collection is missing in the response

## [0.2.4] - 2024-04-13

### Changed

- `CollectionStore::load_merge` merge function takes `Option<Vec>`, `None` is used when succeeded response does not contain collection

## [0.2.3] - 2024-04-03

### Changed

- bump up versions

## [0.2.2] - 2024-03-23

### Changed

- revert of removal of `set_transfer_state`

## [0.2.1] - 2024-03-23

### Changed

- bump up versions

### Changed

## [0.2.0] - 2024-03-10

### Changed

- refactoring of reset functions in both entity and collection stores

## [0.1.10] - 2024-02-29

### Changed

- bump up versions

## [0.1.9] - 2024-02-29

### Changed

- some asynchronous code extracted to artwrap crate

## [0.1.8] - 2024-02-27

### Changed

- bump up dependencies versions

## [0.1.7] - 2024-02-14

### New

- `CollectionStore::load_merge` added

- `Messages` switched to `SmolStr`
- messages in `Messages` extended with parameters
- message type `Section` introduced
- `Messages::localize` and `Message::localize` added
- `Messages::extend` added

### Changed

- `New` and `Dirty` traits have `Sized` bound moved to places where it has sense
- `StatusCode::Undefined` made private, interface of `TransferState` changed to use `Option`

### Fixes

- flattening of messages in `EntityResponse` made manual in order to allow postcard serializer to determine collection size

## [0.1.6] - 2023-11-15

### New

- `MediaType::Xlsx` added

### Changed

- bounds on loading methods simplified, `Clone` is not mandatory for `E`

## [0.1.5] - 2023-10-23

### New

- `Messages::anything_for_key_signal` added
- `collection_state_from_vec` added

### Changed

- API to insert to sorted collections changed, `cmp` function used instead of key
- some `CollectionStore` associated functions moved to underlying crate

### Fixes

- `Messages::error` is recalculated when key is removed

## [0.1.4] - 2023-08-20

### Added

- Signals to `Messages`.
- Iterating for `FileList`.
- `Display` for `MediaType`.

### Changed

- `tracing` feature removed, `log` left. `log` can easily be chained with `tracing`.
- `futures-timeout` removed as it depends on `async_io` which does not suppport `wasm`, own `timeout` and `TimeoutFuture` added as a replacement.
