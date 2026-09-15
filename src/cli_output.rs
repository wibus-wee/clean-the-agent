use std::collections::HashSet;
use std::io::{self, Write};
use std::path::Path;

use clean_any::model::{
    ActionResult, ApplyReport, ApplyStatus, ArtifactKind, CleanupAction, CleanupPlan, Finding,
    Ownership, Safety, ScanReport, ScopeKind,
};
use clean_any::{TweakReport, TweakStatus, TweakSummary};
use serde::Serialize;

#[derive(Clone, Copy)]
pub(crate) struct OutputOptions {
    pub(crate) json: bool,
    pub(crate) verbose: bool,
}

#[derive(Clone, Copy)]
pub(crate) enum CommandOutput<'a> {
    Providers {
        providers: &'a [&'static str],
    },
    Scan {
        report: &'a ScanReport,
        home: &'a Path,
    },
    CleanPreview {
        scan: &'a ScanReport,
        plan: &'a CleanupPlan,
        home: &'a Path,
        awaiting_confirmation: bool,
    },
    CleanApplied {
        scan: &'a ScanReport,
        plan: &'a CleanupPlan,
        result: &'a ApplyReport,
        home: &'a Path,
    },
    Tweaks {
        summaries: &'a [TweakSummary],
    },
    TweakPreview {
        report: &'a TweakReport,
        home: &'a Path,
        awaiting_confirmation: bool,
    },
    TweakApplied {
        report: &'a TweakReport,
        result: &'a ApplyReport,
        home: &'a Path,
    },
    Cancelled {
        operation: &'a str,
    },
}

#[derive(Serialize)]
struct CleanJson<'a> {
    scan: &'a ScanReport,
    plan: &'a CleanupPlan,
    applied: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<&'a ApplyReport>,
}

#[derive(Serialize)]
struct TweakJson<'a> {
    report: &'a TweakReport,
    applied: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<&'a ApplyReport>,
}

struct HumanDocument {
    title: String,
    summary: String,
    sections: Vec<HumanSection>,
    warnings: Vec<String>,
    notes: Vec<String>,
}

struct HumanSection {
    title: String,
    items: Vec<HumanItem>,
}

struct HumanItem {
    subject: String,
    description: String,
    metadata: Option<String>,
    detail: Option<String>,
}

pub(crate) fn emit(output: CommandOutput<'_>, options: OutputOptions) -> io::Result<()> {
    let stdout = io::stdout();
    let mut writer = stdout.lock();
    if options.json {
        write_json(&mut writer, output)
    } else {
        write_human(&mut writer, output, options.verbose)
    }
}

fn write_json(writer: &mut impl Write, output: CommandOutput<'_>) -> io::Result<()> {
    match output {
        CommandOutput::Providers { providers } => write_json_value(writer, providers),
        CommandOutput::Scan { report, .. } => write_json_value(writer, report),
        CommandOutput::CleanPreview { scan, plan, .. } => write_json_value(
            writer,
            &CleanJson {
                scan,
                plan,
                applied: false,
                result: None,
            },
        ),
        CommandOutput::CleanApplied {
            scan, plan, result, ..
        } => write_json_value(
            writer,
            &CleanJson {
                scan,
                plan,
                applied: true,
                result: Some(result),
            },
        ),
        CommandOutput::Tweaks { summaries } => write_json_value(writer, summaries),
        CommandOutput::TweakPreview { report, .. } => write_json_value(
            writer,
            &TweakJson {
                report,
                applied: false,
                result: None,
            },
        ),
        CommandOutput::TweakApplied { report, result, .. } => write_json_value(
            writer,
            &TweakJson {
                report,
                applied: true,
                result: Some(result),
            },
        ),
        CommandOutput::Cancelled { operation } => write_json_value(
            writer,
            &serde_json::json!({"cancelled": true, "operation": operation}),
        ),
    }
}

fn write_json_value<T: Serialize + ?Sized>(writer: &mut impl Write, value: &T) -> io::Result<()> {
    serde_json::to_writer_pretty(&mut *writer, value).map_err(io::Error::other)?;
    writeln!(writer)
}

