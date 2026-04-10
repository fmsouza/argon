#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN="${ARGON_BIN:-$ROOT/target/fast-release/argon}"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

if [[ -z "${ARGON_BIN:-}" ]]; then
  echo "Building Argon CLI with the fast-release profile..."
  cargo build --profile fast-release -p argon-cli >/dev/null
elif [[ ! -x "$BIN" ]]; then
  echo "ARGON_BIN does not point to an executable: $BIN" >&2
  exit 1
fi

WASM_SUBSET="$ROOT/examples/wasm-subset.arg"
MODULES_MAIN="$ROOT/examples/modules/main.arg"

bench() {
  local label="$1"
  shift

  echo
  echo "== $label =="
  /usr/bin/time -p "$@"
}

bench "check (20x)" env BIN="$BIN" FILE="$WASM_SUBSET" bash -lc '
  for i in $(seq 1 20); do
    "$BIN" check "$FILE" >/dev/null
  done
'

bench "compile js (20x)" env BIN="$BIN" FILE="$WASM_SUBSET" OUT="$TMP_DIR/out.js" bash -lc '
  for i in $(seq 1 20); do
    "$BIN" compile "$FILE" --target js -o "$OUT" >/dev/null
  done
'

bench "compile wasm (20x)" env BIN="$BIN" FILE="$WASM_SUBSET" OUT="$TMP_DIR/out.wasm" bash -lc '
  for i in $(seq 1 20); do
    "$BIN" compile "$FILE" --target wasm -o "$OUT" >/dev/null
  done
'

bench "compile native obj (20x)" env BIN="$BIN" FILE="$WASM_SUBSET" OUT="$TMP_DIR/out.o" bash -lc '
  for i in $(seq 1 20); do
    "$BIN" compile "$FILE" --target native --emit obj -o "$OUT" >/dev/null
  done
'

bench "compile native exe (10x)" env BIN="$BIN" FILE="$WASM_SUBSET" OUT="$TMP_DIR/out" bash -lc '
  for i in $(seq 1 10); do
    "$BIN" compile "$FILE" --target native -o "$OUT" >/dev/null
  done
'

bench "compile project js (20x)" env BIN="$BIN" FILE="$MODULES_MAIN" DIST="$TMP_DIR/dist" bash -lc '
  for i in $(seq 1 20); do
    rm -rf "$DIST"
    "$BIN" compile "$FILE" --target js --out-dir "$DIST" >/dev/null
  done
'

bench "run (20x)" env BIN="$BIN" FILE="$WASM_SUBSET" bash -lc '
  for i in $(seq 1 20); do
    "$BIN" run "$FILE" >/dev/null
  done
'
