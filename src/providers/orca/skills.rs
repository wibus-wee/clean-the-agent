use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::Value;
use sha1::{Digest, Sha1};

use crate::model::{ArtifactKind, Ownership, Safety, ScanReport};
use crate::provider::{HomeScope, ScanContext};

use super::support::{FindingSpec, add_path};

const SKILL_LOCK_VERSION: u64 = 3;
const ORCA_SOURCE: &str = "stablyai/orca";
const ORCA_SOURCE_URL: &str = "https://github.com/stablyai/orca.git";
const KNOWLEDGE_APP_VERSION: &str = "1.4.197";
const KNOWLEDGE_COMMIT: &str = "b79206533ea983620ea20378b6b1116ce2cd5bde";

// Global provider roots from stablyai/orca at KNOWLEDGE_COMMIT. Codex reads
// ~/.agents/skills directly and therefore has no separate placement.
const PROVIDER_ROOTS: &[&str] = &[
    ".claude/skills",
    ".cursor/skills",
    ".gemini/skills",
    ".factory/skills",
    ".continue/skills",
    ".trae-cn/skills",
    ".grok/skills",
    ".augment/skills",
];

#[derive(Clone, Copy)]
struct KnownSkillRevision {
    name: &'static str,
    revision: u64,
    git_tree_sha: &'static str,
    first_app_version: &'static str,
}

#[derive(Deserialize)]
struct SkillLock {
    version: u64,
    skills: BTreeMap<String, SkillLockEntry>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SkillLockEntry {
    source: String,
    source_type: String,
    source_url: String,
    skill_path: String,
    skill_folder_hash: String,
}

pub(crate) fn inspect(context: &ScanContext, data_dirs: &[PathBuf], report: &mut ScanReport) {
    warn_for_newer_orca(data_dirs, report);
    for scope in context.home_scopes() {
        inspect_scope(&scope, report);
    }
}

fn inspect_scope(scope: &HomeScope, report: &mut ScanReport) {
    let lock_path = scope.home.join(".agents/.skill-lock.json");
    let bytes = match fs::read(&lock_path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return,
        Err(error) => {
            report.warnings.push(format!(
                "could not read skill provenance lock {}: {error}; Orca skill placements were skipped",
                lock_path.display()
            ));
            return;
        }
    };
    let Ok(lock) = serde_json::from_slice::<SkillLock>(&bytes) else {
        report.warnings.push(format!(
            "could not parse skill provenance lock {}; Orca skill placements were skipped",
            lock_path.display()
        ));
        return;
    };
    if lock.version != SKILL_LOCK_VERSION {
        report.warnings.push(format!(
            "skill provenance lock {} uses unsupported schema version {}; Orca skill placements were skipped",
            lock_path.display(),
            lock.version
        ));
        return;
    }

    for (name, entry) in lock.skills {
        if !is_orca_source(&entry) {
            continue;
        }
        let expected_skill_path = format!("skills/{name}/SKILL.md");
        if entry.skill_path != expected_skill_path {
            warn_unknown_orca_skill(report, &name, &entry.skill_folder_hash);
            continue;
        }
        let Some(revision) = known_revision(&name, &entry.skill_folder_hash) else {
            warn_unknown_orca_skill(report, &name, &entry.skill_folder_hash);
            continue;
        };
        let canonical = scope.home.join(".agents/skills").join(&name);
        let observed = match single_file_skill_tree_sha(&canonical) {
            Ok(hash) => hash,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => {
                report.warnings.push(format!(
                    "could not verify locked Orca skill {}: {error}; provider placements were left untouched",
                    canonical.display()
                ));
                continue;
            }
        };
        if observed != entry.skill_folder_hash {
            report.warnings.push(format!(
                "locked Orca skill {} was modified after installation; provider placements were left untouched",
                canonical.display()
            ));
            continue;
        }
        inspect_provider_placements(scope, &canonical, &revision, report);
    }
}

fn is_orca_source(entry: &SkillLockEntry) -> bool {
    entry.source_type == "github"
        && entry.source.eq_ignore_ascii_case(ORCA_SOURCE)
        && entry.source_url.eq_ignore_ascii_case(ORCA_SOURCE_URL)
}

fn inspect_provider_placements(
    scope: &HomeScope,
    canonical: &Path,
    revision: &KnownSkillRevision,
    report: &mut ScanReport,
) {
    for relative_root in PROVIDER_ROOTS {
        let placement = scope.home.join(relative_root).join(revision.name);
        let Ok(metadata) = fs::symlink_metadata(&placement) else {
            continue;
        };
        let exact_alias = metadata.file_type().is_symlink()
            && fs::canonicalize(&placement).ok() == fs::canonicalize(canonical).ok();
        let exact_copy = metadata.is_dir()
            && single_file_skill_tree_sha(&placement).ok().as_deref()
                == Some(revision.git_tree_sha);
        if !exact_alias && !exact_copy {
            continue;
        }
        let evidence = format!(
            "the skills CLI lock records stablyai/orca, disk bytes match official skill {} revision {} (first recorded with Orca {}), and this provider placement is an exact {}",
            revision.name,
            revision.revision,
            revision.first_app_version,
            if exact_alias { "alias" } else { "copy" }
        );
        add_path(
            report,
            &placement,
            FindingSpec {
                kind: ArtifactKind::SkillPlacement,
                ownership: Ownership::Attributed,
                safety: Safety::Automatic,
                description: "Version-matched Orca provider skill placement",
                evidence: &evidence,
                id_prefix: "remove-skill-placement",
                scope: scope.kind,
                actionable: true,
            },
        );
    }
}

fn warn_unknown_orca_skill(report: &mut ScanReport, name: &str, git_tree_sha: &str) {
    report.warnings.push(format!(
        "Orca skill {name} has unrecognized source revision {git_tree_sha}; detector knowledge ends at Orca {KNOWLEDGE_APP_VERSION} source commit {KNOWLEDGE_COMMIT}, so placements were left untouched and artifacts introduced by a newer Orca may be missing from this scan"
    ));
}

fn warn_for_newer_orca(data_dirs: &[PathBuf], report: &mut ScanReport) {
    for data_dir in data_dirs {
        let Ok(entries) = fs::read_dir(data_dir.join("daemon")) else {
            continue;
        };
        for entry in entries.filter_map(Result::ok).take(128) {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if !name.starts_with("daemon-v") || !name.ends_with(".pid") {
                continue;
            }
            let Some(version) = fs::read(entry.path())
                .ok()
                .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
                .and_then(|value| value.get("appVersion")?.as_str().map(str::to_owned))
            else {
                continue;
            };
            if version_core(&version).is_some_and(|core| core > (1, 4, 197)) {
                report.warnings.push(format!(
                    "observed Orca {version}, newer than this detector's {KNOWLEDGE_APP_VERSION} source snapshot ({KNOWLEDGE_COMMIT}); this scan may not cover every artifact introduced by the newer Orca version"
                ));
                return;
            }
        }
    }
}

fn version_core(version: &str) -> Option<(u64, u64, u64)> {
    let mut parts = version.trim_start_matches('v').split(['.', '-']);
    Some((
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
    ))
}

// Orca's versioned skill registry through KNOWLEDGE_COMMIT contains exactly
// one non-executable SKILL.md per snapshot. Matching that strict shape avoids
// treating sidecars or user additions as official bytes.
fn single_file_skill_tree_sha(directory: &Path) -> std::io::Result<String> {
    let mut entries = fs::read_dir(directory)?.collect::<Result<Vec<_>, _>>()?;
    if entries.len() != 1 || entries[0].file_name() != "SKILL.md" {
        return Err(std::io::Error::other(
            "skill directory does not match the one-file Orca snapshot shape",
        ));
    }
    let path = entries.pop().expect("one entry was checked").path();
    let metadata = fs::symlink_metadata(&path)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(std::io::Error::other("SKILL.md is not a regular file"));
    }
    let bytes = fs::read(path)?;
    if bytes.len() > 8 * 1024 * 1024 {
        return Err(std::io::Error::other(
            "SKILL.md exceeds the verification limit",
        ));
    }
    let blob = git_object_sha("blob", &bytes);
    let mut tree = b"100644 SKILL.md\0".to_vec();
    tree.extend_from_slice(&blob);
    Ok(hex_sha1(git_object_sha("tree", &tree)))
}

