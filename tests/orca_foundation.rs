use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use clean_any::model::{ApplyStatus, ArtifactKind, Safety};
use clean_any::providers::OrcaProvider;
use clean_any::{Engine, Platform, ScanContext};
use serde_json::{Value, json};
use tempfile::TempDir;

fn context(home: &Path, workspace_roots: Vec<PathBuf>) -> ScanContext {
    ScanContext {
        home: home.to_owned(),
        platform: Platform::Linux,
        orca_data_dirs: Vec::new(),
        workspace_roots,
        honor_environment: false,
        additional_homes: Vec::new(),
        stale_after_days: 30,
    }
}

fn engine() -> Engine {
    Engine::new(vec![Box::new(OrcaProvider)])
}

fn write_json(path: &Path, value: &Value) {
    fs::create_dir_all(path.parent().expect("fixture file has a parent")).unwrap();
    fs::write(path, serde_json::to_vec_pretty(value).unwrap()).unwrap();
}

fn capture_tree(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn visit(root: &Path, path: &Path, files: &mut BTreeMap<PathBuf, Vec<u8>>) {
        let mut entries = fs::read_dir(path)
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        entries.sort_by_key(fs::DirEntry::file_name);
        for entry in entries {
            let path = entry.path();
            if entry.file_type().unwrap().is_dir() {
                visit(root, &path, files);
            } else {
                files.insert(
                    path.strip_prefix(root).unwrap().to_owned(),
                    fs::read(path).unwrap(),
                );
            }
        }
    }

    let mut files = BTreeMap::new();
    visit(root, root, &mut files);
    files
}

#[test]
fn scan_is_read_only_and_classifies_representative_orca_state() {
    let fixture = TempDir::new().unwrap();
    let home = fixture.path().join("home");
    let workspace = fixture.path().join("workspaces");
    fs::create_dir_all(home.join(".orca/logs")).unwrap();
    fs::write(home.join(".orca/logs/orca.log"), "log data").unwrap();
    fs::create_dir_all(home.join(".orca/agent-hooks")).unwrap();
    fs::write(home.join(".orca/agent-hooks/claude-hook.sh"), "#!/bin/sh\n").unwrap();
    fs::create_dir_all(workspace.join(".orca-worktree-trash/wt-1700000000000-deadbeef")).unwrap();
    fs::write(
        workspace.join(".orca-worktree-trash/wt-1700000000000-deadbeef/file"),
        "trash",
    )
    .unwrap();
    write_json(
        &home.join(".claude/settings.json"),
        &json!({
            "theme": "dark",
            "hooks": {
                "Stop": [{"hooks": [{"type": "command", "command": "$HOME/.orca/agent-hooks/claude-hook.sh"}]}]
            }
        }),
    );

    let before = capture_tree(fixture.path());
    let scan = engine()
        .scan(&context(&home, vec![workspace]), &[])
        .unwrap();
    let after = capture_tree(fixture.path());

    assert_eq!(before, after);
    assert!(scan.findings.iter().any(|finding| {
        finding.kind == ArtifactKind::ManagedHook && finding.safety == Safety::Automatic
    }));
    assert!(scan.findings.iter().any(|finding| {
        finding.kind == ArtifactKind::ConfigMutation && finding.safety == Safety::Automatic
    }));
    assert!(scan.findings.iter().any(|finding| {
        finding.kind == ArtifactKind::WorktreeTrash && finding.reclaimable_bytes == 5
    }));
}

#[test]
fn apply_removes_only_orca_json_entries_and_owned_data() {
    let fixture = TempDir::new().unwrap();
    let home = fixture.path().join("home");
    fs::create_dir_all(home.join(".orca/agent-hooks")).unwrap();
    fs::write(home.join(".orca/agent-hooks/claude-hook.sh"), "hook").unwrap();
    let settings_path = home.join(".claude/settings.json");
    write_json(
        &settings_path,
        &json!({
            "theme": "dark",
            "hooks": {
                "Stop": [{
                    "matcher": "*",
                    "hooks": [
                        {"type": "command", "command": "user-stop-hook"},
                        {"type": "command", "command": "/tmp/my-claude-hook.sh"},
                        {"type": "command", "command": "$HOME/.orca/agent-hooks/claude-hook.sh"}
                    ]
                }],
                "PreToolUse": [{"hooks": [{"command": "user-pre-hook"}]}]
            },
            "statusLine": {
                "type": "command",
                "command": "sh $HOME/.orca/agent-hooks/claude-statusline.sh"
            }
        }),
    );

    let scan = engine().scan(&context(&home, Vec::new()), &[]).unwrap();
    let plan = Engine::plan(&scan, false);
    let result = Engine::apply(&plan);

    assert!(!result.has_failures());
    assert!(!home.join(".orca").exists());
    let settings: Value = serde_json::from_slice(&fs::read(settings_path).unwrap()).unwrap();
    assert_eq!(settings["theme"], "dark");
    assert_eq!(
        settings["hooks"]["Stop"][0]["hooks"][0]["command"],
        "user-stop-hook"
    );
    assert_eq!(
        settings["hooks"]["Stop"][0]["hooks"][1]["command"],
        "/tmp/my-claude-hook.sh"
    );
    assert_eq!(
        settings["hooks"]["PreToolUse"][0]["hooks"][0]["command"],
        "user-pre-hook"
    );
    assert!(settings.get("statusLine").is_none());
}

