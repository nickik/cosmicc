#!/usr/bin/env sh
# The complete, currently supported Cosmic C validation surface.
set -eu

cargo fmt --all -- --check
cargo test -p saltwater-sia --locked
cargo check --bin cosmicc --locked
cargo check --workspace --locked
