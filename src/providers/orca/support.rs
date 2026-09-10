use std::fs;
use std::path::{Path, PathBuf};

use crate::model::{
    ArtifactKind, CleanupAction, CleanupActionKind, FileFormat, Finding, Ownership, Safety,
    ScanReport, ScopeKind,
};
use crate::operations::{sha256_bytes, snapshot_path};
use sha2::{Digest, Sha256};

#[derive(Clone, Copy)]
pub(crate) struct FindingSpec<'a> {
    pub kind: ArtifactKind,
    pub ownership: Ownership,
    pub safety: Safety,
    pub description: &'a str,
    pub evidence: &'a str,
    pub id_prefix: &'a str,
    pub scope: ScopeKind,
    pub actionable: bool,
}

pub(crate) fn add_path(report: &mut ScanReport, path: &Path, spec: FindingSpec<'_>) {
    match snapshot_path(path) {
        Ok(snapshot) => {
            let id = finding_id(spec.id_prefix, path);
            let reclaimable_bytes = snapshot.bytes;
            let action = spec.actionable.then(|| CleanupAction {
                id: id.clone(),
                provider: "orca".to_owned(),
                description: spec.description.to_owned(),
                path: path.to_owned(),
                scope: spec.scope,
                safety: spec.safety,
                reclaimable_bytes,
                kind: CleanupActionKind::RemovePath { expected: snapshot },
                depends_on: Vec::new(),
            });
            report.findings.push(Finding {
                id,
                provider: "orca".to_owned(),
                kind: spec.kind,
                ownership: spec.ownership,
                safety: spec.safety,
                path: path.to_owned(),
                scope: spec.scope,
                description: spec.description.to_owned(),
                evidence: spec.evidence.to_owned(),
                reclaimable_bytes,
                action,
            });
        }
        Err(error) => report.warnings.push(format!(
            "could not snapshot {} for guarded cleanup: {error}",
            path.display()
        )),
    }
}

pub(crate) struct RewriteSpec<'a> {
    pub kind: ArtifactKind,
    pub safety: Safety,
    pub description: String,
    pub evidence: String,
    pub id_prefix: &'a str,
    pub scope: ScopeKind,
    pub format: FileFormat,
    pub mutation_count: usize,
}

pub(crate) fn add_rewrite(
    report: &mut ScanReport,
    path: PathBuf,
    original: &[u8],
    replacement: Vec<u8>,
    spec: RewriteSpec<'_>,
) {
    let id = finding_id(spec.id_prefix, &path);
    let action = CleanupAction {
        id: id.clone(),
        provider: "orca".to_owned(),
        description: spec.description.clone(),
        path: path.clone(),
        scope: spec.scope,
        safety: spec.safety,
        reclaimable_bytes: original.len().saturating_sub(replacement.len()) as u64,
        kind: CleanupActionKind::RewriteFile {
            expected_sha256: sha256_bytes(original),
            mutation_count: spec.mutation_count,
            format: spec.format,
            replacement,
        },
        depends_on: Vec::new(),
    };
    report.findings.push(Finding {
        id,
        provider: "orca".to_owned(),
        kind: spec.kind,
        ownership: Ownership::InjectedByProvider,
        safety: spec.safety,
        path,
        scope: spec.scope,
        description: spec.description,
        evidence: spec.evidence,
        reclaimable_bytes: action.reclaimable_bytes,
        action: Some(action),
    });
}

pub(crate) fn resolve_file(path: &Path, report: &mut ScanReport) -> Option<PathBuf> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => fs::canonicalize(path)
            .map_err(|error| {
                report.warnings.push(format!(
                    "could not resolve configuration symlink {}: {error}",
                    path.display()
                ));
            })
            .ok(),
        Ok(_) => Some(path.to_owned()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => {
            report
                .warnings
                .push(format!("could not inspect {}: {error}", path.display()));
            None
        }
    }
}

pub(crate) fn finding_id(prefix: &str, path: &Path) -> String {
    let digest = sha256_bytes(path.as_os_str().as_encoded_bytes());
    format!("{prefix}-{}", &digest[..12])
}

pub(crate) fn add_empty_dir(
    report: &mut ScanReport,
    path: &Path,
    kind: ArtifactKind,
    scope: ScopeKind,
    description: &str,
) {
    if !path.is_dir() {
        return;
    }
    let id = finding_id("remove-empty-directory", path);
    report.findings.push(Finding {
        id: id.clone(),
        provider: "orca".to_owned(),
        kind,
        ownership: Ownership::ProviderOwned,
        safety: Safety::Automatic,
        path: path.to_owned(),
        scope,
        description: description.to_owned(),
        evidence: "the directory is removed only if it is empty at apply time".to_owned(),
        reclaimable_bytes: 0,
        action: Some(CleanupAction {
            id,
            provider: "orca".to_owned(),
            description: description.to_owned(),
            path: path.to_owned(),
            scope,
            safety: Safety::Automatic,
            reclaimable_bytes: 0,
            kind: CleanupActionKind::RemoveEmptyDirectory,
            depends_on: Vec::new(),
        }),
    });
}

pub(crate) fn is_older_than(path: &Path, days: u64) -> bool {
    fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(|modified| modified.elapsed().ok())
        .is_some_and(|age| age.as_secs() >= days.saturating_mul(86_400))
}

pub(crate) fn scope_for_path(path: &Path, context: &crate::provider::ScanContext) -> ScopeKind {
    context
        .additional_homes
        .iter()
        .find(|scope| path.starts_with(&scope.home))
        .map_or(ScopeKind::Local, |scope| scope.kind)
}

pub(crate) fn content_equal(left: &Path, right: &Path) -> bool {
    content_digest(left)
        .ok()
        .zip(content_digest(right).ok())
        .is_some_and(|(a, b)| a == b)
}

fn content_digest(root: &Path) -> std::io::Result<String> {
    fn visit(
        root: &Path,
        path: &Path,
        records: &mut Vec<(PathBuf, Vec<u8>)>,
    ) -> std::io::Result<()> {
        let metadata = fs::symlink_metadata(path)?;
        let relative = path.strip_prefix(root).unwrap_or(path).to_owned();
        if metadata.file_type().is_symlink() {
            records.push((
                relative,
                fs::read_link(path)?.as_os_str().as_encoded_bytes().to_vec(),
            ));
        } else if metadata.is_file() {
            records.push((relative, fs::read(path)?));
        } else if metadata.is_dir() {
            records.push((relative, b"directory".to_vec()));
            let mut entries = fs::read_dir(path)?.collect::<Result<Vec<_>, _>>()?;
            entries.sort_by_key(fs::DirEntry::file_name);
            for entry in entries {
                visit(root, &entry.path(), records)?;
            }
        }
        Ok(())
    }
    let mut records = Vec::new();
    visit(root, root, &mut records)?;
    let mut hasher = Sha256::new();
    for (path, bytes) in records {
        hasher.update(path.as_os_str().as_encoded_bytes());
        hasher.update([0]);
        hasher.update(bytes);
        hasher.update([0xff]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}
