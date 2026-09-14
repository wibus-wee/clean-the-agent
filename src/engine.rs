use std::collections::HashSet;

use thiserror::Error;

use crate::model::{
    ActionResult, ApplyReport, ApplyStatus, CleanupActionKind, CleanupPlan, Safety, ScanReport,
};
use crate::operations::{ApplyOutcome, apply_action};
use crate::provider::{Provider, ProviderError, ScanContext};

#[derive(Debug, Error)]
pub enum EngineError {
    #[error("unknown provider: {0}")]
    UnknownProvider(String),
    #[error(transparent)]
    Provider(#[from] ProviderError),
}

pub struct Engine {
    providers: Vec<Box<dyn Provider>>,
}

impl Engine {
    #[must_use]
    pub fn new(providers: Vec<Box<dyn Provider>>) -> Self {
        Self { providers }
    }

    #[must_use]
    pub fn provider_ids(&self) -> Vec<&'static str> {
        self.providers
            .iter()
            .map(|provider| provider.id())
            .collect()
    }

    /// Runs read-only detection for the selected providers.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::UnknownProvider`] when a requested provider is
    /// not registered, or propagates a provider detection error.
    pub fn scan(
        &self,
        context: &ScanContext,
        selected: &[String],
    ) -> Result<ScanReport, EngineError> {
        let requested: HashSet<&str> = selected.iter().map(String::as_str).collect();
        for id in &requested {
            if !self.providers.iter().any(|provider| provider.id() == *id) {
                return Err(EngineError::UnknownProvider((*id).to_owned()));
            }
        }

        let mut report = ScanReport::default();
        for provider in &self.providers {
            if !requested.is_empty() && !requested.contains(provider.id()) {
                continue;
            }
            let mut provider_report = provider.detect(context)?;
            report.findings.append(&mut provider_report.findings);
            report.warnings.append(&mut provider_report.warnings);
        }
        report
            .findings
            .sort_by(|left, right| left.path.cmp(&right.path).then(left.id.cmp(&right.id)));
        Ok(report)
    }

    #[must_use]
    pub fn plan(report: &ScanReport, include_review: bool) -> CleanupPlan {
        let mut actions = Vec::new();
        let mut excluded_review_findings = 0;
        for finding in &report.findings {
            match (&finding.action, finding.safety) {
                (Some(action), Safety::Automatic) => insert_action(&mut actions, action),
                (Some(action), Safety::ReviewRequired) if include_review => {
                    insert_action(&mut actions, action);
                }
                (Some(_), Safety::ReviewRequired) => excluded_review_findings += 1,
                _ => {}
            }
        }
        loop {
            let ids = actions
                .iter()
                .map(|action| action.id.clone())
                .collect::<HashSet<_>>();
            let before = actions.len();
            actions.retain(|action| {
                action
                    .depends_on
                    .iter()
                    .all(|dependency| ids.contains(dependency))
            });
            if actions.len() == before {
                break;
            }
        }
        actions = order_actions(actions);
        let reclaimable_bytes = actions.iter().map(|action| action.reclaimable_bytes).sum();
        CleanupPlan {
            actions,
            excluded_review_findings,
            reclaimable_bytes,
        }
    }

    #[must_use]
    pub fn apply(plan: &CleanupPlan) -> ApplyReport {
        let planned_ids: HashSet<&str> = plan
            .actions
            .iter()
            .map(|action| action.id.as_str())
            .collect();
        let mut completed = HashSet::new();
        let mut results = Vec::with_capacity(plan.actions.len());
        for action in &plan.actions {
            let missing_dependency = action.depends_on.iter().find(|dependency| {
                !planned_ids.contains(dependency.as_str())
                    || !completed.contains(dependency.as_str())
            });
            if let Some(dependency) = missing_dependency {
                results.push(ActionResult {
                    action_id: action.id.clone(),
                    path: action.path.clone(),
                    status: ApplyStatus::Skipped,
                    detail: format!("prerequisite action {dependency} was not applied"),
                });
                continue;
            }
            let result = match apply_action(action) {
                Ok(ApplyOutcome::Applied) => ActionResult {
                    action_id: action.id.clone(),
                    path: action.path.clone(),
                    status: ApplyStatus::Applied,
                    detail: action.description.clone(),
                },
                Ok(ApplyOutcome::Skipped) => ActionResult {
                    action_id: action.id.clone(),
                    path: action.path.clone(),
                    status: ApplyStatus::Skipped,
                    detail: "already absent or no longer empty".to_owned(),
                },
                Err(error) => ActionResult {
                    action_id: action.id.clone(),
                    path: action.path.clone(),
                    status: ApplyStatus::Failed,
                    detail: error.to_string(),
                },
            };
            if matches!(result.status, ApplyStatus::Applied | ApplyStatus::Skipped) {
                completed.insert(action.id.as_str());
            }
            results.push(result);
        }
        ApplyReport { results }
    }
}

fn insert_action(
    actions: &mut Vec<crate::model::CleanupAction>,
    candidate: &crate::model::CleanupAction,
) {
    let Some(existing) = actions
        .iter_mut()
        .find(|action| action.path == candidate.path)
    else {
        actions.push(candidate.clone());
        return;
    };
    if action_strength(&candidate.kind) > action_strength(&existing.kind) {
        *existing = candidate.clone();
    }
}

fn action_strength(kind: &CleanupActionKind) -> u8 {
    match kind {
        CleanupActionKind::RemovePath { .. } | CleanupActionKind::RemoveGitWorktree { .. } => 2,
        CleanupActionKind::RewriteFile { .. } => 1,
        CleanupActionKind::RemoveEmptyDirectory => 0,
    }
}

fn order_actions(
    mut actions: Vec<crate::model::CleanupAction>,
) -> Vec<crate::model::CleanupAction> {
    actions.sort_by_key(action_priority);
    let mut ordered = Vec::with_capacity(actions.len());
    let mut emitted = HashSet::new();
    while !actions.is_empty() {
        let Some(index) = actions.iter().position(|action| {
            action
                .depends_on
                .iter()
                .all(|dependency| emitted.contains(dependency))
        }) else {
            ordered.append(&mut actions);
            break;
        };
        let action = actions.remove(index);
        emitted.insert(action.id.clone());
        ordered.push(action);
    }
    ordered
}

fn action_priority(action: &crate::model::CleanupAction) -> (u8, std::cmp::Reverse<usize>) {
    let priority = match action.kind {
        CleanupActionKind::RewriteFile { .. } => 0,
        CleanupActionKind::RemoveGitWorktree { .. } => 1,
        CleanupActionKind::RemovePath { .. } => 2,
        CleanupActionKind::RemoveEmptyDirectory => 3,
    };
    (
        priority,
        std::cmp::Reverse(action.path.components().count()),
    )
}
