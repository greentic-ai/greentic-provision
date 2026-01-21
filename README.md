# greentic-provision

`greentic-provision` is a Rust workspace for running provider setup "wizards" contained in Greentic packs.
It focuses on a generic provisioning lifecycle (collect, validate, apply, summary) and supports dry-run
planning for deterministic reports.

## Crates
- `greentic-provision-core`: domain-agnostic engine, types, and pack discovery.
- `greentic-provision` (CLI): thin CLI for inspecting packs and running dry-runs.

## CLI usage
```bash
# Inspect a pack manifest (JSON manifest, directory, or .gtpack archive)
greentic-provision pack inspect --pack ./path/to/pack.json

# Dry-run a setup flow (answers.json is optional)
greentic-provision dry-run setup \
  --pack ./path/to/pack.json \
  --executor wasm \
  --provider-id provider-x \
  --install-id install-123 \
  --public-base-url https://example.com \
  --answers ./answers.json \
  --json
```

## Notes
- `.gtpack` archives are supported via zip extraction.
- Use `--executor noop` to run without Wasm execution; `--executor wasm` runs the pack components.
