#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

echo "Phase A: formatting"
cargo fmt --check

echo "Phase A: clippy"
cargo clippy --all-targets --all-features -- -D warnings

echo "Phase A: tests"
cargo test --all-features

echo "Phase A: build"
cargo build --all-features

echo "Phase B: conformance"
FIXTURE_PACK="$ROOT_DIR/tests/fixtures/packs/noop-provision.gtpack"
if [ -e "$FIXTURE_PACK" ]; then
  cargo run -p greentic-provision -- conformance --packs tests/fixtures/packs --report target/conformance.json
else
  echo "fixtures not present yet; skipping conformance"
fi