fn write_human(
    writer: &mut impl Write,
    output: CommandOutput<'_>,
    verbose: bool,
) -> io::Result<()> {
    let document = match output {
        CommandOutput::Providers { providers } => providers_document(providers),
        CommandOutput::Scan { report, home } => scan_document(report, home, verbose),
        CommandOutput::CleanPreview {
            scan,
            plan,
            home,
            awaiting_confirmation,
        } => clean_preview_document(scan, plan, home, verbose, awaiting_confirmation),
        CommandOutput::CleanApplied {
            scan,
            plan,
            result,
            home,
        } => applied_document("Cleanup complete", scan, plan, result, home, verbose),
        CommandOutput::Tweaks { summaries } => tweaks_document(summaries),
        CommandOutput::TweakPreview {
            report,
            home,
            awaiting_confirmation,
        } => tweak_preview_document(report, home, verbose, awaiting_confirmation),
        CommandOutput::TweakApplied {
            report,
            result,
            home,
        } => tweak_applied_document(report, result, home, verbose),
        CommandOutput::Cancelled { operation } => HumanDocument {
            title: "Cancelled".to_owned(),
            summary: format!("{operation} was cancelled. Nothing was changed."),
            sections: Vec::new(),
            warnings: Vec::new(),
            notes: Vec::new(),
        },
    };
    render_document(writer, &document)
}

fn providers_document(providers: &[&str]) -> HumanDocument {
    HumanDocument {
        title: "Available providers".to_owned(),
        summary: format!(
            "{} cleanup {} available.",
            providers.len(),
            noun(providers.len(), "provider", "providers")
        ),
        sections: vec![HumanSection {
            title: "Providers".to_owned(),
            items: providers
                .iter()
                .map(|provider| HumanItem {
                    subject: (*provider).to_owned(),
                    description: String::new(),
                    metadata: None,
                    detail: None,
                })
                .collect(),
        }],
        warnings: Vec::new(),
        notes: vec!["Select one with `--provider <id>`.".to_owned()],
    }
}

fn render_document(writer: &mut impl Write, document: &HumanDocument) -> io::Result<()> {
    writeln!(writer, "{}", document.title)?;
    writeln!(writer, "{}", document.summary)?;

    for section in &document.sections {
        if section.items.is_empty() {
            continue;
        }
        writeln!(writer)?;
        writeln!(writer, "{} ({})", section.title, section.items.len())?;
        for item in &section.items {
            writeln!(writer, "  {}", item.subject)?;
            if !item.description.is_empty() {
                writeln!(writer, "    {}", item.description)?;
            }
            if let Some(metadata) = &item.metadata {
                writeln!(writer, "    {metadata}")?;
            }
            if let Some(detail) = &item.detail {
                writeln!(writer, "    {detail}")?;
            }
        }
    }

    if !document.warnings.is_empty() {
        writeln!(writer)?;
        writeln!(writer, "Warnings ({})", document.warnings.len())?;
        for warning in &document.warnings {
            writeln!(writer, "  - {warning}")?;
        }
    }

    if !document.notes.is_empty() {
        writeln!(writer)?;
        for note in &document.notes {
            writeln!(writer, "{note}")?;
        }
    }
    Ok(())
}

fn scan_document(report: &ScanReport, home: &Path, verbose: bool) -> HumanDocument {
    let automatic = report
        .findings
        .iter()
        .filter(|finding| finding.safety == Safety::Automatic && finding.action.is_some())
        .collect::<Vec<_>>();
    let review = report
        .findings
        .iter()
        .filter(|finding| finding.safety == Safety::ReviewRequired)
        .collect::<Vec<_>>();
    let informational = report
        .findings
        .iter()
        .filter(|finding| {
            finding.safety == Safety::Informational
                || (finding.safety == Safety::Automatic && finding.action.is_none())
        })
        .collect::<Vec<_>>();
    let summary = if report.findings.is_empty() {
        "Nothing to clean.".to_owned()
    } else {
        format!(
            "{} safe to clean, {} {} review, {} kept for information.",
            automatic.len(),
            review.len(),
            if review.len() == 1 { "needs" } else { "need" },
            informational.len()
        )
    };
    let mut notes = vec!["Scan only. Nothing was changed.".to_owned()];
    if !automatic.is_empty() {
        notes.push("Next: run `clean-the-agent clean` to preview the cleanup plan.".to_owned());
    }
    if !review.is_empty() {
        notes.push(
            "Review-gated items are excluded unless `--include-review` is supplied.".to_owned(),
        );
    }
    HumanDocument {
        title: "Scan complete".to_owned(),
        summary,
        sections: vec![
            finding_section("Safe to clean", &automatic, home, verbose),
            finding_section("Needs review", &review, home, verbose),
            finding_section("Kept for information", &informational, home, verbose),
        ],
        warnings: report.warnings.clone(),
        notes,
    }
}

