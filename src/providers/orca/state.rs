use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::model::{ArtifactKind, FileFormat, Ownership, Safety, ScanReport, ScopeKind};

use super::support::{FindingSpec, RewriteSpec, add_path, add_rewrite};

#[derive(Default)]
pub(crate) struct StateHints {
    pub repos: BTreeMap<String, PathBuf>,
    pub worktrees: Vec<WorktreeHint>,
    pub workspace_roots: BTreeSet<PathBuf>,
}

pub(crate) struct WorktreeHint {
    pub path: PathBuf,
    pub repo_id: String,
    pub source: &'static str,
}

pub(crate) fn inspect(data_dirs: &[PathBuf], report: &mut ScanReport) -> StateHints {
    let mut hints = StateHints::default();
    for data_dir in data_dirs {
        inspect_file(&data_dir.join("orca-data.json"), &mut hints, report);
    }
    hints
}

fn inspect_file(path: &Path, hints: &mut StateHints, report: &mut ScanReport) {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return,
        Err(error) => {
            report
                .warnings
                .push(format!("could not read {}: {error}", path.display()));
            return;
        }
    };
    let value: Value = match serde_json::from_slice(&bytes) {
        Ok(value) => value,
        Err(error) => {
            report
                .warnings
                .push(format!("could not parse {}: {error}", path.display()));
            return;
        }
    };
    collect_settings(&value, hints);
    collect_repositories(&value, hints);
    collect_worktree_map(value.get("worktreeMeta"), hints, "orcaCreatedAt provenance");
    collect_identity_worktrees(&value, hints);
    let mut cleaned = value.clone();
    let orphaned = remove_orphaned_metadata(&mut cleaned);
    if orphaned > 0 {
        let mut replacement = match serde_json::to_vec_pretty(&cleaned) {
            Ok(replacement) => replacement,
            Err(error) => {
                report
                    .warnings
                    .push(format!("could not serialize cleaned Orca state: {error}"));
                return;
            }
        };
        replacement.push(b'\n');
        add_rewrite(
            report,
            path.to_owned(),
            &bytes,
            replacement,
            RewriteSpec {
                kind: ArtifactKind::OrphanedState,
                safety: Safety::Automatic,
                description: "Remove stale worktree metadata from Orca state".to_owned(),
                evidence: format!(
                    "{orphaned} provenance-bearing worktree record(s) point to missing paths"
                ),
                id_prefix: "rewrite-orphaned-worktree-state",
                scope: ScopeKind::Local,
                format: FileFormat::Json,
                mutation_count: orphaned,
            },
        );
    }
    add_path(
        report,
        path,
        FindingSpec {
            kind: ArtifactKind::RuntimeState,
            ownership: Ownership::ProviderOwned,
            safety: Safety::Informational,
            description: "Orca persistent application state",
            evidence: "orca-data.json is Orca's primary state store and is never treated as cache",
            id_prefix: "orca-state",
            scope: ScopeKind::Local,
            actionable: false,
        },
    );
}

fn remove_orphaned_metadata(value: &mut Value) -> usize {
    let mut removed = 0;
    if let Some(map) = value.get_mut("worktreeMeta").and_then(Value::as_object_mut) {
        map.retain(|identity, metadata| {
            let missing = identity
                .split_once("::")
                .is_some_and(|(_, path)| !Path::new(path).exists());
            let attributed = is_attributed_metadata(metadata);
            let keep = !(missing && attributed);
            if !keep {
                removed += 1;
            }
            keep
        });
    }
    let identity_metadata = value
        .get("worktreeMetaByIdentity")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let mut removed_identity_keys = BTreeSet::new();
    if let Some(aliases) = value
        .get_mut("worktreeIdentityAliases")
        .and_then(Value::as_object_mut)
    {
        aliases.retain(|alias, keys| {
            let worktree_id = alias.split_once('|').map_or(alias.as_str(), |(_, id)| id);
            let missing = worktree_id
                .split_once("::")
                .is_some_and(|(_, path)| !Path::new(path).exists());
            let attributed = keys
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .filter_map(|key| identity_metadata.get(key))
                .any(is_attributed_metadata);
            if missing && attributed {
                if let Some(keys) = keys.as_array() {
                    removed_identity_keys
                        .extend(keys.iter().filter_map(Value::as_str).map(str::to_owned));
                }
                removed += 1;
                false
            } else {
                true
            }
        });
    }
    let referenced = value
        .get("worktreeIdentityAliases")
        .and_then(Value::as_object)
        .into_iter()
        .flat_map(|aliases| aliases.values())
        .filter_map(Value::as_array)
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    if let Some(metadata) = value
        .get_mut("worktreeMetaByIdentity")
        .and_then(Value::as_object_mut)
    {
        metadata.retain(|key, item| {
            !removed_identity_keys.contains(key)
                || referenced.contains(key)
                || !is_attributed_metadata(item)
        });
    }
    removed
}

