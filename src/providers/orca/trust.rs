use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use jsonc_parser::ParseOptions;
use jsonc_parser::cst::CstRootNode;
use toml_edit::DocumentMut;

use crate::model::{ArtifactKind, FileFormat, Ownership, Safety, ScanReport};
use crate::provider::{HomeScope, ScanContext};

use super::state::StateHints;
use super::support::{FindingSpec, RewriteSpec, add_path, add_rewrite, resolve_file};

pub(crate) fn inspect(
    context: &ScanContext,
    data_dirs: &[PathBuf],
    hints: &StateHints,
    report: &mut ScanReport,
) {
    let candidates = candidates(context, hints);
    for scope in context.home_scopes() {
        inspect_cursor(&scope, &candidates, report);
        inspect_copilot(&scope, &candidates, report);
        inspect_codex(
            &scope,
            &scope.home.join(".codex/config.toml"),
            &candidates,
            report,
        );
    }
    for data_dir in data_dirs {
        inspect_codex(
            &HomeScope {
                home: context.home.clone(),
                kind: crate::model::ScopeKind::Local,
            },
            &data_dir.join("codex-runtime-home/home/config.toml"),
            &candidates,
            report,
        );
    }
}

fn candidates(context: &ScanContext, hints: &StateHints) -> BTreeSet<PathBuf> {
    let mut result = BTreeSet::new();
    result.extend(hints.repos.values().cloned());
    result.extend(hints.worktrees.iter().map(|hint| hint.path.clone()));
    result.extend(hints.workspace_roots.iter().cloned());
    result.extend(context.workspace_roots.iter().cloned());
    for scope in context.home_scopes() {
        result.insert(scope.home.join("orca/workspaces"));
    }
    result
}

fn is_attributed(path: &Path, candidates: &BTreeSet<PathBuf>) -> bool {
    candidates
        .iter()
        .any(|candidate| same_path(path, candidate) || path.starts_with(candidate))
}

fn inspect_cursor(scope: &HomeScope, candidates: &BTreeSet<PathBuf>, report: &mut ScanReport) {
    let projects = scope.home.join(".cursor/projects");
    let Ok(entries) = fs::read_dir(projects) else {
        return;
    };
    for entry in entries.filter_map(Result::ok).take(4096) {
        let marker = entry.path().join(".workspace-trusted");
        let Ok(bytes) = fs::read(&marker) else {
            continue;
        };
        let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
            report.warnings.push(format!(
                "could not parse Cursor trust marker {}",
                marker.display()
            ));
            continue;
        };
        let Some(workspace) = value
            .get("workspacePath")
            .and_then(serde_json::Value::as_str)
            .map(PathBuf::from)
        else {
            continue;
        };
        if !is_attributed(&workspace, candidates) {
            continue;
        }
        let orphaned = !workspace.exists();
        add_path(
            report,
            &marker,
            FindingSpec {
                kind: ArtifactKind::TrustEntry,
                ownership: Ownership::Attributed,
                safety: if orphaned {
                    Safety::Automatic
                } else {
                    Safety::ReviewRequired
                },
                description: "Cursor workspace trust marker induced by Orca",
                evidence: if orphaned {
                    "the marker names an Orca-attributed workspace that no longer exists"
                } else {
                    "the marker payload names a workspace recorded or rooted by Orca"
                },
                id_prefix: "remove-cursor-trust",
                scope: scope.kind,
                actionable: true,
            },
        );
    }
}

fn inspect_copilot(scope: &HomeScope, candidates: &BTreeSet<PathBuf>, report: &mut ScanReport) {
    let configured = scope.home.join(".copilot/config.json");
    let Some(path) = resolve_file(&configured, report) else {
        return;
    };
    let Ok(bytes) = fs::read(&path) else {
        return;
    };
    let Ok(text) = std::str::from_utf8(&bytes) else {
        return;
    };
    let root = match CstRootNode::parse(text, &ParseOptions::default()) {
        Ok(root) => root,
        Err(error) => {
            report.warnings.push(format!(
                "could not parse Copilot config {}: {error}",
                path.display()
            ));
            return;
        }
    };
    let Some(object) = root.value().and_then(|value| value.as_object()) else {
        return;
    };
    let Some(property) = object.get("trustedFolders") else {
        return;
    };
    let Some(array) = property.value().and_then(|value| value.as_array()) else {
        return;
    };
    let mut count = 0;
    let mut all_orphaned = true;
    for element in array.elements() {
        let Some(folder) = element
            .as_string_lit()
            .and_then(|value| value.decoded_value().ok())
            .map(PathBuf::from)
        else {
            continue;
        };
        if is_attributed(&folder, candidates) {
            all_orphaned &= !folder.exists();
            element.remove();
            count += 1;
        }
    }
    if count == 0 {
        return;
    }
    if array.elements().is_empty() {
        property.remove();
    }
    add_rewrite(
        report,
        path,
        &bytes,
        root.to_string().into_bytes(),
        RewriteSpec {
            kind: ArtifactKind::TrustEntry,
            safety: if all_orphaned {
                Safety::Automatic
            } else {
                Safety::ReviewRequired
            },
            description: "Remove Orca-attributed folders from Copilot trust".to_owned(),
            evidence: format!(
                "{count} trustedFolders entry/entries match Orca workspace provenance"
            ),
            id_prefix: "rewrite-copilot-trust",
            scope: scope.kind,
            format: FileFormat::Json,
            mutation_count: count,
        },
    );
}

