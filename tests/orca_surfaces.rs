use std::fs;
use std::path::Path;

use clean_any::model::{ArtifactKind, Safety};
use clean_any::providers::OrcaProvider;
use clean_any::{Engine, HomeScope, Platform, ScanContext, ScopeKind};
use serde_json::{Value, json};
use tempfile::TempDir;

fn context(home: &Path) -> ScanContext {
    ScanContext {
        home: home.to_owned(),
        platform: Platform::Linux,
        orca_data_dirs: Vec::new(),
        workspace_roots: Vec::new(),
        honor_environment: false,
        additional_homes: Vec::new(),
        stale_after_days: 30,
    }
}

fn engine() -> Engine {
    Engine::new(vec![Box::new(OrcaProvider)])
}

fn write(path: &Path, content: impl AsRef<[u8]>) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn write_json(path: &Path, value: impl serde::Serialize) {
    write(path, serde_json::to_vec_pretty(&value).unwrap());
}

#[test]
fn owned_state_has_specific_policies_and_default_workspace_root() {
    let fixture = TempDir::new().unwrap();
    let home = fixture.path().join("home");
    write_json(&home.join(".orca/orca-data.json"), json!({"settings": {}}));
    write(&home.join(".orca/Cache/item"), "cache");
    write(&home.join(".orca/terminal-history/session"), "history");
    write(&home.join(".orca/logs/daemon.log"), "recent");
    write(
        &home.join("orca/workspaces/.orca-worktree-trash/wt-1700000000000-deadbeef/item"),
        "trash",
    );
    write_json(&home.join("project/.mcp.json"), json!({"mcpServers": {}}));

    let scan = engine().scan(&context(&home), &[]).unwrap();
    let automatic = Engine::plan(&scan, false);

    assert!(
        automatic
            .actions
            .iter()
            .any(|action| action.path == home.join(".orca/Cache"))
    );
    assert!(
        automatic
            .actions
            .iter()
            .any(|action| action.path.ends_with("wt-1700000000000-deadbeef"))
    );
    assert!(
        !automatic
            .actions
            .iter()
            .any(|action| action.path == home.join(".orca/orca-data.json"))
    );
    assert!(
        !automatic
            .actions
            .iter()
            .any(|action| action.path == home.join(".orca/terminal-history"))
    );
    assert!(
        !automatic
            .actions
            .iter()
            .any(|action| action.path == home.join(".orca/logs/daemon.log"))
    );
    assert!(
        !scan
            .findings
            .iter()
            .any(|finding| finding.path.ends_with(".mcp.json"))
    );
}

#[test]
fn every_json_hook_integration_is_structurally_cleaned() {
    let fixture = TempDir::new().unwrap();
    let home = fixture.path().join("home");
    let specs = [
        (".claude/settings.json", "claude-hook.sh"),
        (".openclaude/settings.json", "openclaude-hook.sh"),
        (".codex/hooks.json", "codex-hook.sh"),
        (".cursor/hooks.json", "cursor-hook.sh"),
        (".gemini/settings.json", "gemini-hook.sh"),
        (".factory/settings.json", "droid-hook.sh"),
        (".commandcode/settings.json", "command-code-hook.sh"),
    ];
    for (relative, script) in specs {
        write_json(
            &home.join(relative),
            json!({
                "userSetting": true,
                "hooks": {"Stop": [{"hooks": [
                    {"command": "user-hook"},
                    {"command": format!("~/.orca/agent-hooks/{script}")}
                ]}]}
            }),
        );
    }
    write_json(
        &home.join(".gemini/config/hooks.json"),
        json!({
            "userBundle": {"Stop": [{"command": "user-hook"}]},
            "orca-status": {"Stop": [{"command": "~/.orca/agent-hooks/antigravity-stop.cmd"}]}
        }),
    );
    for (relative, script) in [
        (".copilot/hooks/orca.json", "copilot-hook.sh"),
        (".grok/hooks/orca-status.json", "grok-hook.sh"),
    ] {
        write_json(
            &home.join(relative),
            json!({"hooks": {"Stop": [{"bash": format!("~/.orca/agent-hooks/{script}")}]}}),
        );
    }
    write(
        &home.join(".config/devin/config.json"),
        r#"{
  // keep this user comment
  "userSetting": true,
  "hooks": { "Stop": [{ "hooks": [
    { "command": "user-hook" },
    { "command": "~/.orca/agent-hooks/devin-hook.sh" }
  ] }] }
}"#,
    );

    let scan = engine().scan(&context(&home), &[]).unwrap();
    assert!(
        scan.findings
            .iter()
            .filter(|finding| finding.kind == ArtifactKind::ConfigMutation)
            .count()
            >= 11
    );
    let result = Engine::apply(&Engine::plan(&scan, false));
    assert!(!result.has_failures(), "{:#?}", result.results);

    for (relative, _) in specs {
        let value: Value = serde_json::from_slice(&fs::read(home.join(relative)).unwrap()).unwrap();
        assert_eq!(value["userSetting"], true);
        assert_eq!(
            value["hooks"]["Stop"][0]["hooks"][0]["command"],
            "user-hook"
        );
        assert_eq!(
            value["hooks"]["Stop"][0]["hooks"].as_array().unwrap().len(),
            1
        );
    }
    assert!(!home.join(".copilot/hooks/orca.json").exists());
    assert!(!home.join(".grok/hooks/orca-status.json").exists());
    let devin = fs::read_to_string(home.join(".config/devin/config.json")).unwrap();
    assert!(devin.contains("// keep this user comment"));
    assert!(devin.contains("user-hook"));
    assert!(!devin.contains("devin-hook.sh"));
}

