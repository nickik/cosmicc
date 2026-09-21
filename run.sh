#!/bin/bash
set -euo pipefail

unset RUSTC RUSTDOC RUSTC_WRAPPER RUSTFLAGS CARGO_ENCODED_RUSTFLAGS

export RUSTC="$(rustup which --toolchain 1.98.1 rustc)"
export RUSTDOC="$(rustup which --toolchain 1.98.1 rustdoc)"

CARGO=(rustup run 1.98.1 cargo)

git update
"${CARGO[@]}" fmt --all -- --check
"${CARGO[@]}" test --workspace
"${CARGO[@]}" build

