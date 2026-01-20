# GPROV-PR-01 — Create `greentic-provision` core engine (generic setup runner)

REPO: greentic-ai/greentic-provision (new repo)

GOAL
Create a reusable provisioning engine that can run provider setup “wizards” contained in packs (messaging/events/secrets provider extension packs), without being domain-specific.

The engine must support:
- discovering a pack’s provisioning/setup entry
- executing a standardized wizard lifecycle: collect -> validate -> apply -> summary
- dry-run mode that produces a deterministic “plan” and never performs network writes
- producing a stable report (Diagnostics + Plan) for CI and tooling

NON-GOALS
- Do not implement any messaging/events/secrets provider specifics.
- Do not implement cloudflared or dev UX here (belongs in domain CLIs).
- Do not fetch packs from OCI in this PR (local path support only; OCI later or in greentic-pack doctor).

DELIVERABLES

A) Repository + workspace layout
Create repo skeleton with:
- `crates/greentic-provision-core/`
- `crates/greentic-provision-cli/` (thin CLI for debugging; optional but recommended)
- `crates/greentic-provision-types/` (optional; prefer greentic-types if already contains needed structs)
- `README.md` describing purpose and usage
- `docs/architecture.md` (1–2 pages, “how it works”)

B) Core API (library)
In `greentic-provision-core`, implement:

1) Domain-agnostic types:
- `ProvisionMode`: `Install | Update | Delete | DryRun`
- `ProvisionStep`: `Collect | Validate | Apply | Summary`
- `ProvisionInputs`:
  - tenant context (env/tenant/team, optional user)
  - provider_id + install_id (string)
  - public_base_url (optional string; may be required by some packs)
  - answers (JSON object)
  - existing state snapshot (JSON object) (optional)
- `ProvisionPlan` (deterministic, pure output):
  - `config_patch` (JSON merge-patch or JSON Patch; pick one and document)
  - `secrets_patch` (set/delete ops; values redacted in plan by default)
  - `webhook_ops` (register/update/delete desired endpoints)
  - `subscription_ops` (sync/lifecycle operations)
  - `notes` (strings)
- `ProvisionResult`:
  - `plan: ProvisionPlan`
  - `diagnostics: Vec<greentic_types::validate::Diagnostic>`
  - `step_results: Vec<StepResult>` (optional; for debug)
- Ensure these are serde-serializable with stable field names.

2) Pack discovery contract (minimal, pragmatic)
Add a trait:
- `trait ProvisionPackDiscovery { fn discover(pack: &PackManifest) -> Option<ProvisionDescriptor>; }`

Where `ProvisionDescriptor` includes:
- `pack_id`, `pack_version`
- `setup_entry_flow` (string flow id)
- optional `requirements_flow` (string)
- optional `subscriptions_flow` (string or op)
- declared inputs: `requires_public_base_url: bool` (best-effort)
- declared outputs: capabilities used (config/secrets/oauth/subscriptions/http)

Discovery rules (start simple):
- If pack meta.entry_flows contains "setup", treat that as provisioning entry.
- Else if any flow has entry == "setup", treat it as provisioning entry.
- Else return None.

DO NOT hardcode messaging/events names; rely only on generic entry flow names.

3) Core runner orchestrator (no wasm execution yet in this PR)
Implement a “runner” that can orchestrate a wizard even if steps are implemented as flows:
- `ProvisionEngine::plan_from_fixtures(...)`:
  - loads fixture JSON files for collect/validate/apply/summary outputs (for tests)
- `ProvisionEngine::run(...)`:
  - for now, this can call a pluggable executor trait (implemented in PR-03)

Define:
- `trait ProvisionExecutor`:
  - `fn run_step(&self, step: ProvisionStep, ctx: &ProvisionContext) -> StepOutput`
- `StepOutput` is JSON + diagnostics + optional patches

In this PR, implement a `NoopExecutor` that returns empty outputs so you can wire the engine and CLI without Wasmtime yet.

C) CLI (optional but recommended)
`greentic-provision` CLI:
- `greentic-provision pack inspect --pack <path.gtpack>`: prints discovered provisioning descriptor
- `greentic-provision dry-run setup --pack <path.gtpack> --provider-id ... --install-id ... --public-base-url ... --answers answers.json`
- outputs:
  - human summary
  - `--json` full `ProvisionResult`

D) Tests (must exist)
- Unit tests for discovery:
  - pack with entry_flows includes setup -> discover ok
  - pack without setup -> None
- Unit tests for deterministic plan serialization:
  - stable ordering and deterministic output (use BTreeMap)
- CLI tests (smoke) if repo has harness; otherwise unit tests only.

ACCEPTANCE CRITERIA
- Repo builds and `cargo test` passes
- `greentic-provision pack inspect --pack messaging-telegram.gtpack` (or any pack with setup entry) prints a descriptor (manual check)
- Dry-run produces a deterministic JSON output shape (even if empty patches in PR-01)