#[test]
fn plugin_toml_yaml_and_backup_surfaces_are_handled_separately() {
    let fixture = TempDir::new().unwrap();
    let home = fixture.path().join("home");
    let marker = "Managed by Orca. Do not edit; changes may be overwritten.";
    write(
        &home.join(".config/amp/plugins/orca-agent-status.ts"),
        format!("// {marker}\n"),
    );
    write(
        &home.join(".hermes/plugins/orca-status/plugin.yaml"),
        format!("# {marker}\n"),
    );
    write(
        &home.join(".hermes/plugins/orca-status/__init__.py"),
        format!("# {marker}\n"),
    );
    write(
        &home.join(".hermes/config.yaml"),
        "theme: dark\nplugins:\n  enabled:\n    - user-plugin\n    - orca-status\n  disabled:\n    - other\n",
    );
    write(&home.join(".hermes/config.yaml.bak"), "backup");
    write(
        &home.join(".kimi-code/config.toml"),
        "theme = \"dark\"\n\n# >>> orca-managed-kimi-hooks (managed by Orca; do not edit) >>>\n[[hooks]]\nname = \"orca\"\n# <<< orca-managed-kimi-hooks <<<\n",
    );
    write(&home.join(".kimi-code/config.toml.bak"), "backup");

    let scan = engine().scan(&context(&home), &[]).unwrap();
    let result = Engine::apply(&Engine::plan(&scan, false));
    assert!(!result.has_failures(), "{:#?}", result.results);
    assert!(
        !home
            .join(".config/amp/plugins/orca-agent-status.ts")
            .exists()
    );
    assert!(!home.join(".hermes/plugins/orca-status").exists());
    let hermes = fs::read_to_string(home.join(".hermes/config.yaml")).unwrap();
    assert!(hermes.contains("user-plugin"));
    assert!(!hermes.contains("orca-status"));
    let kimi = fs::read_to_string(home.join(".kimi-code/config.toml")).unwrap();
    assert!(kimi.contains("theme = \"dark\""));
    assert!(!kimi.contains("orca-managed-kimi-hooks"));
    assert!(home.join(".hermes/config.yaml.bak").exists());
    assert!(home.join(".kimi-code/config.toml.bak").exists());
}

#[test]
fn orphaned_workspace_trust_and_state_are_removed_without_touching_user_entries() {
    let fixture = TempDir::new().unwrap();
    let home = fixture.path().join("home");
    let missing = home.join("orca/workspaces/gone");
    write_json(
        &home.join(".orca/orca-data.json"),
        json!({
            "userSetting": true,
            "worktreeMeta": {format!("repo-1::{}", missing.display()): {"orcaCreatedAt": 1}}
        }),
    );
    write_json(
        &home.join(".cursor/projects/gone/.workspace-trusted"),
        json!({"trustedAt": "now", "workspacePath": missing}),
    );
    write_json(
        &home.join(".copilot/config.json"),
        json!({"userSetting": true, "trustedFolders": [missing, "/user/project"]}),
    );
    write(
        &home.join(".codex/config.toml"),
        format!(
            "theme = \"dark\"\n\n[projects.\"{}\"]\ntrust_level = \"trusted\"\n\n[projects.\"/user/project\"]\ntrust_level = \"trusted\"\n\n[hooks.state.\"orca\"]\ncommand = \"~/.orca/agent-hooks/codex-hook.sh\"\n",
            missing.display()
        ),
    );

    let scan = engine().scan(&context(&home), &[]).unwrap();
    let result = Engine::apply(&Engine::plan(&scan, false));
    assert!(!result.has_failures(), "{:#?}", result.results);
    let state: Value =
        serde_json::from_slice(&fs::read(home.join(".orca/orca-data.json")).unwrap()).unwrap();
    assert_eq!(state["userSetting"], true);
    assert!(state["worktreeMeta"].as_object().unwrap().is_empty());
    assert!(
        !home
            .join(".cursor/projects/gone/.workspace-trusted")
            .exists()
    );
    let copilot: Value =
        serde_json::from_slice(&fs::read(home.join(".copilot/config.json")).unwrap()).unwrap();
    assert_eq!(copilot["trustedFolders"], json!(["/user/project"]));
    assert_eq!(copilot["userSetting"], true);
    let codex = fs::read_to_string(home.join(".codex/config.toml")).unwrap();
    assert!(codex.contains("theme = \"dark\""));
    assert!(codex.contains("/user/project"));
    assert!(!codex.contains(&missing.display().to_string()));
    assert!(!codex.contains("codex-hook.sh"));
}