fn clean_preview_document(
    scan: &ScanReport,
    plan: &CleanupPlan,
    home: &Path,
    verbose: bool,
    awaiting_confirmation: bool,
) -> HumanDocument {
    let selected_ids = plan
        .actions
        .iter()
        .map(|action| action.id.as_str())
        .collect::<HashSet<_>>();
    let excluded = scan
        .findings
        .iter()
        .filter(|finding| {
            finding.safety == Safety::ReviewRequired
                && finding
                    .action
                    .as_ref()
                    .is_some_and(|action| !selected_ids.contains(action.id.as_str()))
        })
        .collect::<Vec<_>>();
    let informational = scan
        .findings
        .iter()
        .filter(|finding| finding.action.is_none())
        .collect::<Vec<_>>();
    let summary = if plan.actions.is_empty() {
        if excluded.is_empty() {
            "Nothing to clean.".to_owned()
        } else {
            format!(
                "Nothing selected; {} {} review.",
                excluded.len(),
                if excluded.len() == 1 { "needs" } else { "need" }
            )
        }
    } else {
        format!(
            "{} {} selected, reclaiming up to {}.",
            plan.actions.len(),
            noun(plan.actions.len(), "change", "changes"),
            human_bytes(plan.reclaimable_bytes)
        )
    };
    let mut sections = vec![action_section(
        "Will clean",
        &plan.actions,
        scan,
        home,
        verbose,
    )];
    sections.push(finding_section(
        "Not selected — needs review",
        &excluded,
        home,
        verbose,
    ));
    if verbose {
        sections.push(finding_section(
            "Not actionable",
            &informational,
            home,
            true,
        ));
    }
    let mut notes = Vec::new();
    if plan.actions.is_empty() {
        notes.push("Nothing was changed.".to_owned());
    } else if awaiting_confirmation {
        notes.push("Review the plan above before confirming.".to_owned());
    } else {
        notes.push("Dry run. Nothing was changed.".to_owned());
        if !plan.actions.is_empty() {
            notes.push("Next: run `clean-the-agent clean --apply` to continue.".to_owned());
        }
    }
    if !excluded.is_empty() {
        notes.push("Add `--include-review` only after inspecting the excluded items.".to_owned());
    }
    HumanDocument {
        title: "Cleanup preview".to_owned(),
        summary,
        sections,
        warnings: scan.warnings.clone(),
        notes,
    }
}

fn applied_document(
    title: &str,
    scan: &ScanReport,
    plan: &CleanupPlan,
    result: &ApplyReport,
    home: &Path,
    verbose: bool,
) -> HumanDocument {
    let applied = results_with_status(result, ApplyStatus::Applied);
    let skipped = results_with_status(result, ApplyStatus::Skipped);
    let failed = results_with_status(result, ApplyStatus::Failed);
    HumanDocument {
        title: title.to_owned(),
        summary: format!(
            "{} applied, {} skipped, {} failed.",
            applied.len(),
            skipped.len(),
            failed.len()
        ),
        sections: vec![
            result_section("Applied", &applied, plan, home, verbose),
            result_section("Skipped", &skipped, plan, home, verbose),
            result_section("Failed", &failed, plan, home, verbose),
        ],
        warnings: scan.warnings.clone(),
        notes: if failed.is_empty() {
            Vec::new()
        } else {
            vec!["Nothing else was retried. Re-scan before trying again.".to_owned()]
        },
    }
}

fn tweaks_document(summaries: &[TweakSummary]) -> HumanDocument {
    let items = summaries
        .iter()
        .map(|summary| HumanItem {
            subject: summary.id.to_owned(),
            description: format!("{} — {}", summary.product, summary.title),
            metadata: None,
            detail: Some(summary.description.to_owned()),
        })
        .collect::<Vec<_>>();
    HumanDocument {
        title: "Available tweaks".to_owned(),
        summary: format!(
            "{} opt-in preference {} available.",
            summaries.len(),
            noun(summaries.len(), "tweak", "tweaks")
        ),
        sections: vec![HumanSection {
            title: "Tweaks".to_owned(),
            items,
        }],
        warnings: Vec::new(),
        notes: vec!["Inspect one with `clean-the-agent tweak <id>`.".to_owned()],
    }
}

