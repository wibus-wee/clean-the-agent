//! Business-aware view helpers shared by the application pages.

use super::*;

pub(super) fn about_provider_row(
    glyph: Element,
    title: &str,
    detail: &str,
    kind: &str,
    theme: Theme,
) -> Element {
    div()
        .h(42.0)
        .px_2()
        .flex()
        .items_center()
        .justify_between()
        .gap_2()
        .child(
            div()
                .min_w(0.0)
                .flex()
                .items_center()
                .gap_2()
                .child(glyph)
                .child(
                    div()
                        .min_w(0.0)
                        .flex_col()
                        .child(text(title).text_size(12.0).font_semibold())
                        .child(
                            text(detail)
                                .text_size(11.0)
                                .text_color(theme.secondary_text)
                                .truncate(),
                        ),
                ),
        )
        .child(
            text(kind)
                .text_size(11.0)
                .font_medium()
                .text_color(theme.secondary_text),
        )
}

pub(super) fn about_principle(
    glyph: IconName,
    title: &str,
    detail: &str,
    top_border: bool,
    theme: Theme,
) -> Element {
    div()
        .h(48.0)
        .px_2()
        .when(top_border, |item| item.border_top(1.0, theme.separator))
        .flex()
        .items_center()
        .gap_2()
        .child(
            div()
                .w(26.0)
                .h(26.0)
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .child(icon(glyph, 15.0, theme.secondary_text)),
        )
        .child(
            div()
                .min_w(0.0)
                .flex_grow(1.0)
                .flex_col()
                .child(text(title).text_size(12.0).font_semibold())
                .child(
                    text(detail)
                        .min_w(0.0)
                        .text_size(11.0)
                        .text_color(theme.secondary_text)
                        .truncate(),
                ),
        )
}

pub(super) fn tweak_meta(
    glyph: IconName,
    label: &str,
    value: &str,
    monospace: bool,
    theme: Theme,
) -> Element {
    div()
        .min_w(0.0)
        .flex()
        .items_start()
        .gap_2()
        .child(icon(glyph, 17.0, theme.secondary_text))
        .child(
            div()
                .min_w(0.0)
                .flex_col()
                .gap_1()
                .child(
                    text(label)
                        .text_size(11.0)
                        .font_semibold()
                        .text_color(theme.secondary_text),
                )
                .child(
                    text(value)
                        .text_size(12.0)
                        .line_height(17.0)
                        .when(monospace, |value| value.font_family("Geist Mono")),
                ),
        )
}

pub(super) fn app_icon() -> quickgui::Image {
    static ICON: OnceLock<quickgui::Image> = OnceLock::new();
    ICON.get_or_init(|| {
        quickgui::Image::decode(include_bytes!("../../resources/app/icon.png"))
            .expect("bundled application icon must be a valid PNG")
    })
    .clone()
}

pub(super) fn orca_icon() -> quickgui::Image {
    static ICON: OnceLock<quickgui::Image> = OnceLock::new();
    ICON.get_or_init(|| {
        quickgui::Image::decode(include_bytes!("../../resources/providers/orca.png"))
            .expect("bundled Orca provider icon must be a valid PNG")
    })
    .clone()
}

pub(super) fn status_strip(
    app: &CleanerApp,
    choose_home: ClickListener<CleanerApp>,
    theme: Theme,
) -> Element {
    let (findings, actions, reclaimable, warnings) =
        app.report.as_ref().map_or((0, 0, 0, 0), |report| {
            let plan = app.cleanup_plan(report);
            (
                report.findings.len(),
                plan.actions.len(),
                plan.reclaimable_bytes,
                report.warnings.len(),
            )
        });

    panel(theme)
        .id("status-strip")
        .overflow_hidden()
        .flex_col()
        .child(
            div()
                .h(52.0)
                .px_3()
                .flex()
                .items_center()
                .justify_between()
                .gap_4()
                .child(
                    div()
                        .flex_none()
                        .flex_col()
                        .gap_1()
                        .child(text("Orca scan location").text_size(13.0).font_medium())
                        .child(
                            text(if app.selected_home.is_some() {
                                "Another home folder"
                            } else {
                                "Current macOS home"
                            })
                            .text_size(11.0)
                            .text_color(theme.secondary_text),
                        ),
                )
                .child(
                    div()
                        .min_w(0.0)
                        .flex()
                        .items_center()
                        .justify_end()
                        .gap_3()
                        .child(
                            text(app.display_home.display().to_string())
                                .min_w(0.0)
                                .text_size(12.0)
                                .font_family("Geist Mono")
                                .truncate(),
                        )
                        .child(toolbar_button("Change…", choose_home, app.busy(), theme)),
                ),
        )
        .child(
            div()
                .h(64.0)
                .flex()
                .border_top(1.0, theme.separator)
                .child(status_value("Findings", findings.to_string(), false, theme))
                .child(status_value("Ready", actions.to_string(), true, theme))
                .child(status_value(
                    "Reclaimable",
                    human_bytes(reclaimable),
                    true,
                    theme,
                ))
                .child(status_value("Warnings", warnings.to_string(), true, theme)),
        )
}

