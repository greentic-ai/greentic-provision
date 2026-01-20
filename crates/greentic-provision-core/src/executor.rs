use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};
use wasmtime::{Config, Engine, Instance, MemoryAccessError, Module, Store};
use wasmtime::{StoreLimits, StoreLimitsBuilder};

use crate::engine::{ProvisionContext, ProvisionExecutor};
use crate::types::{ProvisionPlanPatch, ProvisionStep, StepOutput};

#[derive(Debug, Clone)]
pub struct ExecutionLimits {
    pub max_output_bytes: usize,
    pub memory_limit_bytes: usize,
    pub timeout_ms: u64,
    pub fuel: u64,
}

impl Default for ExecutionLimits {
    fn default() -> Self {
        Self {
            max_output_bytes: 64 * 1024,
            memory_limit_bytes: 8 * 1024 * 1024,
            timeout_ms: 500,
            fuel: 10_000,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ExecutorError {
    #[error("component not found for step: {0}")]
    ComponentNotFound(String),
    #[error("failed to read component: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to compile component: {0}")]
    Compile(#[from] wasmtime::Error),
    #[error("failed to parse wat: {0}")]
    Wat(#[from] wat::Error),
    #[error("memory access error: {0}")]
    Memory(#[from] MemoryAccessError),
    #[error("execution trap: {0}")]
    Trap(String),
    #[error("output too large: {0} bytes")]
    OutputTooLarge(usize),
    #[error("input too large: {0} bytes")]
    InputTooLarge(usize),
    #[error("invalid output JSON: {0}")]
    OutputJson(#[from] serde_json::Error),
}

#[derive(Debug, Clone)]
pub struct WasmtimeExecutor {
    pack_root: PathBuf,
    limits: ExecutionLimits,
}

impl WasmtimeExecutor {
    pub fn new(
        pack_root: impl Into<PathBuf>,
        limits: ExecutionLimits,
    ) -> Result<Self, ExecutorError> {
        let pack_root = pack_root.into();
        if !pack_root.exists() {
            return Err(ExecutorError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "pack root not found",
            )));
        }
        Ok(Self { pack_root, limits })
    }

    pub fn run_named_step(
        &self,
        step_name: &str,
        ctx: &ProvisionContext,
    ) -> Result<StepOutput, ExecutorError> {
        let component_path = self.resolve_component(step_name)?;
        let output_json = self.execute_component(&component_path, step_name, ctx)?;
        step_output_from_json(output_json)
    }

    fn resolve_component(&self, step_name: &str) -> Result<PathBuf, ExecutorError> {
        let candidates = vec![
            format!("setup_default__{}", step_name),
            "setup_default".to_string(),
        ];
        let roots = [
            self.pack_root.join("components"),
            self.pack_root.join("wasm"),
            self.pack_root.clone(),
        ];

        for candidate in candidates {
            for root in &roots {
                let wasm = root.join(format!("{}.wasm", candidate));
                if wasm.exists() {
                    return Ok(wasm);
                }
                let wat = root.join(format!("{}.wat", candidate));
                if wat.exists() {
                    return Ok(wat);
                }
            }
        }

        Err(ExecutorError::ComponentNotFound(step_name.to_string()))
    }

    fn execute_component(
        &self,
        component_path: &Path,
        step_name: &str,
        ctx: &ProvisionContext,
    ) -> Result<Value, ExecutorError> {
        let wasm_bytes = load_component_bytes(component_path)?;

        let mut config = Config::new();
        config.epoch_interruption(true);

        let engine = Engine::new(&config)?;
        let limits = StoreLimitsBuilder::new()
            .memory_size(self.limits.memory_limit_bytes)
            .build();
        let mut store = Store::new(&engine, StoreState { limits });

        store.limiter(|state| &mut state.limits);

        let epoch_engine = engine.clone();
        let timeout = self.limits.timeout_ms;
        let epoch_handle = thread::spawn(move || {
            thread::sleep(Duration::from_millis(timeout));
            epoch_engine.increment_epoch();
        });
        store.set_epoch_deadline(1);

        let module = Module::new(&engine, wasm_bytes)?;
        let instance = Instance::new(&mut store, &module, &[])?;

        let memory = instance
            .get_memory(&mut store, "memory")
            .ok_or_else(|| ExecutorError::Trap("missing exported memory".to_string()))?;

        let input = json!({
            "step": step_name,
            "inputs": ctx.inputs,
            "state": {
                "answers": ctx.inputs.answers,
                "previous": ctx.prior_results,
            }
        });
        let input_bytes = serde_json::to_vec(&input)?;

        let memory_size = memory.data_size(&store);
        if input_bytes.len() > memory_size {
            return Err(ExecutorError::InputTooLarge(input_bytes.len()));
        }

        let input_ptr = 4096usize;
        if input_ptr + input_bytes.len() > memory_size {
            return Err(ExecutorError::InputTooLarge(input_bytes.len()));
        }
        memory.write(&mut store, input_ptr, &input_bytes)?;

        let func = instance
            .get_func(&mut store, "run")
            .ok_or_else(|| ExecutorError::Trap("missing run export".to_string()))?;
        let func = func.typed::<(i32, i32), (i32, i32)>(&store)?;

        let (output_ptr, output_len) = func
            .call(&mut store, (input_ptr as i32, input_bytes.len() as i32))
            .map_err(|err| ExecutorError::Trap(err.to_string()))?;

        let output_len = output_len as usize;
        if output_len > self.limits.max_output_bytes {
            return Err(ExecutorError::OutputTooLarge(output_len));
        }

        let mut buffer = vec![0u8; output_len];
        memory.read(&mut store, output_ptr as usize, &mut buffer)?;
        let output_json: Value = serde_json::from_slice(&buffer)?;

        let _ = epoch_handle.join();

        Ok(output_json)
    }
}

impl ProvisionExecutor for WasmtimeExecutor {
    fn run_step(&self, step: ProvisionStep, ctx: &ProvisionContext) -> StepOutput {
        let step_name = match step {
            ProvisionStep::Collect => "collect",
            ProvisionStep::Validate => "validate",
            ProvisionStep::Apply => "apply",
            ProvisionStep::Summary => "summary",
        };

        match self.run_named_step(step_name, ctx) {
            Ok(output) => output,
            Err(err) => StepOutput {
                data: json!({ "error": err.to_string(), "step": step_name }),
                diagnostics: Vec::new(),
                plan_patch: None,
                questions: None,
            },
        }
    }
}

fn load_component_bytes(path: &Path) -> Result<Vec<u8>, ExecutorError> {
    let bytes = fs::read(path)?;
    if path.extension().and_then(|ext| ext.to_str()) == Some("wat") {
        let wasm = wat::parse_bytes(&bytes)?;
        Ok(wasm.into())
    } else {
        Ok(bytes)
    }
}

fn step_output_from_json(value: Value) -> Result<StepOutput, ExecutorError> {
    let plan_patch = value
        .get("plan")
        .cloned()
        .map(plan_patch_from_value)
        .transpose()?;

    let questions = value.get("questions").cloned();

    Ok(StepOutput {
        data: value,
        diagnostics: Vec::new(),
        plan_patch,
        questions,
    })
}

fn plan_patch_from_value(value: Value) -> Result<ProvisionPlanPatch, ExecutorError> {
    let config_patch = value
        .get("config_patch")
        .and_then(|v| v.as_object())
        .map(|map| map.iter().map(|(k, v)| (k.clone(), v.clone())).collect());

    let secrets_patch = value.get("secrets_patch").cloned();
    let webhook_ops = value
        .get("webhook_ops")
        .and_then(|v| v.as_array())
        .map(|list| list.to_vec());
    let subscription_ops = value
        .get("subscription_ops")
        .and_then(|v| v.as_array())
        .map(|list| list.to_vec());
    let oauth_ops = value
        .get("oauth_ops")
        .and_then(|v| v.as_array())
        .map(|list| list.to_vec());
    let notes = value.get("notes").and_then(|v| v.as_array()).map(|list| {
        list.iter()
            .filter_map(|item| item.as_str().map(|s| s.to_string()))
            .collect()
    });

    let mut patch = ProvisionPlanPatch {
        config_patch,
        secrets_patch: None,
        webhook_ops: None,
        subscription_ops: None,
        oauth_ops: None,
        notes,
    };

    if let Some(secrets_value) = secrets_patch {
        patch.secrets_patch = Some(serde_json::from_value(secrets_value)?);
    }
    if let Some(webhook_value) = webhook_ops {
        patch.webhook_ops = Some(serde_json::from_value(Value::Array(webhook_value))?);
    }
    if let Some(subscription_value) = subscription_ops {
        patch.subscription_ops = Some(serde_json::from_value(Value::Array(subscription_value))?);
    }
    if let Some(oauth_value) = oauth_ops {
        patch.oauth_ops = Some(serde_json::from_value(Value::Array(oauth_value))?);
    }

    Ok(patch)
}

pub fn timestamp_label() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}-{}", now.as_secs(), now.subsec_millis())
}

#[derive(Debug)]
struct StoreState {
    limits: StoreLimits,
}
