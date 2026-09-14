use std::fs;
use std::path::Path;

use clean_any::model::{ApplyStatus, CleanupActionKind, CleanupPlan, FilePrecondition};
use clean_any::tweaks::DisableCodexPetShortcut;
use clean_any::{Engine, Platform, ScanContext, TweakEngine, TweakReport, TweakStatus};
use serde_json::{Value, json};
use tempfile::TempDir;

fn context(home: &Path) -> ScanContext {
    ScanContext {
        home: home.to_owned(),
        platform: Platform::MacOs,
        orca_data_dirs: Vec::new(),
        workspace_roots: Vec::new(),
        honor_environment: false,
        additional_homes: Vec::new(),
        stale_after_days: 30,
    }
}

fn engine() -> TweakEngine {
    TweakEngine::new(vec![Box::new(DisableCodexPetShortcut)])
}

fn apply(report: &TweakReport) -> clean_any::ApplyReport {
    Engine::apply(&CleanupPlan {
        actions: vec![report.action.clone().expect("tweak needs an action")],
        excluded_review_findings: 0,
        reclaimable_bytes: 0,
    })
}

fn read_keymap(path: &Path) -> Vec<Value> {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

#[test]
fn missing_keymap_is_created_with_a_null_pet_binding() {
    let fixture = TempDir::new().unwrap();
    let home = fixture.path().join("home");
    let report = engine()
        .inspect("codex.disable-pet-shortcut", &context(&home))
        .unwrap();

    assert_eq!(report.status, TweakStatus::NeedsChange);
    assert!(matches!(
        report.action.as_ref().map(|action| &action.kind),
        Some(CleanupActionKind::EnsureFile {
            expected: FilePrecondition::Missing,
            ..
        })
    ));

    let result = apply(&report);
    assert!(!result.has_failures());
    assert_eq!(
        read_keymap(&home.join(".codex/keybindings.json")),
        vec![json!({"command": "openAvatarOverlay", "key": null})]
    );
}

#[test]
fn existing_keymap_preserves_unrelated_bindings_and_replaces_all_pet_bindings() {
    let fixture = TempDir::new().unwrap();
    let home = fixture.path().join("home");
    let path = home.join(".codex/keybindings.json");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(
        &path,
        serde_json::to_vec(&json!([
            {"command": "openAvatarOverlay", "key": "Command+P", "legacy": true},
            {"command": "otherCommand", "key": "Command+O", "future": true},
            {"command": "openAvatarOverlay", "key": "Control+P"}
        ]))
        .unwrap(),
    )
    .unwrap();

    let report = engine()
        .inspect("codex.disable-pet-shortcut", &context(&home))
        .unwrap();
    let result = apply(&report);

    assert!(!result.has_failures());
    let bindings = read_keymap(&path);
    assert_eq!(
        bindings
            .iter()
            .filter(|item| item["command"] == "openAvatarOverlay")
            .collect::<Vec<_>>(),
        vec![&json!({"command": "openAvatarOverlay", "key": null})]
    );
    assert!(bindings.contains(&json!({
        "command": "otherCommand",
        "key": "Command+O",
        "future": true
    })));
}

#[test]
fn null_pet_binding_is_idempotent() {
    let fixture = TempDir::new().unwrap();
    let home = fixture.path().join("home");
    let path = home.join(".codex/keybindings.json");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(
        &path,
        serde_json::to_vec_pretty(&json!([
            {"command": "openAvatarOverlay", "key": null},
            {"command": "otherCommand", "key": "Command+O"}
        ]))
        .unwrap(),
    )
    .unwrap();

    let report = engine()
        .inspect("codex.disable-pet-shortcut", &context(&home))
        .unwrap();

    assert_eq!(report.status, TweakStatus::Satisfied);
    assert!(report.action.is_none());
}

#[test]
fn malformed_keymap_is_blocked_and_unchanged() {
    let fixture = TempDir::new().unwrap();
    let home = fixture.path().join("home");
    let path = home.join(".codex/keybindings.json");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, b"{not-json").unwrap();

    let report = engine()
        .inspect("codex.disable-pet-shortcut", &context(&home))
        .unwrap();

    assert_eq!(report.status, TweakStatus::Blocked);
    assert!(report.action.is_none());
    assert_eq!(fs::read(path).unwrap(), b"{not-json");
}

#[test]
fn apply_rejects_a_keymap_created_after_inspection() {
    let fixture = TempDir::new().unwrap();
    let home = fixture.path().join("home");
    let path = home.join(".codex/keybindings.json");
    let report = engine()
        .inspect("codex.disable-pet-shortcut", &context(&home))
        .unwrap();
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, b"[]").unwrap();

    let result = apply(&report);

    assert_eq!(fs::read(&path).unwrap(), b"[]");
    assert!(result.results.iter().any(|item| {
        matches!(item.status, ApplyStatus::Failed) && item.detail.contains("appeared")
    }));
}

#[test]
fn apply_rejects_a_keymap_changed_after_inspection() {
    let fixture = TempDir::new().unwrap();
    let home = fixture.path().join("home");
    let path = home.join(".codex/keybindings.json");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, b"[]").unwrap();
    let report = engine()
        .inspect("codex.disable-pet-shortcut", &context(&home))
        .unwrap();
    fs::write(&path, br#"[{"command":"userCommand","key":"Command+U"}]"#).unwrap();

    let result = apply(&report);

    assert!(result.results.iter().any(|item| {
        matches!(item.status, ApplyStatus::Failed) && item.detail.contains("changed")
    }));
    assert_eq!(read_keymap(&path)[0]["command"], "userCommand");
}
