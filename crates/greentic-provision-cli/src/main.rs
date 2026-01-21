use std::fs::File;
use std::io::{Cursor, Read};
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use greentic_provision_core::discovery::PackManifest;
use greentic_provision_core::{
    DefaultProvisionPackDiscovery, ExecutionLimits, NoopExecutor, ProvisionEngine,
    ProvisionExecutor, ProvisionInputs, ProvisionMode, ProvisionPackDiscovery, ProvisionStep,
    TenantContext, WasmtimeExecutor,
};
use serde_json::Value;
use tempfile::TempDir;
use zip::ZipArchive;

#[derive(Debug, Parser)]
#[command(name = "greentic-provision")]
#[command(about = "Provisioning engine CLI for Greentic packs", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Pack {
        #[command(subcommand)]
        command: PackCommands,
    },
    DryRun {
        #[command(subcommand)]
        command: DryRunCommands,
    },
    Conformance {
        #[arg(long)]
        packs: PathBuf,
        #[arg(long)]
        report: PathBuf,
        #[arg(long)]
        provider: Option<String>,
        #[arg(long)]
        live: bool,
    },
}

#[derive(Debug, Subcommand)]
enum PackCommands {
    Inspect {
        #[arg(long)]
        pack: PathBuf,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
enum DryRunCommands {
    Setup {
        #[arg(long)]
        pack: PathBuf,
        #[arg(long, default_value = "wasm")]
        executor: ExecutorKind,
        #[arg(long)]
        provider_id: String,
        #[arg(long)]
        install_id: String,
        #[arg(long)]
        public_base_url: Option<String>,
        #[arg(long)]
        answers: Option<PathBuf>,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Clone, clap::ValueEnum)]
enum ExecutorKind {
    Noop,
    Wasm,
}

enum CliExecutor {
    Noop(NoopExecutor),
    Wasm(WasmtimeExecutor),
}

impl ProvisionExecutor for CliExecutor {
    fn run_step(
        &self,
        step: ProvisionStep,
        ctx: &greentic_provision_core::ProvisionContext,
    ) -> greentic_provision_core::StepOutput {
        match self {
            CliExecutor::Noop(exec) => exec.run_step(step, ctx),
            CliExecutor::Wasm(exec) => exec.run_step(step, ctx),
        }
    }
}

fn main() -> Result<(), CliError> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Pack { command } => match command {
            PackCommands::Inspect { pack, json } => {
                let pack_ctx = resolve_pack_path(&pack)?;
                let manifest = load_manifest(&pack_ctx.root)?;
                let descriptor = DefaultProvisionPackDiscovery::discover(&manifest)
                    .ok_or(CliError::NoProvisioningEntry)?;

                if json {
                    println!("{}", serde_json::to_string_pretty(&descriptor)?);
                } else {
                    println!("Pack: {}@{}", descriptor.pack_id, descriptor.pack_version);
                    println!("Setup entry flow: {}", descriptor.setup_entry_flow);
                    if let Some(requirements) = descriptor.requirements_flow {
                        println!("Requirements flow: {}", requirements);
                    }
                    if let Some(subscriptions) = descriptor.subscriptions_flow {
                        println!("Subscriptions flow: {}", subscriptions);
                    }
                    println!(
                        "Requires public base URL: {}",
                        descriptor.requires_public_base_url
                    );
                    if !descriptor.outputs.is_empty() {
                        println!("Declared outputs: {}", descriptor.outputs.join(", "));
                    }
                }
            }
        },
        Commands::DryRun { command } => match command {
            DryRunCommands::Setup {
                pack,
                executor,
                provider_id,
                install_id,
                public_base_url,
                answers,
                json,
            } => {
                let pack_ctx = resolve_pack_path(&pack)?;
                let _manifest = load_manifest(&pack_ctx.root)?;
                let answers_json = answers
                    .map(|path| load_json_value(&path))
                    .transpose()?
                    .unwrap_or(Value::Object(serde_json::Map::new()));

                let inputs = ProvisionInputs {
                    tenant: TenantContext::default(),
                    provider_id,
                    install_id,
                    public_base_url,
                    answers: answers_json,
                    existing_state: None,
                };

                let executor = match executor {
                    ExecutorKind::Noop => CliExecutor::Noop(NoopExecutor),
                    ExecutorKind::Wasm => {
                        let executor =
                            WasmtimeExecutor::new(pack_ctx.root, ExecutionLimits::default())?;
                        CliExecutor::Wasm(executor)
                    }
                };
                let engine = ProvisionEngine::new(executor);
                let result = engine.run(ProvisionMode::DryRun, inputs);

                if json {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    println!(
                        "Dry-run completed with {} diagnostics.",
                        result.diagnostics.len()
                    );
                    println!("Plan notes: {}", result.plan.notes.len());
                }
            }
        },
        Commands::Conformance {
            packs,
            report,
            provider,
            live,
        } => {
            if live {
                eprintln!("warning: live mode is not implemented; running dry-run only");
            }
            run_conformance(&packs, &report, provider.as_deref())?;
        }
    }