fn inspect_codex(
    scope: &HomeScope,
    configured: &Path,
    candidates: &BTreeSet<PathBuf>,
    report: &mut ScanReport,
) {
    let Some(path) = resolve_file(configured, report) else {
        return;
    };
    let Ok(bytes) = fs::read(&path) else {
        return;
    };
    let Ok(text) = std::str::from_utf8(&bytes) else {
        return;
    };
    let mut document = match text.parse::<DocumentMut>() {
        Ok(document) => document,
        Err(error) => {
            report.warnings.push(format!(
                "could not parse Codex TOML {}: {error}",
                path.display()
            ));
            return;
        }
    };
    let mut project_mutations = 0;
    let mut any_live_project = false;
    if let Some(projects) = document
        .get_mut("projects")
        .and_then(toml_edit::Item::as_table_like_mut)
    {
        let keys = projects
            .iter()
            .filter_map(|(key, value)| {
                let attributed = is_attributed(Path::new(key), candidates);
                let trusted = value
                    .as_table_like()
                    .and_then(|table| table.get("trust_level"))
                    .and_then(toml_edit::Item::as_value)
                    .and_then(toml_edit::Value::as_str)
                    == Some("trusted");
                (attributed && trusted).then(|| key.to_owned())
            })
            .collect::<Vec<_>>();
        for key in keys {
            any_live_project |= Path::new(&key).exists();
            if let Some(table) = projects
                .get_mut(&key)
                .and_then(toml_edit::Item::as_table_like_mut)
            {
                table.remove("trust_level");
                project_mutations += 1;
                if table.is_empty() {
                    projects.remove(&key);
                }
            }
        }
    }
    let hook_mutations = remove_managed_hook_trust(&mut document);
    let count = project_mutations + hook_mutations;
    if count == 0 {
        return;
    }
    let safety = if project_mutations > 0 && any_live_project {
        Safety::ReviewRequired
    } else {
        Safety::Automatic
    };
    add_rewrite(
        report,
        path,
        &bytes,
        document.to_string().into_bytes(),
        RewriteSpec {
            kind: ArtifactKind::TrustEntry,
            safety,
            description: "Remove Orca-attributed Codex trust state".to_owned(),
            evidence: format!(
                "{project_mutations} project trust entry/entries match Orca workspace provenance; {hook_mutations} hook trust block(s) reference Orca scripts"
            ),
            id_prefix: "rewrite-codex-trust",
            scope: scope.kind,
            format: FileFormat::Toml,
            mutation_count: count,
        },
    );
}

fn remove_managed_hook_trust(document: &mut DocumentMut) -> usize {
    let Some(state) = document
        .get_mut("hooks")
        .and_then(toml_edit::Item::as_table_like_mut)
        .and_then(|hooks| hooks.get_mut("state"))
        .and_then(toml_edit::Item::as_table_like_mut)
    else {
        return 0;
    };
    let keys = state
        .iter()
        .filter(|(_, value)| {
            let normalized = value.to_string().to_ascii_lowercase().replace('\\', "/");
            normalized.contains("/.orca/agent-hooks/") || normalized.contains("orca/agent-hooks/")
        })
        .map(|(key, _)| key.to_owned())
        .collect::<Vec<_>>();
    let count = keys.len();
    for key in keys {
        state.remove(&key);
    }
    count
}

fn same_path(left: &Path, right: &Path) -> bool {
    match (fs::canonicalize(left), fs::canonicalize(right)) {
        (Ok(left), Ok(right)) => left == right,
        _ => left == right,
    }
}
