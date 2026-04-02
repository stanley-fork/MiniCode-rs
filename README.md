# Rust Workspace

This directory contains the Rust workspace scaffold for MiniCode.

## Usage

Create a new library crate:

```bash
cargo new crates/example-lib --lib
```

Create a new binary crate:

```bash
cargo new apps/example-cli --bin
```

After creating a crate, add it to `members` in `Cargo.toml`.