    Ok(())
}

fn run_conformance(
    packs_dir: &PathBuf,
    report_path: &PathBuf,
    provider: Option<&str>,
) -> Result<(), CliError> {
    let log_dir = PathBuf::from("target/conformance_logs");
    std::fs::create_dir_all(&log_dir)?;
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(packs_dir)? {
        let entry = entry?;
        let path = entry.path();
        if let Some(filter) = provider {
            let file_name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            if file_name != filter {
                continue;
            }
        }
        entries.push(path);
    }

    let mut reports = Vec::new();
    for pack_path in entries {
        let pack_label = pack_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();
        let pack_ctx = match resolve_pack_path(&pack_path) {
            Ok(ctx) => ctx,
            Err(err) => {
                reports.push(ConformancePackReport::failed(
                    &pack_label,
                    format!("pack error: {err}"),
                ));
                continue;
            }
        };

        let manifest = match load_manifest(&pack_ctx.root) {
            Ok(manifest) => manifest,
            Err(err) => {
                reports.push(ConformancePackReport::failed(
                    &pack_label,
                    format!("manifest error: {err}"),
                ));
                continue;
            }
        };

        let descriptor = match DefaultProvisionPackDiscovery::discover(&manifest) {
            Some(desc) => desc,
            None => {
                reports.push(ConformancePackReport::failed(
                    &pack_label,
                    "missing setup entry".to_string(),
                ));
                continue;
            }
        };

        let inputs = ProvisionInputs {
            tenant: TenantContext::default(),
            provider_id: descriptor.pack_id.clone(),
            install_id: format!("{}-install", descriptor.pack_id),
            public_base_url: Some("https://example.invalid".to_string()),
            answers: Value::Object(serde_json::Map::new()),
            existing_state: None,
        };

        let executor = match WasmtimeExecutor::new(&pack_ctx.root, ExecutionLimits::default()) {
            Ok(exec) => exec,
            Err(err) => {
                reports.push(ConformancePackReport::failed(
                    &pack_label,
                    format!("executor error: {err}"),
                ));
                continue;
            }
        };

        if let Some(requirements_flow) = descriptor.requirements_flow.as_deref() {
            let ctx = greentic_provision_core::ProvisionContext {
                inputs: inputs.clone(),
                mode: ProvisionMode::DryRun,
                step: ProvisionStep::Validate,
                prior_results: Vec::new(),
            };
            if let Err(err) = executor.run_named_step(requirements_flow, &ctx) {
                reports.push(ConformancePackReport::failed(
                    &pack_label,
                    format!("requirements failed: {err}"),
                ));
                continue;
            }
        }

        let engine = ProvisionEngine::new(executor);
        let result = engine.run(ProvisionMode::DryRun, inputs.clone());

        let checks = check_conformance(&result);
        let report_entry = if checks.is_empty() {
            ConformancePackReport::passed(&pack_label, descriptor.pack_version.clone(), result)
        } else {
            capture_failure_artifacts(&pack_label, &inputs, &result)?;
            ConformancePackReport::failed_with(&pack_label, descriptor.pack_version.clone(), checks)
        };
        write_conformance_log(&log_dir, &report_entry)?;
        reports.push(report_entry);
    }

    let report = ConformanceReport { packs: reports };
    let json = serde_json::to_string_pretty(&report)?;
    std::fs::write(report_path, json)?;
    println!("Wrote conformance report to {}", report_path.display());

    if report.packs.iter().any(|pack| !pack.ok) {
        return Err(CliError::ConformanceFailed);
    }

    Ok(())
}

fn check_conformance(result: &greentic_provision_core::ProvisionResult) -> Vec<String> {
    let mut errors = Vec::new();
    let serialized_once = serde_json::to_string(&result.plan).unwrap_or_default();
    let serialized_twice = serde_json::to_string(&result.plan).unwrap_or_default();
    if serialized_once != serialized_twice {
        errors.push("plan serialization not deterministic".to_string());
    }
    if result
        .plan
        .secrets_patch
        .set
        .values()
        .any(|value| !value.redacted || value.value.is_some())
    {
        errors.push("secrets_patch contains non-redacted values".to_string());
    }
    errors
}

