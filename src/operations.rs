use std::ffi::OsString;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::UNIX_EPOCH;

use atomic_write_file::AtomicWriteFile;
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::model::{CleanupAction, CleanupActionKind, FilePrecondition, PathSnapshot};

#[derive(Debug, Error)]
pub enum OperationError {
    #[error("{0}")]
    Io(#[from] io::Error),
    #[error("path changed after scanning; run scan again")]
    SnapshotChanged,
    #[error("configuration changed after scanning; run scan again")]
    ContentChanged,
    #[error("target appeared after scanning; run scan again")]
    TargetAppeared,
    #[error("worktree has uncommitted, untracked, or ignored files")]
    DirtyWorktree,
    #[error("worktree is no longer registered with Git")]
    UnregisteredWorktree,
    #[error("Git command failed: {0}")]
    Git(String),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApplyOutcome {
    Applied,
    Skipped,
}

/// Applies one generic action after checking its scan-time preconditions.
///
/// # Errors
///
/// Returns an error when the target changed, a worktree is dirty or no longer
/// registered, or an underlying filesystem or Git operation fails.
pub fn apply_action(action: &CleanupAction) -> Result<ApplyOutcome, OperationError> {
    match &action.kind {
        CleanupActionKind::RewriteFile {
            expected_sha256,
            replacement,
            ..
        } => rewrite_file(&action.path, expected_sha256, replacement),
        CleanupActionKind::EnsureFile {
            expected,
            replacement,
            ..
        } => ensure_file(&action.path, expected, replacement),
        CleanupActionKind::RemoveGitWorktree {
            repository,
            expected,
        } => remove_git_worktree(repository, &action.path, expected),
        CleanupActionKind::RemovePath { expected } => {
            remove_snapshotted_path(&action.path, expected)
        }
        CleanupActionKind::RemoveEmptyDirectory => remove_empty_directory(&action.path),
    }
}

#[must_use]
pub fn sha256_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

/// Captures a non-following metadata snapshot of a path and its descendants.
///
/// # Errors
///
/// Returns an error if any entry needed for a complete snapshot cannot be
/// inspected. Callers must not plan a removal from a partial snapshot.
pub fn snapshot_path(path: &Path) -> Result<PathSnapshot, OperationError> {
    let mut records = Vec::new();
    collect_snapshot_records(path, Path::new(""), &mut records)?;
    records.sort_by(|left, right| left.0.cmp(&right.0));

    let mut hasher = Sha256::new();
    let mut bytes = 0_u64;
    for (relative, record, file_bytes) in &records {
        hasher.update(relative.as_encoded_bytes());
        hasher.update([0]);
        hasher.update(record.as_bytes());
        hasher.update([0xff]);
        bytes = bytes.saturating_add(*file_bytes);
    }

    Ok(PathSnapshot {
        digest: format!("{:x}", hasher.finalize()),
        entries: records.len() as u64,
        bytes,
    })
}

fn collect_snapshot_records(
    path: &Path,
    relative: &Path,
    records: &mut Vec<(OsString, String, u64)>,
) -> Result<(), io::Error> {
    let metadata = fs::symlink_metadata(path)?;
    let file_type = metadata.file_type();
    let kind = if file_type.is_symlink() {
        "symlink"
    } else if file_type.is_dir() {
        "directory"
    } else if file_type.is_file() {
        "file"
    } else {
        "other"
    };
    let modified = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |duration| duration.as_nanos());
    let link_target = if file_type.is_symlink() {
        fs::read_link(path)
            .map(|target| target.to_string_lossy().into_owned())
            .unwrap_or_default()
    } else {
        String::new()
    };
    records.push((
        relative.as_os_str().to_owned(),
        format!("{kind}:{}:{modified}:{link_target}", metadata.len()),
        if file_type.is_file() {
            metadata.len()
        } else {
            0
        },
    ));

    if file_type.is_dir() && !file_type.is_symlink() {
        let mut children = fs::read_dir(path)?.collect::<Result<Vec<_>, _>>()?;
        children.sort_by_key(fs::DirEntry::file_name);
        for child in children {
            let child_relative = relative.join(child.file_name());
            collect_snapshot_records(&child.path(), &child_relative, records)?;
        }
    }
    Ok(())
}

fn rewrite_file(
    path: &Path,
    expected_sha256: &str,
    replacement: &[u8],
) -> Result<ApplyOutcome, OperationError> {
    let current = fs::read(path)?;
    if sha256_bytes(&current) != expected_sha256 {
        return Err(OperationError::ContentChanged);
    }

    let permissions = fs::metadata(path)?.permissions();
    let mut file = AtomicWriteFile::options().open(path)?;
    file.set_permissions(permissions)?;
    file.write_all(replacement)?;
    file.commit()?;
    Ok(ApplyOutcome::Applied)
}

fn ensure_file(
    path: &Path,
    expected: &FilePrecondition,
    replacement: &[u8],
) -> Result<ApplyOutcome, OperationError> {
    match expected {
        FilePrecondition::Matches(expected_sha256) => {
            rewrite_file(path, expected_sha256, replacement)
        }
        FilePrecondition::Missing => create_file(path, replacement),
    }
}

fn create_file(path: &Path, replacement: &[u8]) -> Result<ApplyOutcome, OperationError> {
    match fs::symlink_metadata(path) {
        Ok(_) => return Err(OperationError::TargetAppeared),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }

    let parent = path.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "target has no parent directory",
        )
    })?;
    fs::create_dir_all(parent)?;

    // Persisting without clobbering publishes complete contents while refusing
    // to replace a target created by an intervening writer.
    let mut file = tempfile::NamedTempFile::new_in(parent)?;
    file.write_all(replacement)?;
    file.as_file().sync_all()?;
    file.persist_noclobber(path).map_err(|error| {
        if error.error.kind() == io::ErrorKind::AlreadyExists {
            OperationError::TargetAppeared
        } else {
            error.error.into()
        }
    })?;
    Ok(ApplyOutcome::Applied)
}