pub(super) fn status_value(label: &str, value: String, separated: bool, theme: Theme) -> Element {
    div()
        .h_full()
        .min_w(0.0)
        .flex_grow(1.0)
        .flex_basis(0.0)
        .px_3()
        .flex_col()
        .justify_center()
        .gap_1()
        .when(separated, |item| item.border_left(1.0, theme.separator))
        .child(
            text(label)
                .text_size(12.0)
                .font_medium()
                .text_color(theme.secondary_text),
        )
        .child(text(value).text_size(18.0).font_semibold().truncate())
}

pub(super) fn overview_metric(
    label: &str,
    value: String,
    separated: bool,
    theme: Theme,
) -> Element {
    div()
        .h_full()
        .min_w(0.0)
        .flex_grow(1.0)
        .flex_basis(0.0)
        .px_4()
        .flex_col()
        .justify_center()
        .gap_1()
        .when(separated, |metric| metric.border_left(1.0, theme.separator))
        .child(
            text(label)
                .text_size(11.0)
                .font_medium()
                .text_color(theme.secondary_text),
        )
        .child(text(value).text_size(17.0).font_semibold().truncate())
}

pub(super) fn scan_outcome_panel(
    title: &str,
    detail: &str,
    success: bool,
    theme: Theme,
) -> Element {
    panel(theme)
        .min_h(132.0)
        .p_5()
        .flex()
        .items_center()
        .gap_4()
        .child(
            div()
                .w(44.0)
                .h(44.0)
                .rounded(14.0)
                .border(1.0, theme.separator)
                .bg(theme.sidebar)
                .flex()
                .items_center()
                .justify_center()
                .text_size(if success { 18.0 } else { 16.0 })
                .font_medium()
                .child(if success { "✓" } else { "•••" }),
        )
        .child(
            div()
                .min_w(0.0)
                .flex_col()
                .gap_2()
                .child(text(title).text_size(16.0).font_semibold())
                .child(
                    text(detail)
                        .text_size(13.0)
                        .line_height(20.0)
                        .text_color(theme.secondary_text),
                ),
        )
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum FindingGroup {
    PermanentDelete,
    ConfigurationEdit,
    ExcludedBySelection,
    NeedsReview,
    ReportedOnly,
    ExcludedByScope,
}

impl FindingGroup {
    pub(super) const ALL: [Self; 6] = [
        Self::PermanentDelete,
        Self::ConfigurationEdit,
        Self::ExcludedBySelection,
        Self::NeedsReview,
        Self::ReportedOnly,
        Self::ExcludedByScope,
    ];

    pub(super) const fn title(self) -> &'static str {
        match self {
            Self::PermanentDelete => "Will be permanently deleted",
            Self::ConfigurationEdit => "Configuration files to be edited",
            Self::ExcludedBySelection => "Excluded by you",
            Self::NeedsReview => "Needs your review",
            Self::ReportedOnly => "Reported only",
            Self::ExcludedByScope => "Excluded by cleanup scope",
        }
    }

    pub(super) const fn id(self) -> &'static str {
        match self {
            Self::PermanentDelete => "finding-group-delete",
            Self::ConfigurationEdit => "finding-group-edit",
            Self::ExcludedBySelection => "finding-group-user-excluded",
            Self::NeedsReview => "finding-group-review",
            Self::ReportedOnly => "finding-group-info",
            Self::ExcludedByScope => "finding-group-scope",
        }
    }

    pub(super) const fn status(self, theme: Theme) -> (&'static str, quickgui::Color) {
        match self {
            Self::PermanentDelete => ("Will delete", theme.danger),
            Self::ConfigurationEdit => ("Will edit", theme.warning),
            Self::ExcludedBySelection => ("Not selected", theme.secondary_text),
            Self::NeedsReview => ("Excluded", theme.warning),
            Self::ReportedOnly => ("No changes", theme.secondary_text),
            Self::ExcludedByScope => ("Excluded", theme.secondary_text),
        }
    }
}