#[test]
fn apply_rejects_a_path_that_changed_after_scan() {
    let fixture = TempDir::new().unwrap();
    let home = fixture.path().join("home");
    fs::create_dir_all(home.join(".orca/Cache")).unwrap();
    fs::write(home.join(".orca/Cache/old"), "old").unwrap();

    let scan = engine().scan(&context(&home, Vec::new()), &[]).unwrap();
    let plan = Engine::plan(&scan, false);
    fs::write(home.join(".orca/Cache/new"), "new state").unwrap();
    let result = Engine::apply(&plan);

    assert!(home.join(".orca/Cache/new").exists());
    assert!(result.results.iter().any(|item| {
        item.path == home.join(".orca/Cache") && matches!(item.status, ApplyStatus::Failed)
    }));
}

#[test]
fn apply_rejects_third_party_config_changed_after_scan() {
    let fixture = TempDir::new().unwrap();
    let home = fixture.path().join("home");
    let settings_path = home.join(".gemini/settings.json");
    fs::create_dir_all(home.join(".orca/agent-hooks")).unwrap();
    let script_path = home.join(".orca/agent-hooks/gemini-hook.sh");
    fs::write(&script_path, "managed hook").unwrap();
    write_json(
        &settings_path,
        &json!({
            "hooks": {
                "AfterAgent": [{"hooks": [{"command": "$HOME/.orca/agent-hooks/gemini-hook.sh"}]}]
            }
        }),
    );

    let scan = engine().scan(&context(&home, Vec::new()), &[]).unwrap();
    let plan = Engine::plan(&scan, false);
    write_json(
        &settings_path,
        &json!({
            "newUserSetting": true,
            "hooks": {
                "AfterAgent": [{"hooks": [{"command": "$HOME/.orca/agent-hooks/gemini-hook.sh"}]}]
            }
        }),
    );
    let result = Engine::apply(&plan);

    let settings: Value = serde_json::from_slice(&fs::read(settings_path).unwrap()).unwrap();
    assert_eq!(settings["newUserSetting"], true);
    assert!(script_path.exists());
    assert!(result.results.iter().any(|item| {
        item.path == home.join(".gemini/settings.json")
            && matches!(item.status, ApplyStatus::Failed)
    }));
    assert!(result.results.iter().any(|item| {
        item.path == script_path
            && matches!(item.status, ApplyStatus::Skipped)
            && item.detail.contains("prerequisite")
    }));
}