fn tweak_preview_document(
    report: &TweakReport,
    home: &Path,
    verbose: bool,
    awaiting_confirmation: bool,
) -> HumanDocument {
    let (summary, section_title) = match report.status {
        TweakStatus::NeedsChange => ("This tweak can be applied.", "Proposed change"),
        TweakStatus::Satisfied => ("This preference is already configured.", "Current setting"),
        TweakStatus::Blocked => ("This tweak cannot be applied safely.", "Blocked"),
    };
    let metadata = verbose.then(|| format!("{} · {}", report.product, report.id));
    let mut notes = Vec::new();
    if report.status == TweakStatus::NeedsChange {
        if awaiting_confirmation {
            notes.push("Review the change above before confirming.".to_owned());
        } else {
            notes.push("Dry run. Nothing was changed.".to_owned());
            notes.push(format!(
                "Next: run `clean-the-agent tweak {} --apply` to continue.",
                report.id
            ));
        }
        if report.restart_required {
            notes.push(format!(
                "Restart {} after applying this tweak.",
                report.product
            ));
        }
    }
    HumanDocument {
        title: format!("Tweak: {}", report.title),
        summary: summary.to_owned(),
        sections: vec![HumanSection {
            title: section_title.to_owned(),
            items: vec![HumanItem {
                subject: display_path(&report.path, home),
                description: report.description.clone(),
                metadata,
                detail: Some(report.detail.clone()),
            }],
        }],
        warnings: Vec::new(),
        notes,
    }
}

fn tweak_applied_document(
    report: &TweakReport,
    result: &ApplyReport,
    home: &Path,
    verbose: bool,
) -> HumanDocument {
    let empty_scan = ScanReport::default();
    let plan = CleanupPlan {
        actions: report.action.iter().cloned().collect(),
        excluded_review_findings: 0,
        reclaimable_bytes: 0,
    };
    let mut document = applied_document(
        &format!("Tweak complete: {}", report.title),
        &empty_scan,
        &plan,
        result,
        home,
        verbose,
    );
    if !result.has_failures() && report.restart_required {
        document.notes.push(format!(
            "Restart {} for the change to take effect.",
            report.product
        ));
    }
    document
}

fn finding_section(title: &str, findings: &[&Finding], home: &Path, verbose: bool) -> HumanSection {
    HumanSection {
        title: title.to_owned(),
        items: findings
            .iter()
            .map(|finding| finding_item(finding, home, verbose))
            .collect(),
    }
}

fn finding_item(finding: &Finding, home: &Path, verbose: bool) -> HumanItem {
    let subject = path_with_size(&finding.path, home, finding.reclaimable_bytes);
    HumanItem {
        subject,
        description: finding.description.clone(),
        metadata: verbose.then(|| {
            format!(
                "{} · {} · {} · {}",
                finding.provider,
                artifact_label(finding.kind),
                ownership_label(finding.ownership),
                scope_label(finding.scope)
            )
        }),
        detail: verbose.then(|| format!("Why: {}", finding.evidence)),
    }
}

fn action_section(
    title: &str,
    actions: &[CleanupAction],
    scan: &ScanReport,
    home: &Path,
    verbose: bool,
) -> HumanSection {
    HumanSection {
        title: title.to_owned(),
        items: actions
            .iter()
            .map(|action| {
                let finding = scan.findings.iter().find(|finding| {
                    finding
                        .action
                        .as_ref()
                        .is_some_and(|candidate| candidate.id == action.id)
                });
                HumanItem {
                    subject: path_with_size(&action.path, home, action.reclaimable_bytes),
                    description: action.description.clone(),
                    metadata: verbose.then(|| {
                        format!(
                            "{} · {} · {}",
                            action.provider,
                            safety_label(action.safety),
                            scope_label(action.scope)
                        )
                    }),
                    detail: verbose
                        .then(|| finding.map(|finding| format!("Why: {}", finding.evidence)))
                        .flatten(),
                }
            })
            .collect(),
    }
}