pub(super) fn finding_group(
    finding: &Finding,
    included_by_scope: bool,
    included_in_plan: bool,
    excluded_by_selection: bool,
) -> FindingGroup {
    if finding.action.is_some() && !included_by_scope {
        return FindingGroup::ExcludedByScope;
    }
    if finding.action.is_some() && excluded_by_selection {
        return FindingGroup::ExcludedBySelection;
    }
    if finding.safety == Safety::ReviewRequired && finding.action.is_some() && !included_in_plan {
        return FindingGroup::NeedsReview;
    }
    if !included_in_plan {
        return FindingGroup::ReportedOnly;
    }
    match finding.action.as_ref().map(|action| &action.kind) {
        Some(CleanupActionKind::RewriteFile { .. } | CleanupActionKind::EnsureFile { .. }) => {
            FindingGroup::ConfigurationEdit
        }
        Some(
            CleanupActionKind::RemoveGitWorktree { .. }
            | CleanupActionKind::RemovePath { .. }
            | CleanupActionKind::RemoveEmptyDirectory,
        ) => FindingGroup::PermanentDelete,
        None => FindingGroup::ReportedOnly,
    }
}

pub(super) fn finding_row<V>(
    finding: &Finding,
    group: FindingGroup,
    selected: bool,
    selection: Option<(bool, bool, quickgui::ClickListener<V>)>,
    listener: quickgui::ClickListener<V>,
    theme: Theme,
) -> Element {
    let secondary = if selected {
        theme.selection_text
    } else {
        theme.secondary_text
    };
    let (status, status_color) = group.status(theme);
    let size = if finding.reclaimable_bytes > 0 {
        human_bytes(finding.reclaimable_bytes)
    } else {
        "—".to_owned()
    };
    let content = div()
        .min_w(0.0)
        .flex_grow(1.0)
        .flex_col()
        .items_start()
        .gap_1()
        .child(
            div()
                .w_full()
                .flex()
                .items_center()
                .justify_between()
                .gap_3()
                .child(
                    text(finding.description.as_str())
                        .min_w(0.0)
                        .text_size(13.0)
                        .font_semibold()
                        .truncate(),
                )
                .child(
                    div()
                        .flex_none()
                        .flex()
                        .items_center()
                        .gap_1()
                        .child(div().w(6.0).h(6.0).rounded_full().bg(status_color))
                        .child(text(status).text_size(11.0).text_color(secondary)),
                ),
        )
        .child(
            text(finding.path.display().to_string())
                .text_size(11.0)
                .font_family("Geist Mono")
                .text_color(secondary)
                .truncate(),
        )
        .child(
            div()
                .w_full()
                .flex()
                .items_center()
                .justify_between()
                .gap_3()
                .child(
                    text(format!(
                        "{} · {} · {}",
                        artifact_label(finding.kind),
                        ownership_label(finding.ownership),
                        scope_label(finding.scope)
                    ))
                    .text_size(11.0)
                    .text_color(secondary)
                    .truncate(),
                )
                .child(
                    text(size)
                        .flex_none()
                        .text_size(11.0)
                        .font_medium()
                        .text_color(secondary),
                ),
        );
    let mut row = div()
        .w_full()
        .min_h(80.0)
        .px_3()
        .py(10.0)
        .flex()
        .items_center()
        .gap_2()
        .border_bottom(1.0, theme.separator)
        .when(selected, |row| {
            row.bg(theme.selection).text_color(theme.selection_text)
        })
        .when(!selected, |row| row.hover(|style| style.bg(theme.control)))
        .cursor_pointer()
        .on_click(listener);
    if let Some((checked, enabled, toggle)) = selection {
        row = row.child(
            checkbox(checked)
                .id(format!("toggle-finding-{}", finding.id))
                .flex_none()
                .w(16.0)
                .h(16.0)
                .rounded(4.0)
                .border(
                    1.0,
                    if checked {
                        theme.accent
                    } else {
                        theme.separator
                    },
                )
                .bg(if checked { theme.accent } else { theme.window })
                .text_color(theme.accent_text)
                .flex()
                .items_center()
                .justify_center()
                .text_size(11.0)
                .accessibility_label(if checked {
                    "Exclude this finding from cleanup"
                } else {
                    "Include this finding in cleanup"
                })
                .disabled(!enabled)
                .when(!enabled, |control| control.opacity(0.38))
                .when(enabled, |control| control.cursor_pointer())
                .on_click(toggle)
                .when(checked, |control| control.child("✓")),
        );
    } else {
        row = row.child(div().w(16.0).h(16.0).flex_none());
    }
    row.child(content)
}

