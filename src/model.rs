use std::path::PathBuf;

use serde::Serialize;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactKind {
    OwnedData,
    Cache,
    Log,
    TemporaryState,
    RuntimeState,
    Worktree,
    WorktreeTrash,
    ManagedHook,
    ConfigMutation,
    OrphanedState,
    UserHistory,
    TrustEntry,
    Plugin,
    SkillPlacement,
    Backup,
    RemoteRuntime,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ScopeKind {
    Local,
    Wsl,
    SshRemote,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FileFormat {
    Json,
    Jsonc,
    Toml,
    Yaml,
    Text,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Safety {
    Automatic,
    ReviewRequired,
    Informational,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Ownership {
    ProviderOwned,
    InjectedByProvider,
    Attributed,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PathSnapshot {
    pub digest: String,
    pub entries: u64,
    pub bytes: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CleanupActionKind {
    RewriteFile {
        expected_sha256: String,
        mutation_count: usize,
        format: FileFormat,
        #[serde(skip)]
        replacement: Vec<u8>,
    },
    RemoveGitWorktree {
        repository: PathBuf,
        expected: PathSnapshot,
    },
    RemovePath {
        expected: PathSnapshot,
    },
    RemoveEmptyDirectory,
}

#[derive(Clone, Debug, Serialize)]
pub struct CleanupAction {
    pub id: String,
    pub provider: String,
    pub description: String,
    pub path: PathBuf,
    pub scope: ScopeKind,
    pub safety: Safety,
    pub reclaimable_bytes: u64,
    pub kind: CleanupActionKind,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub depends_on: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Finding {
    pub id: String,
    pub provider: String,
    pub kind: ArtifactKind,
    pub ownership: Ownership,
    pub safety: Safety,
    pub path: PathBuf,
    pub scope: ScopeKind,
    pub description: String,
    pub evidence: String,
    pub reclaimable_bytes: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<CleanupAction>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct ScanReport {
    pub findings: Vec<Finding>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CleanupPlan {
    pub actions: Vec<CleanupAction>,
    pub excluded_review_findings: usize,
    pub reclaimable_bytes: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ApplyStatus {
    Applied,
    Skipped,
    Failed,
}

#[derive(Clone, Debug, Serialize)]
pub struct ActionResult {
    pub action_id: String,
    pub path: PathBuf,
    pub status: ApplyStatus,
    pub detail: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct ApplyReport {
    pub results: Vec<ActionResult>,
}

impl ApplyReport {
    #[must_use]
    pub fn has_failures(&self) -> bool {
        self.results
            .iter()
            .any(|result| matches!(result.status, ApplyStatus::Failed))
    }
}
