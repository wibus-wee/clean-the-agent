use serde::Serialize;
use thiserror::Error;

use crate::model::CleanupAction;
use crate::provider::ScanContext;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TweakStatus {
    NeedsChange,
    Satisfied,
    Blocked,
}

#[derive(Clone, Debug, Serialize)]
pub struct TweakReport {
    pub id: String,
    pub product: String,
    pub title: String,
    pub description: String,
    pub status: TweakStatus,
    pub path: std::path::PathBuf,
    pub detail: String,
    pub restart_required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<CleanupAction>,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct TweakSummary {
    pub id: &'static str,
    pub product: &'static str,
    pub title: &'static str,
    pub description: &'static str,
}

pub trait Tweak: Send + Sync {
    fn summary(&self) -> TweakSummary;
    fn inspect(&self, context: &ScanContext) -> TweakReport;
}

#[derive(Debug, Error)]
pub enum TweakError {
    #[error("unknown tweak: {0}")]
    Unknown(String),
}

pub struct TweakEngine {
    tweaks: Vec<Box<dyn Tweak>>,
}

impl TweakEngine {
    #[must_use]
    pub fn new(tweaks: Vec<Box<dyn Tweak>>) -> Self {
        Self { tweaks }
    }

    #[must_use]
    pub fn summaries(&self) -> Vec<TweakSummary> {
        let mut summaries = self
            .tweaks
            .iter()
            .map(|tweak| tweak.summary())
            .collect::<Vec<_>>();
        summaries.sort_by_key(|summary| summary.id);
        summaries
    }

    /// Inspects one explicitly selected desired-state tweak without changing it.
    ///
    /// # Errors
    ///
    /// Returns [`TweakError::Unknown`] when the identifier is not registered.
    pub fn inspect(&self, id: &str, context: &ScanContext) -> Result<TweakReport, TweakError> {
        self.tweaks
            .iter()
            .find(|tweak| tweak.summary().id == id)
            .map(|tweak| tweak.inspect(context))
            .ok_or_else(|| TweakError::Unknown(id.to_owned()))
    }
}