pub(super) fn safety_badge(group: FindingGroup, safety: Safety, theme: Theme) -> Element {
    let (label, color) = match group {
        FindingGroup::PermanentDelete | FindingGroup::ConfigurationEdit
            if safety == Safety::Automatic =>
        {
            ("Included by default · proven safe", theme.success)
        }
        FindingGroup::PermanentDelete | FindingGroup::ConfigurationEdit => {
            ("Included after review", theme.warning)
        }
        FindingGroup::ExcludedBySelection => ("Excluded by you", theme.secondary_text),
        FindingGroup::NeedsReview => ("Needs review · excluded", theme.warning),
        FindingGroup::ReportedOnly => ("Info only · no changes", theme.secondary_text),
        FindingGroup::ExcludedByScope => ("Excluded by cleanup scope", theme.secondary_text),
    };
    status_badge(label, color, theme)
}

pub(super) fn action_consequence(finding: &Finding) -> (&'static str, String) {
    match finding.action.as_ref().map(|action| &action.kind) {
        Some(CleanupActionKind::RewriteFile { mutation_count, .. }) => (
            "Edit this configuration file",
            format!(
                "Remove {mutation_count} Orca-managed {}. Other settings stay unchanged. The file is edited only if it still matches the scanned version.",
                if *mutation_count == 1 { "entry" } else { "entries" }
            ),
        ),
        Some(CleanupActionKind::EnsureFile { mutation_count, .. }) => (
            "Update this configuration file",
            format!(
                "Apply {mutation_count} Orca-managed {} without overwriting unrelated settings or a target that changed after scanning.",
                if *mutation_count == 1 { "entry" } else { "entries" }
            ),
        ),
        Some(CleanupActionKind::RemoveGitWorktree { .. }) => (
            "Remove this Git worktree",
            "Git removes it permanently only if the worktree is unchanged, clean and still registered. It is not moved to Trash."
                .to_owned(),
        ),
        Some(CleanupActionKind::RemovePath { expected }) if expected.entries > 1 => (
            "Permanently delete this directory",
            "The directory and everything inside it are deleted directly. Nothing is moved to Trash, and cleanup stops if its contents changed after scanning."
                .to_owned(),
        ),
        Some(CleanupActionKind::RemovePath { .. }) => (
            "Permanently delete this item",
            "The item is deleted directly instead of being moved to Trash. Cleanup stops if it changed after scanning."
                .to_owned(),
        ),
        Some(CleanupActionKind::RemoveEmptyDirectory) => (
            "Delete this empty directory",
            "The directory is deleted directly only if it is still empty. Otherwise it is safely skipped; nothing is moved to Trash."
                .to_owned(),
        ),
        None => (
            "No cleanup action",
            "This finding is shown for context only. Clean the Agent will not change it."
                .to_owned(),
        ),
    }
}

pub(super) fn recognition_reason(finding: &Finding) -> &'static str {
    match finding.ownership {
        Ownership::ProviderOwned => {
            "It is inside an Orca-owned location and matches a known Orca artifact."
        }
        Ownership::InjectedByProvider => {
            "The file contains entries marked as Orca-managed; unrelated content is outside this finding."
        }
        Ownership::Attributed => {
            "Orca state links this item to a recorded Orca task, workspace or installation."
        }
    }
}

