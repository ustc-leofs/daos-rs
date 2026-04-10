# daos-rs

`daos-rs` provides Rust bindings for the DAOS C API with two build modes:

1. Default mode (without `mock`): generates `bindings.rs` from `wrapper.h` via `bindgen` during build, and links against the system `daos` library.
2. `mock` mode: uses the in-repo `src/bindings.rs` and `src/mock_daos.rs`, so it can be built and tested without a local DAOS installation.

## Requirements

- Rust 2021
- `bindgen` (build dependency)
- In default mode, DAOS headers and `libdaos` must be available on the system

## Usage

Use it as a dependency (`package = "daos-rs"`, library name `daos`):

```toml
[dependencies]
daos = { package = "daos-rs", version = "0.1.0" }
```

Enable `mock`:

```toml
[dependencies]
daos = { package = "daos-rs", version = "0.1.0", features = ["mock"] }
```

Build examples:

```bash
# Default mode (requires local DAOS environment)
cargo build

# Mock mode (builds without local DAOS)
cargo build --features mock
```
