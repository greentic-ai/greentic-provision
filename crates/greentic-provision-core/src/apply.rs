use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::types::{
    OAuthOp, ProvisionInputs, ProvisionResult, RedactedValue, SubscriptionOp, TenantContext,
};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApplyMode {
    DryRun,
    Apply,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ApplyReport {
    pub mode: ApplyMode,
    pub config_changes: Vec<String>,
    pub secret_set_keys: Vec<String>,
    pub secret_deleted_keys: Vec<String>,
    pub oauth_ops: Vec<OAuthOp>,
    pub subscription_state: Vec<SubscriptionState>,
    pub install_record: ProviderInstallRecord,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProviderInstallRecord {
    pub tenant: TenantContext,
    pub provider_id: String,
    pub install_id: String,
    pub config_namespace: String,
    pub secrets_namespace: String,
    pub subscriptions: Vec<SubscriptionState>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SubscriptionState {
    pub id: String,
    pub resource: String,
    pub expiry: Option<String>,
    pub last_sync: Option<String>,
}

pub trait InstallStore {
    fn get(
        &self,
        tenant: &TenantContext,
        provider_id: &str,
        install_id: &str,
    ) -> Option<ProviderInstallRecord>;
    fn put(&mut self, record: ProviderInstallRecord);
    fn list(&self, tenant: &TenantContext) -> Vec<ProviderInstallRecord>;
    fn delete(&mut self, tenant: &TenantContext, provider_id: &str, install_id: &str) -> bool;
}

#[derive(Debug, Default)]
pub struct InMemoryInstallStore {
    records: Vec<ProviderInstallRecord>,
}

#[derive(Debug)]
pub struct FileInstallStore {
    path: PathBuf,
    records: Vec<ProviderInstallRecord>,
}

impl FileInstallStore {
    pub fn new(path: impl Into<PathBuf>) -> Result<Self, std::io::Error> {
        let path = path.into();
        let records = load_records(&path)?;
        Ok(Self { path, records })
    }

    pub fn default_path() -> PathBuf {
        PathBuf::from(".greentic/provision/installs.json")
    }

    fn persist(&self) -> Result<(), std::io::Error> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let payload = serde_json::to_string_pretty(&self.records).map_err(std::io::Error::other)?;
        std::fs::write(&self.path, payload)?;
        Ok(())
    }
}

impl InstallStore for InMemoryInstallStore {
    fn get(
        &self,
        tenant: &TenantContext,
        provider_id: &str,
        install_id: &str,
    ) -> Option<ProviderInstallRecord> {
        self.records
            .iter()
            .find(|record| {
                record.tenant == *tenant
                    && record.provider_id == provider_id
                    && record.install_id == install_id
            })
            .cloned()
    }

    fn put(&mut self, record: ProviderInstallRecord) {
        if let Some(existing) = self.records.iter_mut().find(|item| {
            item.tenant == record.tenant
                && item.provider_id == record.provider_id
                && item.install_id == record.install_id
        }) {
            *existing = record;
        } else {
            self.records.push(record);
        }
    }

    fn list(&self, tenant: &TenantContext) -> Vec<ProviderInstallRecord> {
        self.records
            .iter()
            .filter(|record| record.tenant == *tenant)
            .cloned()
            .collect()
    }

    fn delete(&mut self, tenant: &TenantContext, provider_id: &str, install_id: &str) -> bool {
        let initial_len = self.records.len();
        self.records.retain(|record| {
            !(record.tenant == *tenant
                && record.provider_id == provider_id
                && record.install_id == install_id)
        });
        initial_len != self.records.len()
    }
}

impl InstallStore for FileInstallStore {
    fn get(
        &self,
        tenant: &TenantContext,
        provider_id: &str,
        install_id: &str,
    ) -> Option<ProviderInstallRecord> {
        self.records
            .iter()
            .find(|record| {
                record.tenant == *tenant
                    && record.provider_id == provider_id
                    && record.install_id == install_id
            })
            .cloned()
    }

    fn put(&mut self, record: ProviderInstallRecord) {
        if let Some(existing) = self.records.iter_mut().find(|item| {
            item.tenant == record.tenant
                && item.provider_id == record.provider_id
                && item.install_id == record.install_id
        }) {
            *existing = record;
        } else {
            self.records.push(record);
        }
        let _ = self.persist();
    }

    fn list(&self, tenant: &TenantContext) -> Vec<ProviderInstallRecord> {
        self.records
            .iter()
            .filter(|record| record.tenant == *tenant)
            .cloned()
            .collect()
    }

    fn delete(&mut self, tenant: &TenantContext, provider_id: &str, install_id: &str) -> bool {
        let initial_len = self.records.len();
        self.records.retain(|record| {
            !(record.tenant == *tenant
                && record.provider_id == provider_id
                && record.install_id == install_id)
        });
        let removed = initial_len != self.records.len();
        if removed {
            let _ = self.persist();
        }
        removed
    }
}

fn load_records(path: &Path) -> Result<Vec<ProviderInstallRecord>, std::io::Error> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let contents = std::fs::read_to_string(path)?;
    let records = serde_json::from_str(&contents)
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
    Ok(records)
}

pub trait ConfigStore {
    fn apply_patch(&mut self, namespace: &str, patch: &BTreeMap<String, Value>) -> Vec<String>;
    fn read_namespace(&self, namespace: &str) -> BTreeMap<String, Value>;
}

pub struct ConfigApplier<C> {
    store: C,
}

impl<C> ConfigApplier<C>
where
    C: ConfigStore,
{
    pub fn new(store: C) -> Self {
        Self { store }
    }

    pub fn plan_only(&self, patch: &BTreeMap<String, Value>) -> Vec<String> {
        patch.keys().cloned().collect()
    }

    pub fn apply(&mut self, namespace: &str, patch: &BTreeMap<String, Value>) -> Vec<String> {
        self.store.apply_patch(namespace, patch)
    }

    pub fn into_inner(self) -> C {
        self.store
    }
}

#[derive(Debug, Default)]
pub struct InMemoryConfigStore {
    namespaces: BTreeMap<String, BTreeMap<String, Value>>,
}

impl ConfigStore for InMemoryConfigStore {
    fn apply_patch(&mut self, namespace: &str, patch: &BTreeMap<String, Value>) -> Vec<String> {
        let entry = self.namespaces.entry(namespace.to_string()).or_default();
        let mut changed = Vec::new();
        for (key, value) in patch {
            entry.insert(key.clone(), value.clone());
            changed.push(key.clone());
        }
        changed
    }

    fn read_namespace(&self, namespace: &str) -> BTreeMap<String, Value> {
        self.namespaces.get(namespace).cloned().unwrap_or_default()
    }
}

pub trait SecretsStore {
    fn set_secret(&mut self, namespace: &str, key: &str, value: &str);
    fn delete_secret(&mut self, namespace: &str, key: &str);
    fn list_keys(&self, namespace: &str) -> Vec<String>;
}

#[derive(Debug, Default)]
pub struct InMemorySecretsStore {
    namespaces: BTreeMap<String, BTreeMap<String, String>>,
}

impl SecretsStore for InMemorySecretsStore {
    fn set_secret(&mut self, namespace: &str, key: &str, value: &str) {
        let entry = self.namespaces.entry(namespace.to_string()).or_default();
        entry.insert(key.to_string(), value.to_string());
    }

    fn delete_secret(&mut self, namespace: &str, key: &str) {
        if let Some(entry) = self.namespaces.get_mut(namespace) {
            entry.remove(key);
        }
    }

    fn list_keys(&self, namespace: &str) -> Vec<String> {
        self.namespaces
            .get(namespace)
            .map(|map| map.keys().cloned().collect())
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OAuthTokenSet {
    pub access_token: String,
    pub refresh_token: Option<String>,
}

pub trait OAuthHandler {
    fn start(&mut self, op: &OAuthOp) -> Option<OAuthTokenSet>;
}

#[derive(Debug, Default)]
pub struct NoopOAuthHandler;

impl OAuthHandler for NoopOAuthHandler {
    fn start(&mut self, _op: &OAuthOp) -> Option<OAuthTokenSet> {
        None
    }
}

pub struct ProvisionApplier<C, S, O, I> {
    inputs: ProvisionInputs,
    config_store: C,
    secrets_store: S,
    oauth_handler: O,
    install_store: I,
}

impl<C, S, O, I> ProvisionApplier<C, S, O, I>
where
    C: ConfigStore,
    S: SecretsStore,
    O: OAuthHandler,
    I: InstallStore,
{
    pub fn new(
        inputs: ProvisionInputs,
        config_store: C,
        secrets_store: S,
        oauth_handler: O,
        install_store: I,
    ) -> Self {
        Self {
            inputs,
            config_store,
            secrets_store,
            oauth_handler,
            install_store,
        }
    }

    pub fn apply(&mut self, result: ProvisionResult, mode: ApplyMode) -> ApplyReport {
        let namespace = provision_namespace(
            &self.inputs.tenant,
            &self.inputs.provider_id,
            &self.inputs.install_id,
        );
        let secrets_namespace = format!("{}:secrets", namespace);

        let (config_changes, secret_set_keys, secret_deleted_keys) = if mode == ApplyMode::Apply {
            let config_changes = self
                .config_store
                .apply_patch(&namespace, &result.plan.config_patch);
            let mut secret_set_keys = Vec::new();
            let mut secret_deleted_keys = Vec::new();
            for (key, value) in &result.plan.secrets_patch.set {
                if let Some(secret_value) = redacted_to_value(value) {
                    self.secrets_store
                        .set_secret(&secrets_namespace, key, &secret_value);
                    secret_set_keys.push(key.clone());
                }
            }
            for key in &result.plan.secrets_patch.delete {
                self.secrets_store.delete_secret(&secrets_namespace, key);
                secret_deleted_keys.push(key.clone());
            }
            (config_changes, secret_set_keys, secret_deleted_keys)
        } else {
            let config_changes = result.plan.config_patch.keys().cloned().collect();
            let secret_set_keys = result.plan.secrets_patch.set.keys().cloned().collect();
            let secret_deleted_keys = result.plan.secrets_patch.delete.clone();
            (config_changes, secret_set_keys, secret_deleted_keys)
        };

        let subscription_state = apply_subscription_ops(&result.plan.subscription_ops);
        let install_record = ProviderInstallRecord {
            tenant: self.inputs.tenant.clone(),
            provider_id: self.inputs.provider_id.clone(),
            install_id: self.inputs.install_id.clone(),
            config_namespace: namespace.clone(),
            secrets_namespace: secrets_namespace.clone(),
            subscriptions: subscription_state.clone(),
        };

        if mode == ApplyMode::Apply {
            self.install_store.put(install_record.clone());
        }

        let mut oauth_ops = Vec::new();
        for op in &result.plan.oauth_ops {
            oauth_ops.push(op.clone());
            if mode == ApplyMode::Apply
                && let Some(token_set) = self.oauth_handler.start(op)
            {
                self.secrets_store.set_secret(
                    &secrets_namespace,
                    "oauth_access_token",
                    &token_set.access_token,
                );
                if let Some(refresh) = token_set.refresh_token {
                    self.secrets_store.set_secret(
                        &secrets_namespace,
                        "oauth_refresh_token",
                        &refresh,
                    );
                }
            }
        }

        ApplyReport {
            mode,
            config_changes,
            secret_set_keys,
            secret_deleted_keys,
            oauth_ops,
            subscription_state,
            install_record,
        }
    }

    pub fn into_parts(self) -> (C, S, O, I) {
        (
            self.config_store,
            self.secrets_store,
            self.oauth_handler,
            self.install_store,
        )
    }
}

fn redacted_to_value(value: &RedactedValue) -> Option<String> {
    if value.redacted {
        return None;
    }
    value.value.clone()
}

fn apply_subscription_ops(ops: &[SubscriptionOp]) -> Vec<SubscriptionState> {
    ops.iter()
        .filter_map(|op| {
            if op.op == "register" || op.op == "update" {
                Some(SubscriptionState {
                    id: op.id.clone().unwrap_or_else(|| "unknown".to_string()),
                    resource: op
                        .metadata
                        .get("resource")
                        .and_then(Value::as_str)
                        .unwrap_or("unknown")
                        .to_string(),
                    expiry: op
                        .metadata
                        .get("expiry")
                        .and_then(Value::as_str)
                        .map(|s| s.to_string()),
                    last_sync: None,
                })
            } else {
                None
            }
        })
        .collect()
}

fn provision_namespace(tenant: &TenantContext, provider_id: &str, install_id: &str) -> String {
    let env = tenant.environment.as_deref().unwrap_or("unknown");
    let tenant_id = tenant.tenant.as_deref().unwrap_or("unknown");
    let team = tenant.team.as_deref().unwrap_or("unknown");
    format!(
        "provision:{}:{}:{}:{}:{}",
        env, tenant_id, team, provider_id, install_id
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ProvisionPlan, RedactedValue, SecretsPatch};

    #[test]
    fn apply_updates_config_namespace() {
        let inputs = ProvisionInputs {
            tenant: TenantContext {
                environment: Some("prod".to_string()),
                tenant: Some("tenant-a".to_string()),
                team: Some("team-a".to_string()),
                user: None,
            },
            provider_id: "provider".to_string(),
            install_id: "install".to_string(),
            public_base_url: None,
            answers: Value::Null,
            existing_state: None,
        };

        let mut plan = ProvisionPlan::default();
        plan.config_patch
            .insert("foo".to_string(), Value::String("bar".to_string()));

        let result = ProvisionResult {
            plan,
            diagnostics: Vec::new(),
            step_results: None,
        };

        let mut applier = ProvisionApplier::new(
            inputs,
            InMemoryConfigStore::default(),
            InMemorySecretsStore::default(),
            NoopOAuthHandler,
            InMemoryInstallStore::default(),
        );

        let report = applier.apply(result, ApplyMode::Apply);
        let (config_store, _secrets, _oauth, _installs) = applier.into_parts();
        let config = config_store.read_namespace(&report.install_record.config_namespace);
        assert_eq!(config.get("foo"), Some(&Value::String("bar".to_string())));
    }

    #[test]
    fn apply_secrets_patch_does_not_expose_values() {
        let inputs = ProvisionInputs {
            tenant: TenantContext::default(),
            provider_id: "provider".to_string(),
            install_id: "install".to_string(),
            public_base_url: None,
            answers: Value::Null,
            existing_state: None,
        };

        let mut plan = ProvisionPlan::default();
        let mut secrets_patch = SecretsPatch::default();
        secrets_patch
            .set
            .insert("secret".to_string(), RedactedValue::plaintext("value"));
        secrets_patch.delete.push("old".to_string());
        plan.secrets_patch = secrets_patch;

        let result = ProvisionResult {
            plan,
            diagnostics: Vec::new(),
            step_results: None,
        };

        let mut applier = ProvisionApplier::new(
            inputs,
            InMemoryConfigStore::default(),
            InMemorySecretsStore::default(),
            NoopOAuthHandler,
            InMemoryInstallStore::default(),
        );

        let report = applier.apply(result, ApplyMode::DryRun);
        assert_eq!(report.secret_set_keys, vec!["secret".to_string()]);
        assert_eq!(report.secret_deleted_keys, vec!["old".to_string()]);
    }

    #[test]
    fn install_record_persisted_and_retrievable() {
        let inputs = ProvisionInputs {
            tenant: TenantContext::default(),
            provider_id: "provider".to_string(),
            install_id: "install".to_string(),
            public_base_url: None,
            answers: Value::Null,
            existing_state: None,
        };

        let result = ProvisionResult {
            plan: ProvisionPlan::default(),
            diagnostics: Vec::new(),
            step_results: None,
        };

        let mut applier = ProvisionApplier::new(
            inputs.clone(),
            InMemoryConfigStore::default(),
            InMemorySecretsStore::default(),
            NoopOAuthHandler,
            InMemoryInstallStore::default(),
        );

        let report = applier.apply(result, ApplyMode::Apply);
        let (_config, _secrets, _oauth, store) = applier.into_parts();
        let stored = store
            .get(&inputs.tenant, &inputs.provider_id, &inputs.install_id)
            .expect("missing record");
        assert_eq!(stored, report.install_record);
    }
}