#[cfg(unix)]
#[test]
fn apply_preserves_a_symlinked_third_party_config() {
    use std::os::unix::fs::symlink;

    let fixture = TempDir::new().unwrap();
    let home = fixture.path().join("home");
    let target = fixture.path().join("dotfiles/claude-settings.json");
    write_json(
        &target,
        &json!({
            "userSetting": "preserved",
            "hooks": {
                "Stop": [{"hooks": [{"command": "$HOME/.orca/agent-hooks/claude-hook.sh"}]}]
            }
        }),
    );
    let configured_path = home.join(".claude/settings.json");
    fs::create_dir_all(configured_path.parent().unwrap()).unwrap();
    symlink(&target, &configured_path).unwrap();

    let scan = engine().scan(&context(&home, Vec::new()), &[]).unwrap();
    let result = Engine::apply(&Engine::plan(&scan, false));

    assert!(!result.has_failures());
    assert!(
        fs::symlink_metadata(&configured_path)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    let settings: Value = serde_json::from_slice(&fs::read(target).unwrap()).unwrap();
    assert_eq!(settings["userSetting"], "preserved");
    assert!(settings["hooks"].as_object().unwrap().is_empty());
}

#[test]
fn malformed_third_party_config_is_reported_and_never_planned() {
    let fixture = TempDir::new().unwrap();
    let home = fixture.path().join("home");
    fs::create_dir_all(home.join(".cursor")).unwrap();
    let config = home.join(".cursor/hooks.json");
    fs::write(&config, b"{not-json").unwrap();
    fs::create_dir_all(home.join(".orca/agent-hooks")).unwrap();
    let script = home.join(".orca/agent-hooks/cursor-hook.sh");
    fs::write(&script, "managed hook").unwrap();

    let scan = engine().scan(&context(&home, Vec::new()), &[]).unwrap();
    let plan = Engine::plan(&scan, false);

    assert!(
        scan.warnings
            .iter()
            .any(|warning| warning.contains("Cursor"))
    );
    assert!(!plan.actions.iter().any(|action| action.path == config));
    assert!(!plan.actions.iter().any(|action| action.path == script));
    assert_eq!(fs::read(config).unwrap(), b"{not-json");
}

#[test]
fn only_exact_lowercase_worktree_trash_names_are_automatic() {
    let fixture = TempDir::new().unwrap();
    let home = fixture.path().join("home");
    let workspace = fixture.path().join("workspaces");
    let trash = workspace.join(".orca-worktree-trash");
    fs::create_dir_all(trash.join("wt-1700000000000-deadbeef")).unwrap();
    fs::create_dir_all(trash.join("wt-1700000000000-DEADBEEF")).unwrap();
    fs::create_dir_all(trash.join("notes")).unwrap();

    let scan = engine()
        .scan(&context(&home, vec![workspace]), &[])
        .unwrap();
    let automatic_paths = Engine::plan(&scan, false)
        .actions
        .into_iter()
        .map(|action| action.path)
        .collect::<Vec<_>>();

    assert!(automatic_paths.contains(&trash.join("wt-1700000000000-deadbeef")));
    assert!(!automatic_paths.contains(&trash.join("wt-1700000000000-DEADBEEF")));
    assert!(!automatic_paths.contains(&trash.join("notes")));
}

#[test]
fn orca_created_worktree_requires_review_opt_in() {
    let fixture = TempDir::new().unwrap();
    let home = fixture.path().join("home");
    let repository = fixture.path().join("repository");
    let worktree = fixture.path().join("orca-worktree");
    fs::create_dir_all(&repository).unwrap();
    git(&repository, &["init"]);
    git(&repository, &["config", "user.email", "test@example.com"]);
    git(&repository, &["config", "user.name", "Test"]);
    fs::write(repository.join("README.md"), "fixture").unwrap();
    git(&repository, &["add", "README.md"]);
    git(&repository, &["commit", "-m", "fixture"]);
    git(
        &repository,
        &[
            "worktree",
            "add",
            worktree.to_str().unwrap(),
            "-b",
            "orca/test",
        ],
    );
    write_json(
        &home.join(".orca/orca-data.json"),
        &json!({
            "repos": [{"id": "repo-1", "path": repository}],
            "worktreeMeta": {
                format!("repo-1::{}", worktree.display()): {
                    "orcaCreatedAt": 1_700_000_000_000_u64
                }
            }
        }),
    );

    let scan = engine().scan(&context(&home, Vec::new()), &[]).unwrap();
    let finding = scan
        .findings
        .iter()
        .find(|finding| finding.path == worktree && finding.kind == ArtifactKind::Worktree)
        .unwrap();

    assert_eq!(finding.safety, Safety::ReviewRequired);
    assert!(
        !Engine::plan(&scan, false)
            .actions
            .iter()
            .any(|action| action.path == worktree)
    );
    assert!(
        Engine::plan(&scan, true)
            .actions
            .iter()
            .any(|action| action.path == worktree)
    );

    fs::write(worktree.join("untracked.txt"), "do not delete").unwrap();
    let dirty_scan = engine().scan(&context(&home, Vec::new()), &[]).unwrap();
    let result = Engine::apply(&Engine::plan(&dirty_scan, true));
    assert!(worktree.join("untracked.txt").exists());
    assert!(result.results.iter().any(|item| {
        item.path == worktree
            && matches!(item.status, ApplyStatus::Failed)
            && item.detail.contains("untracked")
    }));
}

fn git(directory: &Path, arguments: &[&str]) {
    let output = Command::new("git")
        .arg("-C")
        .arg(directory)
        .args(arguments)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {:?} failed: {}",
        arguments,
        String::from_utf8_lossy(&output.stderr)
    );
}
