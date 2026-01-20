use std::collections::BTreeMap;

use greentic_types::validate::Diagnostic;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProvisionMode {
    Install,
    Update,
    Delete,
    DryRun,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProvisionStep {
    Collect,
    Validate,
    Apply,
    Summary,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct TenantContext {
    pub environment: Option<String>,
    pub tenant: Option<String>,
    pub team: Option<String>,
    pub user: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProvisionInputs {
    pub tenant: TenantContext,
    pub provider_id: String,
    pub install_id: String,
    pub public_base_url: Option<String>,
    pub answers: Value,
    pub existing_state: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ProvisionPlan {
    pub config_patch: BTreeMap<String, Value>,
    pub secrets_patch: SecretsPatch,
    pub webhook_ops: Vec<WebhookOp>,
    pub subscription_ops: Vec<SubscriptionOp>,
    pub oauth_ops: Vec<OAuthOp>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct SecretsPatch {
    pub set: BTreeMap<String, RedactedValue>,
    pub delete: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RedactedValue {
    pub redacted: bool,
    pub value: Option<String>,
}

impl RedactedValue {
    pub fn redacted() -> Self {
        Self {
            redacted: true,
            value: None,
        }
    }

    pub fn plaintext(value: impl Into<String>) -> Self {
        Self {
            redacted: false,
            value: Some(value.into()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WebhookOp {
    pub op: String,
    pub id: Option<String>,
    pub url: Option<String>,
    #[serde(default)]
    pub metadata: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SubscriptionOp {
    pub op: String,
    pub id: Option<String>,
    #[serde(default)]
    pub metadata: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProvisionPlanPatch {
    pub config_patch: Option<BTreeMap<String, Value>>,
    pub secrets_patch: Option<SecretsPatch>,
    pub webhook_ops: Option<Vec<WebhookOp>>,
    pub subscription_ops: Option<Vec<SubscriptionOp>>,
    pub oauth_ops: Option<Vec<OAuthOp>>,
    pub notes: Option<Vec<String>>,
}

impl ProvisionPlan {
    pub fn merge_patch(&mut self, patch: ProvisionPlanPatch) {
        if let Some(config_patch) = patch.config_patch {
            self.config_patch.extend(config_patch);
        }
        if let Some(secrets_patch) = patch.secrets_patch {
            self.secrets_patch.set.extend(secrets_patch.set);
            self.secrets_patch.delete.extend(secrets_patch.delete);
        }
        if let Some(webhook_ops) = patch.webhook_ops {
            self.webhook_ops.extend(webhook_ops);
        }
        if let Some(subscription_ops) = patch.subscription_ops {
            self.subscription_ops.extend(subscription_ops);
        }
        if let Some(oauth_ops) = patch.oauth_ops {
            self.oauth_ops.extend(oauth_ops);
        }
        if let Some(notes) = patch.notes {
            self.notes.extend(notes);
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum OAuthOp {
    Start {
        provider: String,
        scopes: Vec<String>,
        redirect_url: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProvisionResult {
    pub plan: ProvisionPlan,
    pub diagnostics: Vec<Diagnostic>,
    pub step_results: Option<Vec<StepResult>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StepResult {
    pub step: ProvisionStep,
    pub output: StepOutput,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StepOutput {
    pub data: Value,
    pub diagnostics: Vec<Diagnostic>,
    pub plan_patch: Option<ProvisionPlanPatch>,
    pub questions: Option<Value>,
}

impl Default for StepOutput {
    fn default() -> Self {
        Self {
            data: Value::Null,
            diagnostics: Vec::new(),
            plan_patch: None,
            questions: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_serialization_is_deterministic() {
        let mut plan = ProvisionPlan::default();
        plan.config_patch.insert("zeta".to_string(), Value::Null);
        plan.config_patch
            .insert("alpha".to_string(), Value::Bool(true));

        let serialized = serde_json::to_string(&plan).expect("failed to serialize plan");
        let alpha_pos = serialized.find("\"alpha\"").expect("missing alpha");
        let zeta_pos = serialized.find("\"zeta\"").expect("missing zeta");
        assert!(alpha_pos < zeta_pos, "expected deterministic key ordering");
    }
}
