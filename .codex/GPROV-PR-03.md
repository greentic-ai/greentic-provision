# GPROV-PR-03 — Wasmtime executor for provisioning flows in packs (sandboxed)

REPO: greentic-ai/greentic-provision

GOAL
Implement an executor that can run provisioning steps defined inside packs as WASM components/flows, in a strict sandbox.

This PR makes provisioning actually execute pack-defined steps (collect/validate/apply/summary) instead of using NoopExecutor.

NON-GOALS
- No network calls by default
- No filesystem access by default
- No provider-specific host logic

DELIVERABLES

A) Define execution contract
Decide the minimal runtime contract between engine and pack step:
- Input: JSON object (ProvisionInputs + step name + prior outputs)
- Output: JSON object containing:
  - diagnostics[]
  - optional plan patches (config/secrets/webhook/subscription)
  - optional next questions (Adaptive Card payload) for collect step

If you already have a standard “flow execution” contract in Greentic (e.g., runner context), reuse it.

B) Wasmtime component execution
- Use Wasmtime component model (wasm32-wasip2).
- Load pack `.gtpack`, locate step components and/or flow graph.
- Provide host capabilities via WIT:
  - config/secrets/oauth/subscriptions interfaces should be mocked in dry-run mode
  - in apply mode, these calls should be routed to the adapters created in PR-02
- Implement resource limits:
  - memory limit
  - timeout (epoch interruption)
  - max output size

C) Step resolution
Given a pack with a setup entry flow:
- Determine which components correspond to collect/validate/apply/summary steps.
- Support two patterns:
  1) Explicit step components named `setup_default__collect`, etc.
  2) Single setup flow that internally decides step based on input `step` field
Start with whatever is already common in your provider packs (Telegram pack shows explicit step modules).

D) Dry-run behavior
- In dry-run mode:
  - host interfaces do not write persistent state
  - network is disabled
  - executor must still produce patches/ops for the plan

E) Tests
- Add a minimal “noop validator pack” or “noop provisioning pack” fixture:
  - collect returns a static question set
  - validate accepts
  - apply emits a small config patch and secret patch
  - summary returns a message
- Test that provisioning run returns expected plan and diagnostics.

ACCEPTANCE CRITERIA
- `greentic-provision dry-run setup --pack <pack.gtpack>` runs real WASM steps and returns a plan.
- No filesystem/network by default.
- Timeouts prevent hangs.