fn collect_settings(value: &Value, hints: &mut StateHints) {
    let Some(settings) = value.get("settings").and_then(Value::as_object) else {
        return;
    };
    add_string_path(settings.get("workspaceDir"), &mut hints.workspace_roots);
    if let Some(history) = settings
        .get("workspaceDirHistory")
        .and_then(Value::as_array)
    {
        for item in history {
            add_string_path(Some(item), &mut hints.workspace_roots);
        }
    }
}

fn collect_repositories(value: &Value, hints: &mut StateHints) {
    let Some(repos) = value.get("repos").and_then(Value::as_array) else {
        return;
    };
    for repo in repos.iter().filter_map(Value::as_object) {
        let (Some(id), Some(path)) = (
            repo.get("id").and_then(Value::as_str),
            repo.get("path").and_then(Value::as_str),
        ) else {
            continue;
        };
        let repo_path = PathBuf::from(path);
        hints.repos.insert(id.to_owned(), repo_path.clone());
        if let Some(base) = repo.get("worktreeBasePath").and_then(Value::as_str) {
            let base = PathBuf::from(base);
            hints.workspace_roots.insert(if base.is_absolute() {
                base
            } else {
                repo_path.join(base)
            });
        }
    }
}

fn collect_worktree_map(value: Option<&Value>, hints: &mut StateHints, source: &'static str) {
    let Some(map) = value.and_then(Value::as_object) else {
        return;
    };
    for (identity, metadata) in map {
        let Some((repo_id, raw_path)) = identity.split_once("::") else {
            continue;
        };
        let attributed = is_attributed_metadata(metadata);
        if !attributed {
            continue;
        }
        let path = PathBuf::from(raw_path);
        if let Some(parent) = path.parent() {
            hints.workspace_roots.insert(parent.to_owned());
        }
        hints.worktrees.push(WorktreeHint {
            path,
            repo_id: repo_id.to_owned(),
            source,
        });
    }
}

fn collect_identity_worktrees(value: &Value, hints: &mut StateHints) {
    let (Some(aliases), Some(metadata)) = (
        value
            .get("worktreeIdentityAliases")
            .and_then(Value::as_object),
        value
            .get("worktreeMetaByIdentity")
            .and_then(Value::as_object),
    ) else {
        return;
    };
    for (alias, identity_keys) in aliases {
        let worktree_id = alias.split_once('|').map_or(alias.as_str(), |(_, id)| id);
        let Some((repo_id, raw_path)) = worktree_id.split_once("::") else {
            continue;
        };
        let attributed = identity_keys
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .filter_map(|key| metadata.get(key))
            .any(is_attributed_metadata);
        if !attributed {
            continue;
        }
        let path = PathBuf::from(raw_path);
        if let Some(parent) = path.parent() {
            hints.workspace_roots.insert(parent.to_owned());
        }
        hints.worktrees.push(WorktreeHint {
            path,
            repo_id: repo_id.to_owned(),
            source: "worktree identity provenance",
        });
    }
}

fn is_attributed_metadata(metadata: &Value) -> bool {
    metadata.get("orcaCreatedAt").is_some() || metadata.get("orcaCreationSource").is_some()
}

fn add_string_path(value: Option<&Value>, paths: &mut BTreeSet<PathBuf>) {
    if let Some(path) = value
        .and_then(Value::as_str)
        .filter(|path| !path.is_empty())
    {
        paths.insert(PathBuf::from(path));
    }
}
