use std::fs;
use std::path::{Path, PathBuf};

use crate::model::{ArtifactKind, Ownership, Safety, ScanReport, ScopeKind};
use crate::provider::ScanContext;

use super::support::{FindingSpec, add_empty_dir, add_path, content_equal, is_older_than};

const MIRRORED_RESOURCES: &[&str] = &[
    "skills",
    "hooks",
    "plugins",
    "plugin-state",
    "profile-v2",
    "themes",
    "prompts",
    "AGENTS.md",
];

pub(crate) fn inspect(context: &ScanContext, data_dirs: &[PathBuf], report: &mut ScanReport) {
    for data_dir in data_dirs {
        inspect_runtime_home(context, data_dir, report);
        inspect_backfill(context, data_dir, report);
    }
}

fn inspect_runtime_home(context: &ScanContext, data_dir: &Path, report: &mut ScanReport) {
    let runtime_root = data_dir.join("codex-runtime-home");
    let home = runtime_root.join("home");
    let Ok(entries) = fs::read_dir(&home) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        let Ok(metadata) = fs::symlink_metadata(&path) else {
            continue;
        };
        let mirrored = MIRRORED_RESOURCES.contains(&name.as_str());
        let source = context.home.join(".codex").join(&name);
        let (safety, evidence) = if mirrored && metadata.file_type().is_symlink() {
            (
                Safety::Automatic,
                "the runtime resource is a link inside Orca-owned CODEX_HOME",
            )
        } else if mirrored && source.exists() && content_equal(&path, &source) {
            (
                Safety::Automatic,
                "the fallback copy is byte-for-byte equivalent to the real Codex resource",
            )
        } else if mirrored {
            (
                Safety::ReviewRequired,
                "the fallback copy cannot be proven equivalent to the real Codex resource",
            )
        } else {
            (
                Safety::ReviewRequired,
                "the path is Orca runtime state but may contain credentials, sessions, or promoted settings",
            )
        };
        add_path(
            report,
            &path,
            FindingSpec {
                kind: ArtifactKind::RuntimeState,
                ownership: Ownership::ProviderOwned,
                safety,
                description: "Orca Codex runtime-home resource",
                evidence,
                id_prefix: "remove-codex-runtime-resource",
                scope: ScopeKind::Local,
                actionable: true,
            },
        );
    }
    add_empty_dir(
        report,
        &home,
        ArtifactKind::RuntimeState,
        ScopeKind::Local,
        "remove empty Orca Codex runtime home",
    );
    add_empty_dir(
        report,
        &runtime_root,
        ArtifactKind::RuntimeState,
        ScopeKind::Local,
        "remove empty Orca Codex runtime metadata directory",
    );
}

fn inspect_backfill(context: &ScanContext, data_dir: &Path, report: &mut ScanReport) {
    let path = data_dir.join("codex-session-backfill");
    if !path.exists() {
        return;
    }
    let stale = is_older_than(&path, context.stale_after_days);
    add_path(
        report,
        &path,
        FindingSpec {
            kind: ArtifactKind::RuntimeState,
            ownership: Ownership::ProviderOwned,
            safety: if stale {
                Safety::Automatic
            } else {
                Safety::ReviewRequired
            },
            description: "Orca Codex session backfill state",
            evidence: if stale {
                "the Orca-owned migration state exceeds the configured retention window"
            } else {
                "the Orca-owned migration state may still be active"
            },
            id_prefix: "remove-codex-backfill",
            scope: ScopeKind::Local,
            actionable: true,
        },
    );
}
