# GPROV-PR-02 — Add integrations: config, secrets, oauth, subscriptions adapters (apply engine)

REPO: greentic-ai/greentic-provision

GOAL
Implement the “apply” side of provisioning:
- take a `ProvisionPlan`
- apply it via platform services (greentic-config, greentic-secrets, greentic-oauth)
- persist a Provider Installation Record (in a store abstraction)
- support dry-run (no writes) and apply (writes)

NON-GOALS
- Do not implement provider-specific logic here.
- Do not implement network calls to provider APIs here (packs may request webhook ops, but the host decides how/when to execute them; keep them as ops in plan for now unless a host service exists).

DELIVERABLES

A) Provider installation record persistence
- Use `greentic-types` ProviderInstallRecord model (if already added) OR define equivalent in this repo temporarily.
- Add trait `InstallStore`:
  - `get(tenant_ctx, provider_id, install_id) -> Option<Record>`
  - `put(record)`
  - `list(tenant_ctx) -> Vec<Record>`
  - `delete(...)`
- Provide an `InMemoryInstallStore` for tests and dev.
- Optionally a file-backed store under `.greentic/provision/installs.json` (dev-only).

B) Config adapter
- Integrate `greentic-config`:
  - apply `config_patch` to a per-install config namespace:
    `provision:{env}:{tenant}:{team}:{provider_id}:{install_id}`
  - ensure consistent keying and avoids collisions
- Expose a `ConfigApplier` that can:
  - `plan_only`: show what keys change
  - `apply`: write changes

C) Secrets adapter
- Integrate `greentic-secrets`:
  - apply `secrets_patch` (set/delete)
  - secrets values must not be printed in logs
  - plan output should redact values by default
- Ensure “no env var reading” in this library; secrets come from plan inputs only.

D) OAuth adapter
- Integrate `greentic-oauth`:
  - Provide a helper for “oauth start/finish” but do not build a webserver here.
  - The engine should support:
    - `ProvisionPlan` can request `oauth_op: Start(provider, scopes, redirect_url)`
    - store the resulting token/refresh token in secrets store using standard keys
- Keep OAuth ops as part of plan unless the domain app executes them.

E) Subscriptions adapter
- Provide types and storage for subscriptions state:
  - subscription id, resource, expiry, last_sync
- Apply subscription ops to the install record (state), but do not call external APIs here unless a host service exists.
- Provide a hook point so domain apps can run the external calls and report results back.

F) API surface
Expose from `greentic-provision-core` (or a new crate `greentic-provision-runtime`):
- `ProvisionApplier::apply(result: ProvisionResult, mode: ApplyMode) -> ApplyReport`
Where ApplyMode = DryRun | Apply.

G) Tests
- Apply config patch updates store keys correctly
- Secrets patch sets/deletes with redaction in logs
- Install record persisted and can be retrieved

ACCEPTANCE CRITERIA
- End-to-end dry-run + apply works using in-memory stores
- Install records are created/updated deterministically
- No secret values printed
