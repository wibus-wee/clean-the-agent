use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::model::{ArtifactKind, Ownership, Safety, ScanReport, ScopeKind};
use crate::provider::ScanContext;

use super::support::{FindingSpec, add_empty_dir, add_path, is_older_than, scope_for_path};

const AUTOMATIC_NAMES: &[&str] = &[
    "orca-github-cache.json",
    "Cache",
    "Code Cache",
    "GPUCache",
    "DawnCache",
    "blob_storage",
    "Service Worker",
    "Crashpad",
    "temp",
    "tmp",
];

pub(crate) fn inspect(context: &ScanContext, data_dirs: &[PathBuf], report: &mut ScanReport) {
    let mut roots: BTreeSet<PathBuf> = data_dirs.iter().cloned().collect();
    for scope in context.home_scopes() {
        roots.insert(scope.home.join(".orca"));
    }
    for root in roots {
        inspect_root(context, &root, report);
    }
}

fn inspect_root(context: &ScanContext, root: &Path, report: &mut ScanReport) {
    let Ok(metadata) = fs::symlink_metadata(root) else {
        return;
    };
    let scope = scope_for_path(root, context);
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        add_path(
            report,
            root,
            FindingSpec {
                kind: ArtifactKind::OwnedData,
                ownership: Ownership::ProviderOwned,
                safety: Safety::ReviewRequired,
                description: "Unexpected object at an Orca data root",
                evidence: "the location is Orca-specific but its type is unexpected",
                id_prefix: "review-orca-root",
                scope,
                actionable: true,
            },
        );
        return;
    }
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries.filter_map(Result::ok).collect::<Vec<_>>(),
        Err(error) => {
            report
                .warnings
                .push(format!("could not inspect {}: {error}", root.display()));
            return;
        }
    };
    for entry in entries {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        match name.as_ref() {
            "orca-data.json"
            | "agent-hooks"
            | "codex-runtime-home"
            | "codex-session-backfill"
            | "terminal-history"
            | "terminal-history-wsl"
            | "logs" => {}
            name if AUTOMATIC_NAMES.contains(&name) => add_path(
                report,
                &path,
                FindingSpec {
                    kind: if name.contains("Cache") || name.contains("cache") {
                        ArtifactKind::Cache
                    } else {
                        ArtifactKind::TemporaryState
                    },
                    ownership: Ownership::ProviderOwned,
                    safety: Safety::Automatic,
                    description: "Orca cache or disposable runtime data",
                    evidence: "the exact direct-child name is a known Orca/Electron disposable path",
                    id_prefix: "remove-orca-disposable",
                    scope,
                    actionable: true,
                },
            ),
            _ => add_path(
                report,
                &path,
                FindingSpec {
                    kind: ArtifactKind::OwnedData,
                    ownership: Ownership::ProviderOwned,
                    safety: Safety::ReviewRequired,
                    description: "Other Orca-owned application data",
                    evidence: "the path is inside an Orca-exclusive root but has no disposable-data rule",
                    id_prefix: "review-orca-data",
                    scope,
                    actionable: true,
                },
            ),
        }
    }
    inspect_terminal_history(root, scope, report);
    inspect_logs(context, root, scope, report);
    add_empty_dir(
        report,
        root,
        ArtifactKind::OwnedData,
        scope,
        "remove empty Orca data root",
    );
}

fn inspect_terminal_history(root: &Path, scope: ScopeKind, report: &mut ScanReport) {
    for name in ["terminal-history", "terminal-history-wsl"] {
        let path = root.join(name);
        if !path.exists() {
            continue;
        }
        add_path(
            report,
            &path,
            FindingSpec {
                kind: ArtifactKind::UserHistory,
                ownership: Ownership::ProviderOwned,
                safety: Safety::ReviewRequired,
                description: "Orca terminal history",
                evidence: "this content is user history, not disposable cache",
                id_prefix: "review-terminal-history",
                scope,
                actionable: true,
            },
        );
        visit_pending_deletes(&path, scope, report);
    }
}

fn visit_pending_deletes(root: &Path, scope: ScopeKind, report: &mut ScanReport) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if entry
            .file_name()
            .to_string_lossy()
            .contains(".pending-delete")
        {
            add_path(
                report,
                &path,
                FindingSpec {
                    kind: ArtifactKind::OrphanedState,
                    ownership: Ownership::ProviderOwned,
                    safety: Safety::Automatic,
                    description: "Interrupted terminal-history deletion tombstone",
                    evidence: "the generated .pending-delete name records an interrupted Orca deletion",
                    id_prefix: "remove-history-tombstone",
                    scope,
                    actionable: true,
                },
            );
        } else if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
            visit_pending_deletes(&path, scope, report);
        }
    }
}

fn inspect_logs(context: &ScanContext, root: &Path, scope: ScopeKind, report: &mut ScanReport) {
    let logs = root.join("logs");
    let Ok(entries) = fs::read_dir(&logs) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        let stale = is_older_than(&path, context.stale_after_days);
        add_path(
            report,
            &path,
            FindingSpec {
                kind: ArtifactKind::Log,
                ownership: Ownership::ProviderOwned,
                safety: if stale {
                    Safety::Automatic
                } else {
                    Safety::Informational
                },
                description: if stale {
                    "Expired Orca log"
                } else {
                    "Active or recent Orca log"
                },
                evidence: if stale {
                    "its modification time exceeds the configured retention window"
                } else {
                    "it is younger than the configured retention window"
                },
                id_prefix: "orca-log",
                scope,
                actionable: stale,
            },
        );
    }
    add_empty_dir(
        report,
        &logs,
        ArtifactKind::Log,
        scope,
        "remove empty Orca log directory",
    );
}