#[cfg(unix)]
#[test]
fn runtime_skill_and_remote_scopes_require_verifiable_ownership() {
    use std::os::unix::fs::symlink;

    let fixture = TempDir::new().unwrap();
    let home = fixture.path().join("home");
    let remote = fixture.path().join("remote");
    let canonical = home.join(".agents/skills/demo");
    write(&canonical.join("SKILL.md"), "skill");
    fs::create_dir_all(home.join(".claude/skills")).unwrap();
    symlink(&canonical, home.join(".claude/skills/demo")).unwrap();
    write(&home.join(".codex/prompts/prompt.md"), "prompt");
    fs::create_dir_all(home.join(".orca/codex-runtime-home/home")).unwrap();
    symlink(
        home.join(".codex/prompts"),
        home.join(".orca/codex-runtime-home/home/prompts"),
    )
    .unwrap();
    write(
        &home.join(".orca/codex-runtime-home/home/themes/custom"),
        "diverged",
    );
    write(
        &remote.join("orca/workspaces/.orca-worktree-trash/wt-1700000000000-deadbeef/item"),
        "trash",
    );
    write(&remote.join(".orca-remote/relay-v1/bin"), "runtime");

    let mut context = context(&home);
    context.stale_after_days = 0;
    context.additional_homes.push(HomeScope {
        home: remote.clone(),
        kind: ScopeKind::SshRemote,
    });
    let scan = engine().scan(&context, &[]).unwrap();
    let automatic = Engine::plan(&scan, false);
    assert!(
        automatic
            .actions
            .iter()
            .any(|action| action.path == home.join(".claude/skills/demo"))
    );
    assert!(
        automatic
            .actions
            .iter()
            .any(|action| action.path == home.join(".orca/codex-runtime-home/home/prompts"))
    );
    assert!(
        !automatic
            .actions
            .iter()
            .any(|action| action.path == home.join(".orca/codex-runtime-home/home/themes"))
    );
    assert!(scan.findings.iter().any(|finding| {
        finding.scope == ScopeKind::SshRemote && finding.path.starts_with(&remote)
    }));
    assert!(automatic.actions.iter().any(|action| {
        action.scope == ScopeKind::SshRemote && action.path == remote.join(".orca-remote/relay-v1")
    }));
}

#[test]
fn live_attributed_workspaces_keep_trust_review_gated() {
    let fixture = TempDir::new().unwrap();
    let home = fixture.path().join("home");
    let workspace = home.join("orca/workspaces/live");
    fs::create_dir_all(&workspace).unwrap();
    write_json(
        &home.join(".copilot/config.json"),
        json!({"trustedFolders": [workspace]}),
    );
    let scan = engine().scan(&context(&home), &[]).unwrap();
    let finding = scan
        .findings
        .iter()
        .find(|finding| finding.path == home.join(".copilot/config.json"))
        .unwrap();
    assert_eq!(finding.safety, Safety::ReviewRequired);
    assert!(
        !Engine::plan(&scan, false)
            .actions
            .iter()
            .any(|action| action.path == finding.path)
    );
}

#[test]
fn identity_metadata_is_resolved_through_locator_aliases() {
    let fixture = TempDir::new().unwrap();
    let home = fixture.path().join("home");
    let missing = home.join("orca/workspaces/missing");
    write_json(
        &home.join(".orca/orca-data.json"),
        json!({
            "worktreeMeta": {},
            "worktreeMetaByIdentity": {
                "wt2:local:owned": {"orcaCreatedAt": 1},
                "wt2:local:user": {"lastActivityAt": 2}
            },
            "worktreeIdentityAliases": {
                format!("local|repo-1::{}", missing.display()): ["wt2:local:owned"],
                "local|repo-2::/user/missing": ["wt2:local:user"]
            }
        }),
    );

    let scan = engine().scan(&context(&home), &[]).unwrap();
    let result = Engine::apply(&Engine::plan(&scan, false));
    assert!(!result.has_failures(), "{:#?}", result.results);
    let state: Value =
        serde_json::from_slice(&fs::read(home.join(".orca/orca-data.json")).unwrap()).unwrap();
    assert!(
        state["worktreeMetaByIdentity"]
            .get("wt2:local:owned")
            .is_none()
    );
    assert!(
        state["worktreeMetaByIdentity"]
            .get("wt2:local:user")
            .is_some()
    );
    assert_eq!(
        state["worktreeIdentityAliases"]["local|repo-2::/user/missing"],
        json!(["wt2:local:user"])
    );
}
