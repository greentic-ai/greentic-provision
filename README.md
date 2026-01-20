# greentic-provision

`greentic-provision` is a Rust workspace for running provider setup "wizards" contained in Greentic packs.
It focuses on a generic provisioning lifecycle (collect, validate, apply, summary) and supports dry-run
planning for deterministic reports.

## Crates
- `greentic-provision-core`: domain-agnostic engine, types, and pack discovery.
- `greentic-provision` (CLI): thin CLI for inspecting packs and running dry-runs.

## CLI usage
```bash
# Inspect a pack manifest (expects a JSON manifest or a directory containing pack.json/manifest.json)
greentic-provision pack inspect --pack ./path/to/pack.json

# Dry-run a setup flow (answers.json is optional)
greentic-provision dry-run setup \
  --pack ./path/to/pack.json \
  --provider-id provider-x \
  --install-id install-123 \
  --public-base-url https://example.com \
  --answers ./answers.json \
  --json
```

## Notes
- Pack loading is JSON-only for now; `.gtpack` archives are expected to be unpacked before use.
- Provision execution uses a `NoopExecutor` in PR-01; step execution is wired in PR-03.