fn capture_failure_artifacts(
    pack_label: &str,
    inputs: &ProvisionInputs,
    result: &greentic_provision_core::ProvisionResult,
) -> Result<(), CliError> {
    let timestamp = greentic_provision_core::executor::timestamp_label();
    let artifact_dir = PathBuf::from(".greentic/provision/artifacts")
        .join(pack_label)
        .join(timestamp);
    std::fs::create_dir_all(&artifact_dir)?;
    std::fs::write(
        artifact_dir.join("inputs.json"),
        serde_json::to_string_pretty(inputs)?,
    )?;
    std::fs::write(
        artifact_dir.join("step_outputs.json"),
        serde_json::to_string_pretty(&result.step_results)?,
    )?;
    std::fs::write(
        artifact_dir.join("diagnostics.json"),
        serde_json::to_string_pretty(&result.diagnostics)?,
    )?;
    std::fs::write(
        artifact_dir.join("pack.json"),
        serde_json::to_string_pretty(&serde_json::json!({ "pack": pack_label }))?,
    )?;
    Ok(())
}

fn write_conformance_log(
    log_dir: &std::path::Path,
    report: &ConformancePackReport,
) -> Result<(), CliError> {
    let log_path = log_dir.join(format!("{}.log", report.pack));
    let mut contents = String::new();
    contents.push_str(&format!("pack={}\n", report.pack));
    if let Some(version) = &report.version {
        contents.push_str(&format!("version={}\n", version));
    }
    contents.push_str(&format!("ok={}\n", report.ok));
    if !report.errors.is_empty() {
        contents.push_str("errors:\n");
        for err in &report.errors {
            contents.push_str(&format!("- {}\n", err));
        }
    }
    std::fs::write(log_path, contents)?;
    Ok(())
}

#[derive(Debug, serde::Serialize)]
struct ConformanceReport {
    packs: Vec<ConformancePackReport>,
}

#[derive(Debug, serde::Serialize)]
struct ConformancePackReport {
    pack: String,
    version: Option<String>,
    ok: bool,
    errors: Vec<String>,
    plan_notes: usize,
    secret_keys: Vec<String>,
}

impl ConformancePackReport {
    fn passed(
        pack: &str,
        version: String,
        result: greentic_provision_core::ProvisionResult,
    ) -> Self {
        Self {
            pack: pack.to_string(),
            version: Some(version),
            ok: true,
            errors: Vec::new(),
            plan_notes: result.plan.notes.len(),
            secret_keys: result.plan.secrets_patch.set.keys().cloned().collect(),
        }
    }

    fn failed(pack: &str, error: String) -> Self {
        Self {
            pack: pack.to_string(),
            version: None,
            ok: false,
            errors: vec![error],
            plan_notes: 0,
            secret_keys: Vec::new(),
        }
    }

    fn failed_with(pack: &str, version: String, errors: Vec<String>) -> Self {
        Self {
            pack: pack.to_string(),
            version: Some(version),
            ok: false,
            errors,
            plan_notes: 0,
            secret_keys: Vec::new(),
        }
    }
}

fn load_manifest(path: &PathBuf) -> Result<PackManifest, CliError> {
    if path.is_dir() {
        let mut candidates = vec![
            path.join("pack.json"),
            path.join("manifest.json"),
            path.join("manifest.cbor"),
        ];
        for candidate in candidates.drain(..) {
            if candidate.exists() {
                return load_manifest_from_file(&candidate);
            }
        }
        return Err(CliError::ManifestNotFound(path.clone()));
    }

    load_manifest_from_file(path)
}

fn load_manifest_from_file(path: &PathBuf) -> Result<PackManifest, CliError> {
    if path.extension().and_then(|ext| ext.to_str()) == Some("cbor") {
        let bytes = std::fs::read(path)?;
        return load_manifest_from_cbor_bytes(&bytes);
    }

    let mut file = File::open(path)?;
    let mut buffer = String::new();
    file.read_to_string(&mut buffer)?;
    let manifest = serde_json::from_str(&buffer)?;
    Ok(manifest)
}

