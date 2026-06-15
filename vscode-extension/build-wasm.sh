#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
WASM_DIR="$SCRIPT_DIR/wasm"
CRATE_DIR="$(cd "$SCRIPT_DIR/../next_kaubo/crates/kaubo-wasm" && pwd)"
WORKSPACE_ROOT="$(cd "$SCRIPT_DIR/../next_kaubo" && pwd)"

echo "[kaubo-vscode] Building WASM for Node.js target..."

cd "$WORKSPACE_ROOT"
wasm-pack build "$CRATE_DIR" --target nodejs --out-dir "$WASM_DIR" --out-name kaubo_wasm

echo "[kaubo-vscode] WASM built to $WASM_DIR"
ls -la "$WASM_DIR/"
