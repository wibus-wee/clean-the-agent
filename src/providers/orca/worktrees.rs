use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::model::{
    ArtifactKind, CleanupAction, CleanupActionKind, Finding, Ownership, Safety, ScanReport,
    ScopeKind,
};
use crate::operations::snapshot_path;

use super::state::StateHints;
use super::support::{FindingSpec, add_empty_dir, add_path, finding_id};

const TRASH_NAME: &str = ".orca-worktree-trash";
const MAX_CONTAINERS: usize = 200;

pub(crate) fn inspect_attributed(hints: &StateHints, report: &mut ScanReport) {
    let mut seen = HashSet::new();
    for worktree in &hints.worktrees {
        if !seen.insert(worktree.path.clone()) {
            continue;
        }
        if !worktree.path.exists() {
            report.findings.push(Finding {
                id: finding_id("orphaned-worktree-state", &worktree.path),
                provider: "orca".to_owned(),
                kind: ArtifactKind::OrphanedState,
                ownership: Ownership::Attributed,
                safety: Safety::Informational,
                path: worktree.path.clone(),
                scope: ScopeKind::Local,
                description: "Orca metadata references a missing worktree".to_owned(),
                evidence: format!("the persisted record carries {}", worktree.source),
                reclaimable_bytes: 0,
                action: None,
            });
            continue;
        }
        let Some(repository) = hints.repos.get(&worktree.repo_id) else {
            report.warnings.push(format!(
                "Orca-created worktree {} has no registered repository",
                worktree.path.display()
            ));
            continue;
        };
        if !worktree.path.join(".git").exists() || same_path(repository, &worktree.path) {
            continue;
        }
        match snapshot_path(&worktree.path) {
            Ok(snapshot) => {
                let id = finding_id("remove-worktree", &worktree.path);
                let reclaimable_bytes = snapshot.bytes;
                report.findings.push(Finding {
                    id: id.clone(),
                    provider: "orca".to_owned(),
                    kind: ArtifactKind::Worktree,
                    ownership: Ownership::Attributed,
                    safety: Safety::ReviewRequired,
                    path: worktree.path.clone(),
                    scope: ScopeKind::Local,
                    description: "Orca-created Git worktree".to_owned(),
                    evidence: format!(
                        "Orca state records {} and the checkout has Git worktree metadata",
                        worktree.source
                    ),
                    reclaimable_bytes,
                    action: Some(CleanupAction {
                        id,
                        provider: "orca".to_owned(),
                        description: "remove the clean, registered Orca-created Git worktree"
                            .to_owned(),
                        path: worktree.path.clone(),
                        scope: ScopeKind::Local,
                        safety: Safety::ReviewRequired,
                        reclaimable_bytes,
                        kind: CleanupActionKind::RemoveGitWorktree {
                            repository: repository.clone(),
                            expected: snapshot,
                        },
                        depends_on: Vec::new(),
                    }),
                });
            }
            Err(error) => report.warnings.push(format!(
                "could not snapshot Orca worktree {}: {error}",
                worktree.path.display()
            )),
        }
    }
}

pub(crate) fn inspect_trash(
    workspace_roots: &BTreeMap<PathBuf, ScopeKind>,
    report: &mut ScanReport,
) {
    let mut trash_roots = BTreeMap::new();
    for (root, scope) in workspace_roots {
        trash_roots.insert(root.join(TRASH_NAME), *scope);
        let Ok(entries) = fs::read_dir(root) else {
            continue;
        };
        for entry in entries.filter_map(Result::ok).take(MAX_CONTAINERS) {
            if entry.file_type().is_ok_and(|kind| kind.is_dir()) && entry.file_name() != TRASH_NAME
            {
                trash_roots.insert(entry.path().join(TRASH_NAME), *scope);
            }
        }
    }
    for (root, scope) in trash_roots {
        inspect_trash_root(&root, scope, report);
    }
}

fn inspect_trash_root(root: &Path, scope: ScopeKind, report: &mut ScanReport) {
    let Ok(metadata) = fs::symlink_metadata(root) else {
        return;
    };
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        report.warnings.push(format!(
            "refusing non-directory worktree trash {}",
            root.display()
        ));
        return;
    }
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        let recognized = is_trash_entry(&entry.file_name().to_string_lossy());
        add_path(
            report,
            &path,
            FindingSpec {
                kind: ArtifactKind::WorktreeTrash,
                ownership: if recognized {
                    Ownership::ProviderOwned
                } else {
                    Ownership::Attributed
                },
                safety: if recognized {
                    Safety::Automatic
                } else {
                    Safety::Informational
                },
                description: if recognized {
                    "Stale Orca worktree trash"
                } else {
                    "Unrecognized worktree trash entry"
                },
                evidence: if recognized {
                    "the parent and wt-<timestamp>-<nonce> name match Orca's deferred-deletion format"
                } else {
                    "the name does not match Orca's generated deletion format"
                },
                id_prefix: "orca-worktree-trash",
                scope,
                actionable: recognized,
            },
        );
    }
    add_empty_dir(
        report,
        root,
        ArtifactKind::WorktreeTrash,
        scope,
        "remove empty Orca worktree trash directory",
    );
}

fn is_trash_entry(name: &str) -> bool {
    let Some(rest) = name.strip_prefix("wt-") else {
        return false;
    };
    let Some((timestamp, nonce)) = rest.split_once('-') else {
        return false;
    };
    !timestamp.is_empty()
        && timestamp.bytes().all(|byte| byte.is_ascii_digit())
        && nonce.len() == 8
        && nonce
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn same_path(left: &Path, right: &Path) -> bool {
    match (fs::canonicalize(left), fs::canonicalize(right)) {
        (Ok(left), Ok(right)) => left == right,
        _ => left == right,
    }
}
