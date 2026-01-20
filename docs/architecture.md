# greentic-provision Architecture

## Overview
`greentic-provision` is a generic provisioning engine for Greentic packs. Packs define setup flows,
and this engine orchestrates the standardized lifecycle:

1. Collect
2. Validate
3. Apply
4. Summary

The engine is deliberately domain-agnostic. Provider-specific behavior lives inside packs, not the
engine itself.

## Core concepts

### ProvisionInputs
Inputs carry tenant context, identifiers, optional public base URL, and the answers/state payloads
used by provisioning steps. These inputs are passed to each step through a `ProvisionContext`.

### ProvisionPlan
Provisioning produces a deterministic plan that can be serialized for CI tooling. The plan describes
config patches, secret operations, webhook/subscription operations, and human-readable notes.

### Pack discovery
The engine discovers a pack's provisioning entry flow from its manifest. Discovery is intentionally
minimal in PR-01:
- If `meta.entry_flows` defines `setup`, use that flow.
- Otherwise, search for a flow with `entry == "setup"`.

### Execution
PR-01 wires a `ProvisionExecutor` interface that runs each step. A `NoopExecutor` is used initially
so the engine and CLI can be exercised without WebAssembly execution. PR-03 adds a Wasmtime-based
executor.

## Determinism
The plan is built from `BTreeMap`-backed structures to keep serialization order stable. Secrets are
redacted in the plan by default to prevent leaking sensitive data.
