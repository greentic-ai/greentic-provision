use std::fs::File;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::types::{
    ProvisionInputs, ProvisionMode, ProvisionPlan, ProvisionResult, ProvisionStep, StepOutput,
    StepResult,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvisionContext {
    pub inputs: ProvisionInputs,
    pub mode: ProvisionMode,
    pub step: ProvisionStep,
    pub prior_results: Vec<StepResult>,
}

pub trait ProvisionExecutor {
    fn run_step(&self, step: ProvisionStep, ctx: &ProvisionContext) -> StepOutput;
}

#[derive(Debug, Default)]
pub struct NoopExecutor;

impl ProvisionExecutor for NoopExecutor {
    fn run_step(&self, _step: ProvisionStep, _ctx: &ProvisionContext) -> StepOutput {
        StepOutput::default()
    }
}

pub struct ProvisionEngine<E: ProvisionExecutor> {
    executor: E,
}

impl<E: ProvisionExecutor> ProvisionEngine<E> {
    pub fn new(executor: E) -> Self {
        Self { executor }
    }

    pub fn run(&self, mode: ProvisionMode, inputs: ProvisionInputs) -> ProvisionResult {
        let mut step_results = Vec::new();
        let mut plan = ProvisionPlan::default();
        let mut diagnostics = Vec::new();

        for step in [
            ProvisionStep::Collect,
            ProvisionStep::Validate,
            ProvisionStep::Apply,
            ProvisionStep::Summary,
        ] {
            let ctx = ProvisionContext {
                inputs: inputs.clone(),
                mode: mode.clone(),
                step: step.clone(),
                prior_results: step_results.clone(),
            };
            let output = self.executor.run_step(step.clone(), &ctx);
            if let Some(patch) = output.plan_patch.clone() {
                plan.merge_patch(patch);
            }
            diagnostics.extend(output.diagnostics.clone());
            step_results.push(StepResult { step, output });
        }

        ProvisionResult {
            plan,
            diagnostics,
            step_results: Some(step_results),
        }
    }

    pub fn plan_from_fixtures(
        &self,
        fixtures: FixturePaths,
    ) -> Result<ProvisionResult, FixtureError> {
        let mut plan = ProvisionPlan::default();
        let mut diagnostics = Vec::new();
        let mut step_results = Vec::new();

        for (step, path) in fixtures.into_iter() {
            let output = load_step_output(&path)?;
            if let Some(patch) = output.plan_patch.clone() {
                plan.merge_patch(patch);
            }
            diagnostics.extend(output.diagnostics.clone());
            step_results.push(StepResult { step, output });
        }

        Ok(ProvisionResult {
            plan,
            diagnostics,
            step_results: Some(step_results),
        })
    }
}

#[derive(Debug)]
pub struct FixturePaths {
    pub collect: Option<std::path::PathBuf>,
    pub validate: Option<std::path::PathBuf>,
    pub apply: Option<std::path::PathBuf>,
    pub summary: Option<std::path::PathBuf>,
}

impl FixturePaths {
    pub fn iter(&self) -> Vec<(ProvisionStep, std::path::PathBuf)> {
        let mut entries = Vec::new();
        if let Some(path) = self.collect.clone() {
            entries.push((ProvisionStep::Collect, path));
        }
        if let Some(path) = self.validate.clone() {
            entries.push((ProvisionStep::Validate, path));
        }
        if let Some(path) = self.apply.clone() {
            entries.push((ProvisionStep::Apply, path));
        }
        if let Some(path) = self.summary.clone() {
            entries.push((ProvisionStep::Summary, path));
        }
        entries
    }
}

impl IntoIterator for FixturePaths {
    type Item = (ProvisionStep, std::path::PathBuf);
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter().into_iter()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum FixtureError {
    #[error("failed to read fixture: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to parse fixture JSON: {0}")]
    Json(#[from] serde_json::Error),
}

fn load_step_output(path: &Path) -> Result<StepOutput, FixtureError> {
    let file = File::open(path)?;
    let output = serde_json::from_reader(file)?;
    Ok(output)
}