fn remove_snapshotted_path(
    path: &Path,
    expected: &PathSnapshot,
) -> Result<ApplyOutcome, OperationError> {
    match snapshot_path(path) {
        Ok(current) if current == *expected => {}
        Ok(_) => return Err(OperationError::SnapshotChanged),
        Err(OperationError::Io(error)) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(ApplyOutcome::Skipped);
        }
        Err(error) => return Err(error),
    }

    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_dir() && !metadata.file_type().is_symlink() {
        fs::remove_dir_all(path)?;
    } else {
        fs::remove_file(path)?;
    }
    Ok(ApplyOutcome::Applied)
}

fn remove_empty_directory(path: &Path) -> Result<ApplyOutcome, OperationError> {
    match fs::remove_dir(path) {
        Ok(()) => Ok(ApplyOutcome::Applied),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(ApplyOutcome::Skipped),
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::DirectoryNotEmpty | io::ErrorKind::PermissionDenied
            ) =>
        {
            Ok(ApplyOutcome::Skipped)
        }
        Err(error) => Err(error.into()),
    }
}

fn remove_git_worktree(
    repository: &Path,
    worktree: &Path,
    expected: &PathSnapshot,
) -> Result<ApplyOutcome, OperationError> {
    match snapshot_path(worktree) {
        Ok(current) if current == *expected => {}
        Ok(_) => return Err(OperationError::SnapshotChanged),
        Err(OperationError::Io(error)) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(ApplyOutcome::Skipped);
        }
        Err(error) => return Err(error),
    }

    let status = git_output(
        worktree,
        &[
            "status",
            "--porcelain=v1",
            "--untracked-files=all",
            "--ignored=matching",
        ],
    )?;
    if !status.trim().is_empty() {
        return Err(OperationError::DirtyWorktree);
    }

    let list = git_output(repository, &["worktree", "list", "--porcelain"])?;
    let canonical_worktree = fs::canonicalize(worktree)?;
    let registered = parse_worktree_paths(&list)
        .into_iter()
        .any(|candidate| fs::canonicalize(candidate).is_ok_and(|path| path == canonical_worktree));
    if !registered {
        return Err(OperationError::UnregisteredWorktree);
    }

    let output = Command::new("git")
        .arg("-C")
        .arg(repository)
        .args(["worktree", "remove"])
        .arg(worktree)
        .output()?;
    if !output.status.success() {
        return Err(OperationError::Git(command_error(&output.stderr)));
    }
    Ok(ApplyOutcome::Applied)
}

fn git_output(directory: &Path, arguments: &[&str]) -> Result<String, OperationError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(directory)
        .args(arguments)
        .output()?;
    if !output.status.success() {
        return Err(OperationError::Git(command_error(&output.stderr)));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn command_error(stderr: &[u8]) -> String {
    let message = String::from_utf8_lossy(stderr).trim().to_owned();
    if message.is_empty() {
        "command exited unsuccessfully".to_owned()
    } else {
        message
    }
}

fn parse_worktree_paths(output: &str) -> Vec<PathBuf> {
    output
        .lines()
        .filter_map(|line| line.strip_prefix("worktree "))
        .map(PathBuf::from)
        .collect()
}
