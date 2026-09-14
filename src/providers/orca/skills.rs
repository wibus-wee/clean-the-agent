use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::model::{ArtifactKind, Ownership, Safety, ScanReport};
use crate::provider::{HomeScope, ScanContext};

use super::support::{FindingSpec, add_path, content_equal};

const PROVIDER_ROOTS: &[&str] = &[
    ".claude/skills",
    ".cursor/skills",
    ".gemini/skills",
    ".factory/skills",
    ".continue/skills",
    ".trae-cn/skills",
    ".trae/skills",
    ".grok/skills",
    ".augment/skills",
];

pub(crate) fn inspect(context: &ScanContext, data_dirs: &[PathBuf], report: &mut ScanReport) {
    let receipts = receipt_placements(data_dirs, report);
    for scope in context.home_scopes() {
        inspect_scope(&scope, &receipts, report);
    }
}

fn receipt_placements(data_dirs: &[PathBuf], report: &mut ScanReport) -> HashSet<PathBuf> {
    let mut placements = HashSet::new();
    for data_dir in data_dirs {
        let receipts = data_dir.join("skill-installs/receipts");
        let Ok(entries) = fs::read_dir(receipts) else {
            continue;
        };
        for entry in entries.filter_map(Result::ok).take(2048) {
            let path = entry.path();
            let Some(value): Option<Value> = fs::read(&path)
                .ok()
                .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            else {
                report
                    .warnings
                    .push(format!("could not parse skill receipt {}", path.display()));
                continue;
            };
            if value.get("schemaVersion").and_then(Value::as_u64) != Some(1) {
                continue;
            }
            if let Some(items) = value.get("placements").and_then(Value::as_array) {
                for item in items {
                    if matches!(
                        item.get("status").and_then(Value::as_str),
                        Some("installed" | "unchanged")
                    ) {
                        if let Some(path) = item.get("path").and_then(Value::as_str) {
                            placements.insert(PathBuf::from(path));
                        }
                    }
                }
            }
        }
    }
    placements
}

fn inspect_scope(scope: &HomeScope, receipts: &HashSet<PathBuf>, report: &mut ScanReport) {
    let canonical_root = scope.home.join(".agents/skills");
    let Ok(skills) = fs::read_dir(&canonical_root) else {
        return;
    };
    let skill_names = skills
        .filter_map(Result::ok)
        .map(|entry| entry.file_name())
        .collect::<Vec<_>>();
    for relative_root in PROVIDER_ROOTS {
        let provider_root = scope.home.join(relative_root);
        for name in &skill_names {
            let canonical = canonical_root.join(name);
            let placement = provider_root.join(name);
            if !placement.exists() && fs::symlink_metadata(&placement).is_err() {
                continue;
            }
            inspect_placement(
                scope,
                &canonical,
                &placement,
                receipts.contains(&placement),
                report,
            );
        }
    }
}

fn inspect_placement(
    scope: &HomeScope,
    canonical: &Path,
    placement: &Path,
    receipted: bool,
    report: &mut ScanReport,
) {
    let Ok(metadata) = fs::symlink_metadata(placement) else {
        return;
    };
    let points_to_canonical = metadata.file_type().is_symlink()
        && fs::canonicalize(placement).ok() == fs::canonicalize(canonical).ok();
    let identical_copy = !metadata.file_type().is_symlink() && content_equal(placement, canonical);
    let owned = receipted && (points_to_canonical || identical_copy);
    let attributed = points_to_canonical || identical_copy || receipted;
    if !attributed {
        return;
    }
    add_path(
        report,
        placement,
        FindingSpec {
            kind: ArtifactKind::SkillPlacement,
            ownership: if owned {
                Ownership::ProviderOwned
            } else {
                Ownership::Attributed
            },
            safety: if owned || points_to_canonical {
                Safety::Automatic
            } else {
                Safety::ReviewRequired
            },
            description: "Orca-created provider skill placement",
            evidence: if receipted {
                "an Orca install receipt records this path and it matches the canonical skill"
            } else if points_to_canonical {
                "the provider placement links exactly to the canonical .agents skill"
            } else {
                "the provider copy currently matches the canonical .agents skill, but no receipt was found"
            },
            id_prefix: "remove-skill-placement",
            scope: scope.kind,
            actionable: true,
        },
    );
}