fn result_section(
    title: &str,
    results: &[&ActionResult],
    plan: &CleanupPlan,
    home: &Path,
    verbose: bool,
) -> HumanSection {
    HumanSection {
        title: title.to_owned(),
        items: results
            .iter()
            .map(|result| {
                let action = plan
                    .actions
                    .iter()
                    .find(|action| action.id == result.action_id);
                HumanItem {
                    subject: display_path(&result.path, home),
                    description: result.detail.clone(),
                    metadata: verbose.then(|| {
                        action.map_or_else(
                            || result.action_id.clone(),
                            |action| {
                                format!(
                                    "{} · {} · {}",
                                    action.provider,
                                    safety_label(action.safety),
                                    scope_label(action.scope)
                                )
                            },
                        )
                    }),
                    detail: None,
                }
            })
            .collect(),
    }
}

fn results_with_status(report: &ApplyReport, status: ApplyStatus) -> Vec<&ActionResult> {
    report
        .results
        .iter()
        .filter(|result| result.status == status)
        .collect()
}

fn path_with_size(path: &Path, home: &Path, bytes: u64) -> String {
    let path = display_path(path, home);
    if bytes == 0 {
        path
    } else {
        format!("{path}  {}", human_bytes(bytes))
    }
}

fn display_path(path: &Path, home: &Path) -> String {
    if path == home {
        return "~".to_owned();
    }
    path.strip_prefix(home).map_or_else(
        |_| path.display().to_string(),
        |relative| format!("~/{}", relative.display()),
    )
}

fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut divisor = 1_u64;
    let mut unit = 0;
    while bytes / divisor >= 1024 && unit < UNITS.len() - 1 {
        divisor *= 1024;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} {}", UNITS[unit])
    } else {
        let whole = bytes / divisor;
        let tenths = (bytes % divisor) * 10 / divisor;
        format!("{whole}.{tenths} {}", UNITS[unit])
    }
}

fn noun<'a>(count: usize, singular: &'a str, plural: &'a str) -> &'a str {
    if count == 1 { singular } else { plural }
}

fn artifact_label(kind: ArtifactKind) -> &'static str {
    match kind {
        ArtifactKind::OwnedData => "owned data",
        ArtifactKind::Cache => "cache",
        ArtifactKind::Log => "log",
        ArtifactKind::TemporaryState => "temporary state",
        ArtifactKind::RuntimeState => "runtime state",
        ArtifactKind::Worktree => "worktree",
        ArtifactKind::WorktreeTrash => "worktree trash",
        ArtifactKind::ManagedHook => "managed hook",
        ArtifactKind::ConfigMutation => "configuration change",
        ArtifactKind::OrphanedState => "orphaned state",
        ArtifactKind::UserHistory => "user history",
        ArtifactKind::TrustEntry => "trust entry",
        ArtifactKind::Plugin => "plugin",
        ArtifactKind::SkillPlacement => "skill placement",
        ArtifactKind::Backup => "backup",
        ArtifactKind::RemoteRuntime => "remote runtime",
    }
}

fn ownership_label(ownership: Ownership) -> &'static str {
    match ownership {
        Ownership::ProviderOwned => "provider-owned",
        Ownership::InjectedByProvider => "provider-injected",
        Ownership::Attributed => "attributed",
    }
}

fn safety_label(safety: Safety) -> &'static str {
    match safety {
        Safety::Automatic => "safe to clean",
        Safety::ReviewRequired => "needs review",
        Safety::Informational => "informational",
    }
}