pub(super) fn cleanup_plan_summary(
    report: &ScanReport,
    plan: &CleanupPlan,
    user_excluded: usize,
    theme: Theme,
) -> Element {
    let deleted = plan
        .actions
        .iter()
        .filter(|action| {
            matches!(
                action.kind,
                CleanupActionKind::RemoveGitWorktree { .. }
                    | CleanupActionKind::RemovePath { .. }
                    | CleanupActionKind::RemoveEmptyDirectory
            )
        })
        .count();
    let edited = plan.actions.len().saturating_sub(deleted);
    let informational = report
        .findings
        .iter()
        .filter(|finding| finding.safety == Safety::Informational || finding.action.is_none())
        .count();

    div()
        .id("cleanup-plan")
        .w_full()
        .py(12.0)
        .border_top(1.0, theme.separator)
        .border_bottom(1.0, theme.separator)
        .flex_col()
        .gap_2()
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(text("Cleanup plan").text_size(14.0).font_semibold())
                .child(
                    text(human_bytes(plan.reclaimable_bytes))
                        .text_size(12.0)
                        .font_medium()
                        .text_color(theme.secondary_text),
                ),
        )
        .child(
            div()
                .grid()
                .grid_cols(2)
                .gap_2()
                .child(plan_summary_line(
                    theme.danger,
                    format!(
                        "{deleted} {} will be permanently deleted",
                        plural(deleted, "item", "items")
                    ),
                    theme,
                ))
                .child(plan_summary_line(
                    theme.warning,
                    format!(
                        "{edited} configuration {} will be edited",
                        plural(edited, "file", "files")
                    ),
                    theme,
                ))
                .child(plan_summary_line(
                    theme.warning,
                    format!(
                        "{} {} require review and are excluded",
                        plan.excluded_review_findings,
                        plural(plan.excluded_review_findings, "item", "items")
                    ),
                    theme,
                ))
                .child(plan_summary_line(
                    theme.secondary_text,
                    format!(
                        "{informational} {} are informational only",
                        plural(informational, "item", "items")
                    ),
                    theme,
                ))
                .child(plan_summary_line(
                    theme.secondary_text,
                    format!(
                        "{user_excluded} {} excluded by you",
                        plural(user_excluded, "item", "items")
                    ),
                    theme,
                )),
        )
        .child(
            text("Nothing will be moved to Trash. Cleanup cannot be undone.")
                .text_size(11.0)
                .font_medium()
                .text_color(theme.danger),
        )
}

fn plan_summary_line(color: quickgui::Color, label: String, theme: Theme) -> Element {
    div()
        .min_w(0.0)
        .flex()
        .items_center()
        .gap_2()
        .child(div().w(6.0).h(6.0).flex_none().rounded_full().bg(color))
        .child(text(label).text_size(11.0).text_color(theme.secondary_text))
}

const fn plural<'a>(count: usize, singular: &'a str, plural: &'a str) -> &'a str {
    if count == 1 { singular } else { plural }
}

pub(super) const fn artifact_label(kind: ArtifactKind) -> &'static str {
    match kind {
        ArtifactKind::OwnedData => "Application data",
        ArtifactKind::Cache => "Cache",
        ArtifactKind::Log => "Log",
        ArtifactKind::TemporaryState => "Temporary state",
        ArtifactKind::RuntimeState => "Runtime state",
        ArtifactKind::Worktree => "Git worktree",
        ArtifactKind::WorktreeTrash => "Worktree trash",
        ArtifactKind::ManagedHook => "Agent hook",
        ArtifactKind::ConfigMutation => "Configuration entry",
        ArtifactKind::OrphanedState => "Orphaned state",
        ArtifactKind::UserHistory => "User history",
        ArtifactKind::TrustEntry => "Workspace trust",
        ArtifactKind::Plugin => "Integration plugin",
        ArtifactKind::SkillPlacement => "Skill placement",
        ArtifactKind::Backup => "Configuration backup",
        ArtifactKind::RemoteRuntime => "Remote runtime",
    }
}

pub(super) const fn ownership_label(ownership: Ownership) -> &'static str {
    match ownership {
        Ownership::ProviderOwned => "Orca-owned",
        Ownership::InjectedByProvider => "Orca-managed",
        Ownership::Attributed => "Attributed to Orca",
    }
}

pub(super) const fn scope_label(scope: ScopeKind) -> &'static str {
    match scope {
        ScopeKind::Local => "Local Mac",
        ScopeKind::Wsl => "WSL home",
        ScopeKind::SshRemote => "SSH home",
    }
}