fn load_manifest_from_cbor_bytes(bytes: &[u8]) -> Result<PackManifest, CliError> {
    if let Ok(manifest) = ciborium::de::from_reader(Cursor::new(bytes)) {
        return Ok(manifest);
    }

    let value: Value = ciborium::de::from_reader(Cursor::new(bytes))?;
    let normalized = normalize_manifest_value(value)
        .ok_or_else(|| CliError::ManifestDecode("unsupported CBOR manifest shape".to_string()))?;
    let manifest = serde_json::from_value(normalized)?;
    Ok(manifest)
}

fn normalize_manifest_value(value: Value) -> Option<Value> {
    match value {
        Value::Object(map) => {
            if let Some(Value::Object(nested)) = map
                .get("pack")
                .cloned()
                .or_else(|| map.get("manifest").cloned())
                .or_else(|| map.get("pack_manifest").cloned())
            {
                return Some(Value::Object(normalize_manifest_object(nested)));
            }
            Some(Value::Object(normalize_manifest_object(map)))
        }
        _ => None,
    }
}

fn normalize_manifest_object(
    mut map: serde_json::Map<String, Value>,
) -> serde_json::Map<String, Value> {
    if map.contains_key("id") {
        map.remove("pack_id");
        map.remove("packId");
    }
    if !map.contains_key("id") {
        if let Some(value) = resolve_pack_id(&map) {
            map.insert("id".to_string(), value);
        }
        map.remove("pack_id");
        map.remove("packId");
    }
    if map.contains_key("version") {
        map.remove("pack_version");
        map.remove("packVersion");
    }
    if !map.contains_key("version") {
        if let Some(value) = map
            .get("pack_version")
            .cloned()
            .or_else(|| map.get("packVersion").cloned())
        {
            map.insert("version".to_string(), value);
        }
        map.remove("pack_version");
        map.remove("packVersion");
    }
    map
}

fn resolve_pack_id(map: &serde_json::Map<String, Value>) -> Option<Value> {
    if let Some(value) = map
        .get("pack_id")
        .cloned()
        .or_else(|| map.get("packId").cloned())
    {
        if let Value::Number(index) = value {
            if let Some(idx) = index.as_u64().map(|v| v as usize)
                && let Some(Value::Object(symbols)) = map.get("symbols")
                && let Some(Value::Array(pack_ids)) = symbols.get("pack_ids")
                && let Some(Value::String(pack_id)) = pack_ids.get(idx)
            {
                return Some(Value::String(pack_id.clone()));
            }
            return None;
        }
        return Some(value);
    }
    None
}

struct PackContext {
    root: PathBuf,
    _temp: Option<TempDir>,
}

fn resolve_pack_path(path: &PathBuf) -> Result<PackContext, CliError> {
    if path.is_dir() {
        return Ok(PackContext {
            root: path.clone(),
            _temp: None,
        });
    }

    if path.extension().and_then(|ext| ext.to_str()) == Some("gtpack") {
        let temp = TempDir::new()?;
        let file = File::open(path)?;
        let mut archive = ZipArchive::new(file)?;
        for i in 0..archive.len() {
            let mut entry = archive.by_index(i)?;
            let out_path = temp.path().join(entry.name());
            if entry.is_dir() {
                std::fs::create_dir_all(&out_path)?;
                continue;
            }
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut out_file = File::create(&out_path)?;
            std::io::copy(&mut entry, &mut out_file)?;
        }
        return Ok(PackContext {
            root: temp.path().to_path_buf(),
            _temp: Some(temp),
        });
    }

    Ok(PackContext {
        root: path.clone(),
        _temp: None,
    })
}

fn load_json_value(path: &PathBuf) -> Result<Value, CliError> {
    let mut file = File::open(path)?;
    let mut buffer = String::new();
    file.read_to_string(&mut buffer)?;
    let value = serde_json::from_str(&buffer)?;
    Ok(value)
}

#[derive(Debug, thiserror::Error)]
enum CliError {
    #[error("failed to read file: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to parse JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("failed to parse CBOR: {0}")]
    Cbor(#[from] ciborium::de::Error<std::io::Error>),
    #[error("unsupported manifest format: {0}")]
    ManifestDecode(String),
    #[error("no provisioning entry found in pack manifest")]
    NoProvisioningEntry,
    #[error("manifest not found in directory: {0}")]
    ManifestNotFound(PathBuf),
    #[error("zip error: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("executor error: {0}")]
    Executor(#[from] greentic_provision_core::executor::ExecutorError),
    #[error("conformance failed")]
    ConformanceFailed,
}