fn scope_label(scope: ScopeKind) -> &'static str {
    match scope {
        ScopeKind::Local => "local",
        ScopeKind::Wsl => "WSL",
        ScopeKind::SshRemote => "SSH remote",
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use clean_any::model::{CleanupActionKind, PathSnapshot};

    use super::*;

    fn action(id: &str, path: &Path, safety: Safety) -> CleanupAction {
        CleanupAction {
            id: id.to_owned(),
            provider: "orca".to_owned(),
            description: "Remove generated data".to_owned(),
            path: path.to_owned(),
            scope: ScopeKind::Local,
            safety,
            reclaimable_bytes: 2048,
            kind: CleanupActionKind::RemovePath {
                expected: PathSnapshot {
                    digest: "digest".to_owned(),
                    entries: 1,
                    bytes: 2048,
                },
            },
            depends_on: Vec::new(),
        }
    }

    fn finding(id: &str, path: &Path, safety: Safety) -> Finding {
        let action = action(id, path, safety);
        Finding {
            id: id.to_owned(),
            provider: "orca".to_owned(),
            kind: ArtifactKind::Cache,
            ownership: Ownership::ProviderOwned,
            safety,
            path: path.to_owned(),
            scope: ScopeKind::Local,
            description: "Generated cache".to_owned(),
            evidence: "an internal ownership marker matched".to_owned(),
            reclaimable_bytes: 2048,
            action: Some(action),
        }
    }

    fn render(output: CommandOutput<'_>, verbose: bool) -> String {
        let mut bytes = Vec::new();
        write_human(&mut bytes, output, verbose).unwrap();
        String::from_utf8(bytes).unwrap()
    }

    #[test]
    fn scan_groups_by_decision_and_hides_internal_evidence_by_default() {
        let home = PathBuf::from("/home/test");
        let report = ScanReport {
            findings: vec![
                finding("safe", &home.join(".orca/Cache"), Safety::Automatic),
                finding(
                    "review",
                    &home.join("orca/workspaces/feature"),
                    Safety::ReviewRequired,
                ),
            ],
            warnings: Vec::new(),
        };

        let concise = render(
            CommandOutput::Scan {
                report: &report,
                home: &home,
            },
            false,
        );
        assert!(concise.contains("Safe to clean (1)"));
        assert!(concise.contains("Needs review (1)"));
        assert!(concise.contains("~/.orca/Cache  2.0 KiB"));
        assert!(!concise.contains("Automatic"));
        assert!(!concise.contains("internal ownership marker"));

        let verbose = render(
            CommandOutput::Scan {
                report: &report,
                home: &home,
            },
            true,
        );
        assert!(verbose.contains("cache · provider-owned · local"));
        assert!(verbose.contains("Why: an internal ownership marker matched"));
    }

    #[test]
    fn clean_preview_includes_excluded_review_items_and_next_step() {
        let home = PathBuf::from("/home/test");
        let safe = finding("safe", &home.join(".orca/Cache"), Safety::Automatic);
        let review = finding(
            "review",
            &home.join("orca/workspaces/feature"),
            Safety::ReviewRequired,
        );
        let scan = ScanReport {
            findings: vec![safe.clone(), review],
            warnings: Vec::new(),
        };
        let plan = CleanupPlan {
            actions: vec![safe.action.unwrap()],
            excluded_review_findings: 1,
            reclaimable_bytes: 2048,
        };

        let output = render(
            CommandOutput::CleanPreview {
                scan: &scan,
                plan: &plan,
                home: &home,
                awaiting_confirmation: false,
            },
            false,
        );

        assert!(output.contains("Will clean (1)"));
        assert!(output.contains("Not selected — needs review (1)"));
        assert!(output.contains("clean-the-agent clean --apply"));
        assert!(output.contains("Nothing was changed"));
    }

    #[test]
    fn tweak_uses_the_same_document_sections_and_path_formatting() {
        let home = PathBuf::from("/home/test");
        let report = TweakReport {
            id: "codex.disable-pet-shortcut".to_owned(),
            product: "Codex".to_owned(),
            title: "Disable the pet shortcut".to_owned(),
            description: "Remove the active shortcut".to_owned(),
            status: TweakStatus::NeedsChange,
            path: home.join(".codex/keybindings.json"),
            detail: "Other shortcuts are preserved.".to_owned(),
            restart_required: true,
            action: Some(action(
                "tweak",
                &home.join(".codex/keybindings.json"),
                Safety::Automatic,
            )),
            revert_action: None,
        };

        let output = render(
            CommandOutput::TweakPreview {
                report: &report,
                home: &home,
                awaiting_confirmation: false,
            },
            false,
        );

        assert!(output.contains("Proposed change (1)"));
        assert!(output.contains("~/.codex/keybindings.json"));
        assert!(output.contains("clean-the-agent tweak codex.disable-pet-shortcut --apply"));
        assert!(output.contains("Restart Codex"));
    }
}