pub(super) fn finding_detail_section(
    glyph: IconName,
    title: &str,
    detail: &str,
    monospace: bool,
    theme: Theme,
) -> Element {
    div()
        .w_full()
        .flex_none()
        .py(12.0)
        .border_top(1.0, theme.separator)
        .flex()
        .items_start()
        .gap_2()
        .child(
            div()
                .w(28.0)
                .h(28.0)
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .child(icon(glyph, 16.0, theme.secondary_text)),
        )
        .child(
            div()
                .min_w(0.0)
                .flex_col()
                .child(text(title).text_size(12.0).font_semibold())
                .child(
                    text(detail)
                        .text_size(12.0)
                        .line_height(18.0)
                        .text_color(theme.secondary_text)
                        .when(monospace, |copy| copy.font_family("Geist Mono").truncate()),
                ),
        )
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct ApplyCounts {
    pub(super) applied: usize,
    pub(super) skipped: usize,
    pub(super) failed: usize,
}

pub(super) fn apply_counts(report: &ApplyReport) -> ApplyCounts {
    let applied = report
        .results
        .iter()
        .filter(|result| result.status == ApplyStatus::Applied)
        .count();
    let failed = report
        .results
        .iter()
        .filter(|result| result.status == ApplyStatus::Failed)
        .count();
    ApplyCounts {
        applied,
        skipped: report.results.len().saturating_sub(applied + failed),
        failed,
    }
}

pub(super) fn apply_toast(label: &str, counts: ApplyCounts) -> Toast {
    let result = format!(
        "{} applied, {} skipped, {} failed.",
        counts.applied, counts.skipped, counts.failed
    );
    if counts.failed > 0 {
        return Toast::new(if label == "Tweak" {
            "Codex preference needs attention"
        } else {
            "Cleanup needs attention"
        })
        .description(result)
        .kind(ToastKind::Error)
        .duration(Duration::from_secs(8));
    }
    if counts.skipped > 0 {
        return Toast::new(if label == "Tweak" {
            "Codex preference was not fully updated"
        } else {
            "Cleanup finished with skipped items"
        })
        .description(result)
        .kind(ToastKind::Warning)
        .duration(Duration::from_secs(7));
    }
    Toast::new(if label == "Tweak" {
        "Codex preference updated"
    } else {
        "Cleanup finished"
    })
    .description(if label == "Tweak" {
        format!(
            "Applied {} {}. Restart Codex to use it.",
            counts.applied,
            if counts.applied == 1 {
                "change"
            } else {
                "changes"
            }
        )
    } else {
        result
    })
    .kind(ToastKind::Success)
    .duration(Duration::from_secs(4))
}

pub(super) fn apply_result_panel(label: &str, report: &ApplyReport, theme: Theme) -> Element {
    let counts = apply_counts(report);
    let color = if counts.failed == 0 {
        theme.success
    } else {
        theme.danger
    };
    panel(theme)
        .p_3()
        .border_left(3.0, color)
        .flex_col()
        .gap_2()
        .child(text(format!("{label} finished")).font_semibold())
        .child(
            text(format!(
                "{} applied, {} skipped, {} failed.",
                counts.applied, counts.skipped, counts.failed
            ))
            .text_sm()
            .text_color(theme.secondary_text),
        )
        .children(report.results.iter().map(|result| {
            let result_color = match result.status {
                ApplyStatus::Applied => theme.success,
                ApplyStatus::Skipped => theme.secondary_text,
                ApplyStatus::Failed => theme.danger,
            };
            div()
                .p_2()
                .rounded(6.0)
                .border(1.0, theme.separator)
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap_2()
                        .child(text(result.path.display().to_string()).text_sm().truncate())
                        .child(status_badge(
                            &humanize_debug(&format!("{:?}", result.status)),
                            result_color,
                            theme,
                        )),
                )
                .child(
                    text(result.detail.as_str())
                        .text_xs()
                        .text_color(theme.secondary_text),
                )
        }))
}

pub(super) fn warnings_panel(warnings: &[String], theme: Theme) -> Element {
    panel(theme)
        .p_3()
        .border_left(3.0, theme.warning)
        .flex_col()
        .gap_2()
        .child(text("Scan Warnings").font_semibold())
        .children(warnings.iter().map(|warning| {
            text(warning.as_str())
                .text_sm()
                .text_color(theme.secondary_text)
        }))
}