fn git_object_sha(kind: &str, bytes: &[u8]) -> [u8; 20] {
    let mut hasher = Sha1::new();
    hasher.update(format!("{kind} {}\0", bytes.len()).as_bytes());
    hasher.update(bytes);
    hasher.finalize().into()
}

fn hex_sha1(bytes: [u8; 20]) -> String {
    let mut hex = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(&mut hex, "{byte:02x}").expect("writing to a String cannot fail");
    }
    hex
}

fn known_revision(name: &str, git_tree_sha: &str) -> Option<KnownSkillRevision> {
    KNOWN_SKILL_REVISIONS
        .lines()
        .filter_map(parse_known_revision)
        .find(|revision| revision.name == name && revision.git_tree_sha == git_tree_sha)
}

fn parse_known_revision(line: &'static str) -> Option<KnownSkillRevision> {
    let mut fields = line.split('\t');
    let revision = KnownSkillRevision {
        name: fields.next()?,
        revision: fields.next()?.parse().ok()?,
        git_tree_sha: fields.next()?,
        first_app_version: fields.next()?,
    };
    fields.next().is_none().then_some(revision)
}

// Generated from resources/skills/snapshot-registry.json and
// resources/skills/release-mapping.json in stablyai/orca at KNOWLEDGE_COMMIT.
const KNOWN_SKILL_REVISIONS: &str = include_str!("orca-skill-revisions.tsv");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computes_the_same_one_file_tree_identity_as_git() {
        let directory = tempfile::TempDir::new().unwrap();
        fs::write(directory.path().join("SKILL.md"), b"hello\n").unwrap();

        assert_eq!(
            single_file_skill_tree_sha(directory.path()).unwrap(),
            "456850a02c40a8f6f5c712a17f7a0af65b0e9a79"
        );
    }

    #[test]
    fn rejects_metadata_and_sidecars() {
        let directory = tempfile::TempDir::new().unwrap();
        fs::write(directory.path().join("SKILL.md"), b"skill").unwrap();
        fs::write(directory.path().join(".DS_Store"), b"metadata").unwrap();

        assert!(single_file_skill_tree_sha(directory.path()).is_err());
    }

    #[test]
    fn compares_only_the_numeric_version_core() {
        assert_eq!(version_core("v1.4.198-rc.1"), Some((1, 4, 198)));
        assert_eq!(version_core("not-a-version"), None);
    }

    #[test]
    fn generated_revision_catalog_is_well_formed() {
        assert_eq!(KNOWN_SKILL_REVISIONS.lines().count(), 90);
        assert!(
            KNOWN_SKILL_REVISIONS
                .lines()
                .all(|line| parse_known_revision(line).is_some())
        );
    }
}
