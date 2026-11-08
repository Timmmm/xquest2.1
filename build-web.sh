#!/bin/sh
# Builds the web version into web/: needs the wasm32-unknown-unknown target and a
# wasm-bindgen-cli whose version matches the wasm-bindgen crate in Cargo.lock.
set -e
cargo build --profile wasm-release --target wasm32-unknown-unknown --bin xquest
wasm-bindgen --target web --no-typescript --out-dir web target/wasm32-unknown-unknown/wasm-release/xquest.wasm
echo "Serve the web/ directory, for example: python3 -m http.server -d web"
