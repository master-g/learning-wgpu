#!/usr/bin/env bash

set -e

RUSTFLAGS='--cfg getrandom_backend="wasm_js"' cargo build --no-default-features --features web_rwh --target wasm32-unknown-unknown

# Generate binding
wasm-bindgen --no-typescript --out-dir wasm --web "../../../target/wasm32-unknown-unknown/debug/wgpu_in_web.wasm"

cp wasm/wgpu_in_web.js public/wgpu_in_web.js
cp wasm/wgpu_in_web_bg.wasm public/wgpu_in_web_bg.wasm

cp -r assets public/

basic-http-server public

