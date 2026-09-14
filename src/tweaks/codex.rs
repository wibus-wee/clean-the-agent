use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::model::{
    CleanupAction, CleanupActionKind, FileFormat, FilePrecondition, Safety, ScopeKind,
};
use crate::operations::sha256_bytes;
use crate::provider::ScanContext;
use crate::tweak::{Tweak, TweakReport, TweakStatus, TweakSummary};

const ID: &str = "codex.disable-pet-shortcut";
const COMMAND: &str = "openAvatarOverlay";

pub struct DisableCodexPetShortcut;

impl Tweak for DisableCodexPetShortcut {
    fn summary(&self) -> TweakSummary {
        TweakSummary {
            id: ID,
            product: "Codex",
            title: "Disable the Pet keyboard shortcut",
            description: "Keep Codex Pet available from menus while preventing keyboard activation",
        }
    }

    fn inspect(&self, context: &ScanContext) -> TweakReport {
        let configured_path = codex_home(context).join("keybindings.json");
        match read_existing(&configured_path) {
            Ok(None) => report(
                &configured_path,
                TweakStatus::NeedsChange,
                "Codex has no keybinding override file; a null binding will disable Pet",
                Some(action(
                    configured_path.clone(),
                    FilePrecondition::Missing,
                    disabled_keymap(&[]),
                    1,
                )),
            ),
            Ok(Some((path, bytes))) => inspect_existing(&path, &bytes),
            Err(detail) => report(&configured_path, TweakStatus::Blocked, &detail, None),
        }
    }
}

fn codex_home(context: &ScanContext) -> PathBuf {
    if context.honor_environment {
        if let Some(path) = context.environment_path("CODEX_HOME") {
            return path;
        }
    }
    context.home.join(".codex")
}

fn read_existing(path: &Path) -> Result<Option<(PathBuf, Vec<u8>)>, String> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("could not inspect {}: {error}", path.display())),
    };
    let resolved = if metadata.file_type().is_symlink() {
        fs::canonicalize(path)
            .map_err(|error| format!("could not resolve {}: {error}", path.display()))?
    } else {
        path.to_owned()
    };
    let bytes = fs::read(&resolved)
        .map_err(|error| format!("could not read {}: {error}", resolved.display()))?;
    Ok(Some((resolved, bytes)))
}

fn inspect_existing(path: &Path, bytes: &[u8]) -> TweakReport {
    let bindings = match parse_keymap(path, bytes) {
        Ok(bindings) => bindings,
        Err(detail) => return report(path, TweakStatus::Blocked, &detail, None),
    };
    let pet_bindings = bindings
        .iter()
        .filter(|binding| binding.get("command").and_then(Value::as_str) == Some(COMMAND))
        .collect::<Vec<_>>();
    if pet_bindings.len() == 1 && pet_bindings[0].get("key").is_some_and(Value::is_null) {
        return report(
            path,
            TweakStatus::Satisfied,
            "Codex Pet already has an explicit null keyboard binding",
            None,
        );
    }

    let mutation_count = pet_bindings.len().max(1);
    let replacement = disabled_keymap(&bindings);
    report(
        path,
        TweakStatus::NeedsChange,
        "Codex Pet does not have an explicit null keyboard binding",
        Some(action(
            path.to_owned(),
            FilePrecondition::Matches(sha256_bytes(bytes)),
            replacement,
            mutation_count,
        )),
    )
}

fn parse_keymap(path: &Path, bytes: &[u8]) -> Result<Vec<Value>, String> {
    let value: Value = serde_json::from_slice(bytes).map_err(|error| {
        format!(
            "could not parse Codex keybindings {}: {error}",
            path.display()
        )
    })?;
    let bindings = value.as_array().ok_or_else(|| {
        format!(
            "Codex keybindings {} must contain a JSON array",
            path.display()
        )
    })?;
    for (index, binding) in bindings.iter().enumerate() {
        let Some(object) = binding.as_object() else {
            return Err(format!(
                "Codex keybindings {} entry {index} must be an object",
                path.display()
            ));
        };
        if !object.get("command").is_some_and(Value::is_string)
            || !object
                .get("key")
                .is_some_and(|key| key.is_string() || key.is_null())
        {
            return Err(format!(
                "Codex keybindings {} entry {index} must have a string command and string-or-null key",
                path.display()
            ));
        }
    }
    Ok(bindings.clone())
}

fn disabled_keymap(bindings: &[Value]) -> Vec<u8> {
    let mut replacement = bindings
        .iter()
        .filter(|binding| binding.get("command").and_then(Value::as_str) != Some(COMMAND))
        .cloned()
        .collect::<Vec<_>>();
    replacement.push(json!({"command": COMMAND, "key": null}));
    replacement.sort_by(|left, right| {
        left.get("command")
            .and_then(Value::as_str)
            .cmp(&right.get("command").and_then(Value::as_str))
    });
    let mut bytes = serde_json::to_vec_pretty(&replacement).expect("JSON values always serialize");
    bytes.push(b'\n');
    bytes
}

fn action(
    path: PathBuf,
    expected: FilePrecondition,
    replacement: Vec<u8>,
    mutation_count: usize,
) -> CleanupAction {
    CleanupAction {
        id: "tweak-codex-disable-pet-shortcut".to_owned(),
        provider: "codex".to_owned(),
        description: "Disable the Codex Pet keyboard shortcut".to_owned(),
        path,
        scope: ScopeKind::Local,
        safety: Safety::ReviewRequired,
        reclaimable_bytes: 0,
        kind: CleanupActionKind::EnsureFile {
            expected,
            mutation_count,
            format: FileFormat::Json,
            replacement,
        },
        depends_on: Vec::new(),
    }
}

fn report(
    path: &Path,
    status: TweakStatus,
    detail: &str,
    action: Option<CleanupAction>,
) -> TweakReport {
    let summary = DisableCodexPetShortcut.summary();
    TweakReport {
        id: summary.id.to_owned(),
        product: summary.product.to_owned(),
        title: summary.title.to_owned(),
        description: summary.description.to_owned(),
        status,
        path: path.to_owned(),
        detail: detail.to_owned(),
        restart_required: action.is_some(),
        action,
    }
}
