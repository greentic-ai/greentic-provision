# Repository Overview

## 1. High-Level Purpose
- This repository is a Rust workspace for a generic provisioning engine that runs setup “wizards” defined in Greentic packs.
- It provides a core engine, pack discovery, an apply engine, a Wasmtime-based executor for pack steps, conformance tooling, and a CLI for inspection and dry-run execution.

## 2. Main Components and Functionality
- **Path:** `Cargo.toml`
  - **Role:** Workspace configuration and shared dependency versions.
  - **Key functionality:** Declares workspace members and pins Greentic shared crates plus Wasmtime tooling.
  - **Key dependencies / integration points:** `greentic-types`, `greentic-interfaces`, `greentic-oauth-client`, `wasmtime`, `wat`, `zip`, `tempfile`.
- **Path:** `crates/greentic-provision-core`
  - **Role:** Core provisioning engine, apply adapters, executor, and domain-agnostic types.
  - **Key functionality:**
    - Defines provisioning types (`ProvisionMode`, `ProvisionStep`, `ProvisionInputs`, `ProvisionPlan`, `ProvisionResult`, `OAuthOp`).
    - Implements pack discovery from a typed `PackManifest` model.
    - Provides `ProvisionEngine` for collect/validate/apply/summary lifecycle.
    - Provides `ProvisionApplier` with in-memory stores plus optional file-backed install store and config applier helpers.
    - Implements `WasmtimeExecutor` with sandbox limits to run pack step components and merge plan patches.
  - **Key dependencies / integration points:** Uses `greentic-types` diagnostics; WIT bindings are future integration points.
- **Path:** `crates/greentic-provision-cli`
  - **Role:** CLI for inspecting packs, running dry-run setups, and conformance checks.
  - **Key functionality:**
    - `pack inspect` loads a JSON manifest and prints a provisioning descriptor.
    - `dry-run setup` executes with `--executor wasm|noop` and prints a deterministic plan.
    - `conformance` runs packs in dry-run, verifies invariants, emits a report, and captures artifacts on failure.
    - `.gtpack` archives are supported via zip extraction to a temp directory.
  - **Key dependencies / integration points:** Uses `greentic-provision-core` and `clap`.
- **Path:** `crates/greentic-provision-cli/tests/cli_smoke.rs`
  - **Role:** CLI smoke tests.
  - **Key functionality:** Validates `pack inspect` and `dry-run setup --executor noop` against fixture packs.
- **Path:** `tests/fixtures/pack_src/noop-provision`
  - **Role:** Source for a minimal noop provisioning pack.
  - **Key functionality:** Defines a manifest and WAT-based step components for collect/validate/apply/summary.
- **Path:** `tests/fixtures/packs/noop-provision.gtpack`
  - **Role:** Fixture pack used for conformance and executor tests.
  - **Key functionality:** Contains the manifest and WAT step components consumed by the Wasmtime executor.
- **Path:** `tests/fixtures/build_fixtures.sh`
  - **Role:** Fixture build helper.
  - **Key functionality:** Attempts to use `greentic-dev` and `greentic-pack` to build `.gtpack`, with a copy fallback.
- **Path:** `ci/local_check.sh`
  - **Role:** Local CI script.
  - **Key functionality:** Runs fmt, clippy, tests, build, and conformance when fixtures exist.
- **Path:** `.github/workflows/ci.yml`
  - **Role:** Local check workflow.
  - **Key functionality:** Runs `ci/local_check.sh` on PRs and pushes to `main`/`master`.
- **Path:** `.github/workflows/nightly-conformance.yml`
  - **Role:** Nightly conformance workflow.
  - **Key functionality:** Runs conformance on a schedule and via manual dispatch.
- **Path:** `README.md`
  - **Role:** Repository overview and CLI usage.
  - **Key functionality:** Documents pack inspection and dry-run usage.
- **Path:** `docs/architecture.md`
  - **Role:** Architecture notes for provisioning lifecycle and executor.
  - **Key functionality:** Describes the engine flow and determinism goals.

## 3. Work In Progress, TODOs, and Stubs
- **Location:** `crates/greentic-provision-core/src/apply.rs`
  - **Status:** partial
  - **Short description:** Apply adapters are in-memory/file-backed only; greentic-config/secrets/oauth integrations are placeholders.
- **Location:** `crates/greentic-provision-core/src/executor.rs`
  - **Status:** partial
  - **Short description:** Executor uses a minimal JSON-in/JSON-out contract and WAT compilation; component model/WIT integration is pending.
- **Location:** `crates/greentic-provision-cli/src/main.rs`
  - **Status:** partial
  - **Short description:** Pack loading relies on JSON manifests and zip-extracted `.gtpack` archives; OCI pack fetching is not implemented.

## 4. Broken, Failing, or Conflicting Areas
- None observed in the latest local run; `ci/local_check.sh` completed successfully.

## 5. Notes for Future Work
- Replace fixture WAT components with real pack components and WIT-based component model bindings.
- Add OCI pack loading once pack tooling is available.
- Wire greentic-config/secrets/oauth adapters to concrete platform services.
