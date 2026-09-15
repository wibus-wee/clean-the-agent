use super::*;

impl CleanerApp {
    #[cfg(debug_assertions)]
    pub(super) fn render_ui_lab(&self, cx: &mut ViewContext<'_, Self>, theme: Theme) -> Element {
        let success_toast = cx.listener("ui-lab-toast-success", |this, cx: &mut EventContext| {
            this.toasts.push(
                Toast::new("Cleanup finished")
                    .description("6 items removed · 1.2 GB reclaimed")
                    .kind(ToastKind::Success)
                    .duration(Duration::from_secs(4)),
                Instant::now(),
            );
            cx.invalidate();
        });
        let warning_toast = cx.listener("ui-lab-toast-warning", |this, cx: &mut EventContext| {
            this.toasts.push(
                Toast::new("Some items need review")
                    .description("2 findings were left untouched")
                    .kind(ToastKind::Warning)
                    .duration(Duration::from_secs(5)),
                Instant::now(),
            );
            cx.invalidate();
        });
        let error_toast = cx.listener("ui-lab-toast-error", |this, cx: &mut EventContext| {
            this.toasts.push(
                Toast::new("Cleanup could not finish")
                    .description("The target changed after it was scanned")
                    .kind(ToastKind::Error)
                    .duration(Duration::from_secs(6)),
                Instant::now(),
            );
            cx.invalidate();
        });

        div()
            .id("ui-lab-page")
            .w_full()
            .p_5()
            .flex_col()
            .gap_5()
            .child(
                div()
                    .flex()
                    .items_end()
                    .justify_between()
                    .gap_5()
                    .child(
                        div()
                            .flex_col()
                            .gap_1()
                            .child(text("Acceptance UI Lab").text_2xl().font_semibold())
                            .child(
                                text("Load deterministic states without scanning or touching your files.")
                                    .text_sm()
                                    .text_color(theme.secondary_text),
                            ),
                    )
                    .child(status_badge("Debug builds only", theme.warning, theme)),
            )
            .child(
                div()
                    .flex_col()
                    .gap_2()
                    .child(text("Product screens").text_size(14.0).font_semibold())
                    .child(
                        div()
                            .grid()
                            .grid_cols(2)
                            .gap_3()
                            .child(ui_lab_card(
                                "ui-lab-orca-ready",
                                "Orca · Ready to clean",
                                "8 findings across every capability group, with automatic and review-gated items.",
                                "READY",
                                theme.success,
                                ui_lab_scenario_listener(
                                    cx,
                                    "ui-lab-orca-ready",
                                    UiLabScenario::OrcaReady,
                                ),
                                theme,
                            ))
                            .child(ui_lab_card(
                                "ui-lab-orca-warning",
                                "Orca · Review required",
                                "A populated scan with warnings and review-gated cleanup included.",
                                "REVIEW",
                                theme.warning,
                                ui_lab_scenario_listener(
                                    cx,
                                    "ui-lab-orca-warning",
                                    UiLabScenario::OrcaWarning,
                                ),
                                theme,
                            ))
                            .child(ui_lab_card(
                                "ui-lab-orca-empty",
                                "Orca · All clear",
                                "The completed empty state after all supported artifact types were checked.",
                                "EMPTY",
                                theme.secondary_text,
                                ui_lab_scenario_listener(
                                    cx,
                                    "ui-lab-orca-empty",
                                    UiLabScenario::OrcaEmpty,
                                ),
                                theme,
                            ))
                            .child(ui_lab_card(
                                "ui-lab-orca-scanning",
                                "Orca · Scanning",
                                "The in-progress skeleton and disabled toolbar state without starting background work.",
                                "BUSY",
                                theme.warning,
                                ui_lab_scenario_listener(
                                    cx,
                                    "ui-lab-orca-scanning",
                                    UiLabScenario::OrcaScanning,
                                ),
                                theme,
                            ))
                            .child(ui_lab_card(
                                "ui-lab-overview",
                                "Unified overview",
                                "A populated cross-provider summary for validating counts, hierarchy and navigation.",
                                "2 PROVIDERS",
                                theme.success,
                                ui_lab_scenario_listener(
                                    cx,
                                    "ui-lab-overview",
                                    UiLabScenario::Overview,
                                ),
                                theme,
                            ))
                            .child(ui_lab_card(
                                "ui-lab-codex-allowed",
                                "Codex · Shortcut allowed",
                                "The reversible preference is available and can be toggled in preview mode.",
                                "ALLOWED",
                                theme.secondary_text,
                                ui_lab_scenario_listener(
                                    cx,
                                    "ui-lab-codex-allowed",
                                    UiLabScenario::CodexAllowed,
                                ),
                                theme,
                            ))
                            .child(ui_lab_card(
                                "ui-lab-codex-blocked",
                                "Codex · Shortcut blocked",
                                "The configured state, including the active SVG treatment and restart metadata.",
                                "APPLIED",
                                theme.success,
                                ui_lab_scenario_listener(
                                    cx,
                                    "ui-lab-codex-blocked",
                                    UiLabScenario::CodexBlocked,
                                ),
                                theme,
                            ))
                            .child(ui_lab_card(
                                "ui-lab-codex-attention",
                                "Codex · Needs attention",
                                "The fail-closed state used when a preference cannot be changed safely.",
                                "BLOCKED",
                                theme.danger,
                                ui_lab_scenario_listener(
                                    cx,
                                    "ui-lab-codex-attention",
                                    UiLabScenario::CodexAttention,
                                ),
                                theme,
                            )),
                    ),
            )
            .child(
                div()
                    .flex_col()
                    .gap_2()
                    .child(text("Feedback").text_size(14.0).font_semibold())
                    .child(
                        panel(theme)
                            .p_4()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_4()
                            .child(
                                div()
                                    .flex_col()
                                    .gap_1()
                                    .child(text("Animated toast states").text_size(13.0).font_semibold())
                                    .child(
                                        text("Trigger the production entrance, icon and dismissal animations.")
                                            .text_size(12.0)
                                            .text_color(theme.secondary_text),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(toolbar_button(
                                        "Success",
                                        success_toast,
                                        false,
                                        theme,
                                    ))
                                    .child(toolbar_button(
                                        "Warning",
                                        warning_toast,
                                        false,
                                        theme,
                                    ))
                                    .child(toolbar_button(
                                        "Error",
                                        error_toast,
                                        false,
                                        theme,
                                    )),
                            ),
                    ),
            )
            .child(message_panel(
                "Preview isolation",
                "Mock actions never reach the filesystem. Run Real Scan exits preview mode and restores production data.",
                theme.success,
                theme,
            ))
    }
}

fn ui_lab_scenario_listener(
    cx: &mut ViewContext<'_, CleanerApp>,
    id: &'static str,
    scenario: UiLabScenario,
) -> ClickListener<CleanerApp> {
    cx.listener(id, move |this, cx: &mut EventContext| {
        this.install_ui_lab_scenario(scenario);
        cx.invalidate();
    })
}

fn ui_lab_card<V>(
    id: &'static str,
    title: &'static str,
    detail: &'static str,
    state: &'static str,
    state_color: quickgui::Color,
    listener: ClickListener<V>,
    theme: Theme,
) -> Element {
    button()
        .id(id)
        .group()
        .min_h(124.0)
        .p_4()
        .rounded(14.0)
        .border(1.0, theme.separator)
        .bg(theme.control)
        .flex_col()
        .items_start()
        .justify_between()
        .gap_3()
        .text_left()
        .cursor_pointer()
        .hover(|style| style.bg(theme.control_hover))
        .on_click(listener)
        .child(
            div()
                .w_full()
                .flex()
                .items_center()
                .justify_between()
                .gap_3()
                .child(text(title).text_size(14.0).font_semibold())
                .child(status_badge(state, state_color, theme)),
        )
        .child(
            text(detail)
                .text_size(12.0)
                .line_height(18.0)
                .text_color(theme.secondary_text),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_1()
                .text_size(12.0)
                .font_medium()
                .child("Open preview")
                .child(text("→").text_color(theme.secondary_text)),
        )
}

fn mock_cleanup_action(
    id: &str,
    path: &str,
    safety: Safety,
    reclaimable_bytes: u64,
) -> CleanupAction {
    CleanupAction {
        id: format!("mock-action-{id}"),
        provider: "orca".to_owned(),
        description: format!("remove {id}"),
        path: PathBuf::from(path),
        scope: ScopeKind::Local,
        safety,
        reclaimable_bytes,
        kind: CleanupActionKind::RemovePath {
            expected: PathSnapshot {
                digest: "0".repeat(64),
                entries: 12,
                bytes: reclaimable_bytes,
            },
        },
        depends_on: Vec::new(),
    }
}

#[allow(clippy::too_many_arguments)]
fn mock_finding(
    id: &str,
    kind: ArtifactKind,
    ownership: Ownership,
    safety: Safety,
    path: &str,
    description: &str,
    evidence: &str,
    reclaimable_bytes: u64,
) -> Finding {
    Finding {
        id: format!("mock-{id}"),
        provider: "orca".to_owned(),
        kind,
        ownership,
        safety,
        path: PathBuf::from(path),
        scope: if kind == ArtifactKind::RemoteRuntime {
            ScopeKind::SshRemote
        } else {
            ScopeKind::Local
        },
        description: description.to_owned(),
        evidence: evidence.to_owned(),
        reclaimable_bytes,
        action: (safety != Safety::Informational)
            .then(|| mock_cleanup_action(id, path, safety, reclaimable_bytes)),
    }
}

pub(super) fn mock_orca_report(with_warnings: bool) -> ScanReport {
    const MB: u64 = 1024 * 1024;
    let warnings = if with_warnings {
        vec![
            "A mounted SSH home became unavailable after discovery.".to_owned(),
            "One managed hook changed while the scan was running.".to_owned(),
        ]
    } else {
        Vec::new()
    };
    let mut findings = vec![
        mock_finding(
            "cache",
            ArtifactKind::Cache,
            Ownership::ProviderOwned,
            Safety::Automatic,
            "/Users/demo/Library/Caches/com.orca.agent",
            "Orca download and model metadata cache.",
            "The directory contains Orca's provider marker and cache manifest.",
            842 * MB,
        ),
        mock_finding(
            "logs",
            ArtifactKind::Log,
            Ownership::ProviderOwned,
            Safety::Automatic,
            "/Users/demo/Library/Logs/Orca",
            "Rotated agent and extension logs.",
            "Every file is located below Orca's owned logging root.",
            286 * MB,
        ),
        mock_finding(
            "worktree",
            ArtifactKind::Worktree,
            Ownership::Attributed,
            Safety::ReviewRequired,
            "/Users/demo/Projects/.orca/worktrees/stale-feature",
            "A detached worktree from a completed agent task.",
            "Orca metadata attributes the worktree to a finished task, but uncommitted files remain.",
            1_420 * MB,
        ),
        mock_finding(
            "hook",
            ArtifactKind::ConfigMutation,
            Ownership::InjectedByProvider,
            Safety::ReviewRequired,
            "/Users/demo/.claude/settings.json",
            "Orca hook entries in Claude settings.",
            "The configuration contains Orca-managed hook entries alongside user settings.",
            8 * 1024,
        ),
        mock_finding(
            "runtime",
            ArtifactKind::RuntimeState,
            Ownership::ProviderOwned,
            Safety::Automatic,
            "/Users/demo/.orca/runtime/sessions",
            "Completed local agent session state.",
            "Session leases are closed and the runtime index has no active references.",
            96 * MB,
        ),
        mock_finding(
            "history",
            ArtifactKind::UserHistory,
            Ownership::Attributed,
            Safety::ReviewRequired,
            "/Users/demo/.orca/history.jsonl",
            "Prompt and command history retained by Orca.",
            "The file is user history, so Clean the Agent requires explicit review before removal.",
            18 * MB,
        ),
        mock_finding(
            "skill",
            ArtifactKind::SkillPlacement,
            Ownership::InjectedByProvider,
            Safety::Automatic,
            "/Users/demo/.agents/skills/orca-legacy",
            "An obsolete Orca-managed skill installation.",
            "Its manifest points to a provider version that is no longer installed.",
            3 * MB,
        ),
        mock_finding(
            "remote",
            ArtifactKind::RemoteRuntime,
            Ownership::Attributed,
            Safety::ReviewRequired,
            "/Volumes/dev-home/.orca/runtime",
            "Inactive runtime state from a mounted development host.",
            "The remote lease is expired, but the host is currently offline.",
            614 * MB,
        ),
    ];
    if let Some(action) = findings[2].action.as_mut() {
        action.kind = CleanupActionKind::RemoveGitWorktree {
            repository: PathBuf::from("/Users/demo/Projects/atlas"),
            expected: PathSnapshot {
                digest: "0".repeat(64),
                entries: 24,
                bytes: 1_420 * MB,
            },
        };
        action.description = "remove the stale Orca Git worktree".to_owned();
    }
    if let Some(action) = findings[3].action.as_mut() {
        action.kind = CleanupActionKind::RewriteFile {
            expected_sha256: "0".repeat(64),
            mutation_count: 2,
            format: FileFormat::Json,
            replacement: b"{}\n".to_vec(),
        };
        action.description = "remove two Orca-managed Claude hook entries".to_owned();
    }
    ScanReport { findings, warnings }
}

pub(super) fn mock_codex_report(status: TweakStatus) -> TweakReport {
    let path = "/Users/demo/.codex/keybindings.json";
    let action = mock_cleanup_action("codex-pet-block", path, Safety::Automatic, 0);
    let revert_action = mock_cleanup_action("codex-pet-allow", path, Safety::Automatic, 0);
    let detail = match status {
        TweakStatus::NeedsChange => "Codex Pet can currently be activated from the keyboard",
        TweakStatus::Satisfied => "Codex Pet already has an explicit null keyboard binding",
        TweakStatus::Blocked => "The keybindings file changed after it was inspected",
    };
    TweakReport {
        id: "codex.disable-pet-shortcut".to_owned(),
        product: "Codex".to_owned(),
        title: "Disable the Pet keyboard shortcut".to_owned(),
        description: "Keep Codex Pet available from menus while preventing keyboard activation"
            .to_owned(),
        status,
        path: PathBuf::from(path),
        detail: detail.to_owned(),
        restart_required: status != TweakStatus::Blocked,
        action: (status == TweakStatus::NeedsChange).then_some(action),
        revert_action: (status == TweakStatus::Satisfied).then_some(revert_action),
    }
}
