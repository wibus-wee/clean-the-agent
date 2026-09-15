use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use serde_json::Value;
use tempfile::TempDir;

fn run(home: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_clean-any"))
        .arg("--home")
        .arg(home)
        .args(args)
        .output()
        .unwrap()
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).unwrap()
}

#[test]
fn scan_and_tweak_share_the_human_document_layout() {
    let fixture = TempDir::new().unwrap();
    let home = fixture.path().join("home");

    let scan = run(&home, &["scan"]);
    assert!(scan.status.success(), "{}", stderr(&scan));
    assert_eq!(
        stdout(&scan),
        "Scan complete\nNothing to clean.\n\nScan only. Nothing was changed.\n"
    );

    let tweak = run(&home, &["tweak", "codex.disable-pet-shortcut"]);
    assert!(tweak.status.success(), "{}", stderr(&tweak));
    let tweak = stdout(&tweak);
    assert!(tweak.starts_with("Tweak: Disable the Pet keyboard shortcut\n"));
    assert!(tweak.contains("Proposed change (1)\n"));
    assert!(tweak.contains("~/.codex/keybindings.json"));
    assert!(!tweak.contains("NeedsChange"));
}

#[test]
fn verbose_is_the_only_human_mode_that_prints_detection_evidence() {
    let fixture = TempDir::new().unwrap();
    let home = fixture.path().join("home");
    fs::create_dir_all(home.join(".orca/Cache")).unwrap();
    fs::write(home.join(".orca/Cache/item"), b"cache").unwrap();

    let concise = run(&home, &["scan"]);
    assert!(concise.status.success(), "{}", stderr(&concise));
    let concise = stdout(&concise);
    assert!(concise.contains("Safe to clean"));
    assert!(!concise.contains("Why:"));
    assert!(!concise.contains("ProviderOwned"));

    let verbose = run(&home, &["scan", "--verbose"]);
    assert!(verbose.status.success(), "{}", stderr(&verbose));
    let verbose = stdout(&verbose);
    assert!(verbose.contains("Why:"));
    assert!(verbose.contains("provider-owned"));
}

#[test]
fn noninteractive_apply_requires_yes_and_does_not_change_files() {
    let fixture = TempDir::new().unwrap();
    let home = fixture.path().join("home");
    let cache = home.join(".orca/Cache/item");
    fs::create_dir_all(cache.parent().unwrap()).unwrap();
    fs::write(&cache, b"cache").unwrap();

    let output = run(&home, &["clean", "--apply"]);

    assert!(!output.status.success());
    assert!(stderr(&output).contains("interactive terminal or --yes"));
    assert!(cache.exists());
}

#[test]
fn confirmed_cleanup_and_tweak_use_the_same_result_sections() {
    let fixture = TempDir::new().unwrap();
    let cleanup_home = fixture.path().join("cleanup-home");
    let cache = cleanup_home.join(".orca/Cache/item");
    fs::create_dir_all(cache.parent().unwrap()).unwrap();
    fs::write(&cache, b"cache").unwrap();

    let cleanup = run(&cleanup_home, &["clean", "--apply", "--yes"]);
    assert!(cleanup.status.success(), "{}", stderr(&cleanup));
    let cleanup_output = stdout(&cleanup);
    assert!(cleanup_output.starts_with("Cleanup complete\n"));
    assert!(cleanup_output.contains("Applied ("));
    assert!(!cache.exists());

    let tweak_home = fixture.path().join("tweak-home");
    let tweak = run(
        &tweak_home,
        &["tweak", "codex.disable-pet-shortcut", "--apply", "--yes"],
    );
    assert!(tweak.status.success(), "{}", stderr(&tweak));
    let tweak_output = stdout(&tweak);
    assert!(tweak_output.starts_with("Tweak complete: Disable the Pet keyboard shortcut\n"));
    assert!(tweak_output.contains("Applied (1)"));
    assert!(tweak_output.contains("Restart Codex"));
}

#[test]
fn json_contracts_remain_command_specific() {
    let fixture = TempDir::new().unwrap();
    let home = fixture.path().join("home");

    let providers = run(&home, &["providers", "--json"]);
    assert!(providers.status.success(), "{}", stderr(&providers));
    let providers: Value = serde_json::from_slice(&providers.stdout).unwrap();
    assert_eq!(providers, serde_json::json!(["orca"]));

    let scan = run(&home, &["scan", "--json"]);
    assert!(scan.status.success(), "{}", stderr(&scan));
    let scan: Value = serde_json::from_slice(&scan.stdout).unwrap();
    assert!(scan.get("findings").is_some());
    assert!(scan.get("scan").is_none());

    let clean = run(&home, &["clean", "--json"]);
    assert!(clean.status.success(), "{}", stderr(&clean));
    let clean: Value = serde_json::from_slice(&clean.stdout).unwrap();
    assert_eq!(clean["applied"], false);
    assert!(clean.get("scan").is_some());
    assert!(clean.get("plan").is_some());

    let tweak = run(&home, &["tweak", "codex.disable-pet-shortcut", "--json"]);
    assert!(tweak.status.success(), "{}", stderr(&tweak));
    let tweak: Value = serde_json::from_slice(&tweak.stdout).unwrap();
    assert_eq!(tweak["applied"], false);
    assert_eq!(tweak["report"]["id"], "codex.disable-pet-shortcut");
}

#[test]
fn no_command_requires_a_terminal_for_the_guided_menu() {
    let fixture = TempDir::new().unwrap();
    let home = fixture.path().join("home");

    let output = run(&home, &[]);

    assert!(!output.status.success());
    assert!(stderr(&output).contains("interactive mode requires a terminal"));
    assert!(stderr(&output).contains("choose scan, clean, tweaks, or tweak"));
}

#[test]
fn interactive_help_exposes_only_human_output_options() {
    let fixture = TempDir::new().unwrap();
    let home = fixture.path().join("home");

    let output = run(&home, &["interactive", "--help"]);

    assert!(output.status.success(), "{}", stderr(&output));
    let help = stdout(&output);
    assert!(help.contains("--verbose"));
    assert!(!help.contains("--json"));
}
