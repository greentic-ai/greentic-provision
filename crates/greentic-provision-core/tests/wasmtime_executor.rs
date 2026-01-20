use std::path::PathBuf;

use greentic_provision_core::{
    ExecutionLimits, ProvisionEngine, ProvisionInputs, ProvisionMode, TenantContext,
    WasmtimeExecutor,
};
use serde_json::Value;

fn fixture_pack() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tests/fixtures/packs/noop-provision.gtpack")
}

#[test]
fn wasmtime_executor_runs_fixture_pack() {
    let pack = fixture_pack();
    let executor =
        WasmtimeExecutor::new(pack, ExecutionLimits::default()).expect("failed to create executor");
    let engine = ProvisionEngine::new(executor);

    let inputs = ProvisionInputs {
        tenant: TenantContext::default(),
        provider_id: "noop-provision".to_string(),
        install_id: "install".to_string(),
        public_base_url: None,
        answers: Value::Object(serde_json::Map::new()),
        existing_state: None,
    };

    let result = engine.run(ProvisionMode::DryRun, inputs);
    assert_eq!(
        result.plan.config_patch.get("foo"),
        Some(&Value::String("bar".to_string()))
    );
    assert!(result.plan.secrets_patch.set.contains_key("token"));
}

#[test]
fn mutation_inputs_do_not_panic() {
    let pack = fixture_pack();
    let executor =
        WasmtimeExecutor::new(pack, ExecutionLimits::default()).expect("failed to create executor");
    let engine = ProvisionEngine::new(executor);

    let mut answers = serde_json::Map::new();
    answers.insert("field".to_string(), Value::String("value".to_string()));

    let inputs = ProvisionInputs {
        tenant: TenantContext::default(),
        provider_id: "noop-provision".to_string(),
        install_id: "install".to_string(),
        public_base_url: None,
        answers: Value::Object(answers),
        existing_state: None,
    };

    let _ = engine.run(ProvisionMode::DryRun, inputs);
}
