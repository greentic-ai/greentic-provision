#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
PACK_SRC="$ROOT_DIR/tests/fixtures/pack_src/noop-provision"
PACK_OUT_DIR="$ROOT_DIR/tests/fixtures/packs"
PACK_OUT="$PACK_OUT_DIR/noop-provision.gtpack"

mkdir -p "$PACK_SRC" "$PACK_OUT_DIR"

if command -v greentic-dev >/dev/null 2>&1; then
  (cd "$PACK_SRC" && greentic-dev pack init noop-provision) || true
  (cd "$PACK_SRC" && greentic-dev component init noop-provision-step) || true
fi

if command -v greentic-pack >/dev/null 2>&1; then
  greentic-pack build --pack "$PACK_SRC" --out "$PACK_OUT"
  greentic-pack doctor --pack "$PACK_OUT" --validate
else
  rm -rf "$PACK_OUT"
  mkdir -p "$PACK_OUT"
  cp -R "$PACK_SRC"/* "$PACK_OUT"/
  echo "greentic-pack not available; copied fixture pack source to $PACK_OUT"
fi
