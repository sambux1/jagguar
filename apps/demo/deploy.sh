#!/usr/bin/env bash
# Build the wasm into ./pkg, then serve this folder with Caddy.
set -e
cd "$(dirname "$0")"

wasm-pack build ../../jagguar-wasm --target web --out-dir ../apps/demo/pkg
caddy run
