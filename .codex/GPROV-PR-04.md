# GPROV-PR-04 — Provision conformance suite + fuzz/mutation tests (10x bug discovery)

REPO: greentic-ai/greentic-provision

GOAL
Prevent non-functional setup flows from shipping by adding:
- a conformance runner that executes provisioning across packs using fixtures
- fuzz/mutation tests that stress validate/apply logic
- artifact capture to reproduce failures quickly

NON-GOALS
- No live network tests by default (live mode can exist but must be gated).

DELIVERABLES

A) Conformance runner CLI
Add:
- `greentic-provision conformance --packs <dir> --report <path.json> [--provider <name>] [--live]`

For each pack:
1) discover setup entry
2) run requirements (if present)
3) run setup in dry-run using fixtures:
   - answers fixture (or empty)
   - public_base_url fixture (placeholder)
4) verify output invariants:
   - returns deterministic plan (stable keys)
   - does not include secret values in report
   - config/secrets patches are well-formed
   - subscriptions/webhook ops are well-formed if present
5) write report + per-pack logs

B) Failure artifact capture
On failure, write:
- inputs.json
- step outputs per phase
- diagnostics.json
- pack id/version
into `.greentic/provision/artifacts/<pack>/<timestamp>/`

C) Fuzz/mutation tests
Add a test suite that:
- mutates answers JSON:
  - missing fields
  - wrong types
  - extra fields
- ensures:
  - validate step rejects with actionable diagnostics
  - apply step never panics/traps
- Add “schema-guided” fuzzing if config schema is available (optional).

D) Integration into CI
Add a workflow that runs conformance on every PR:
- on a minimal fixture pack set
and nightly:
- runs across all packs in a specified directory.

ACCEPTANCE CRITERIA
- Conformance run produces a stable report across runs
- Failures produce reproducible artifacts
- Fuzz tests run within a reasonable time budget and catch panics
