#!/usr/bin/env sh
# The complete, currently supported Cosmic C validation surface.
set -eu

cargo fmt --all -- --check
cargo test --workspace --locked
cargo build --locked
cargo check --workspace --locked
