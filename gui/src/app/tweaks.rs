use super::*;

impl CleanerApp {
    pub(super) fn render_tweaks(&self, cx: &mut ViewContext<'_, Self>, theme: Theme) -> Element {
        let mut content = div()
            .w_full()
            .p_5()
            .flex_col()
            .gap_3()
            .child(self.render_codex_tweak_header(theme));

        if let Some(error) = &self.error {
            content = content.child(message_panel(
                "The operation could not finish",
                error,
                theme.danger,
                theme,
            ));
        }
        if let Some((label, report)) = &self.last_apply
            && label == "Tweak"
        {
            content = content.child(apply_result_panel(label, report, theme));
        }

        if self.tweak_reports.is_empty() {
            return content.child(empty_state(
                if self.scanning {
                    "Inspecting Codex preferences…"
                } else {
                    "No Codex preferences available"
                },
                "Scan again to refresh the built-in preference recipes.",
                theme,
            ));
        }

        let selected = self
            .selected_tweak
            .unwrap_or(0)
            .min(self.tweak_reports.len().saturating_sub(1));
        let settings = self
            .tweak_reports
            .iter()
            .enumerate()
            .map(|(index, report)| {
                self.render_tweak_row(cx, index, report, selected == index, theme)
            });
        content.child(
            panel(theme)
                .id("codex-settings-browser")
                .h(490.0)
                .min_h(420.0)
                .p(1.0)
                .overflow_hidden()
                .flex()
                .child(
                    div()
                        .w(300.0)
                        .h_full()
                        .flex_none()
                        .flex_col()
                        .border_right(1.0, theme.separator)
                        .child(
                            div()
                                .h(38.0)
                                .px_3()
                                .flex()
                                .items_center()
                                .border_bottom(1.0, theme.separator)
                                .bg(theme.sidebar)
                                .child(
                                    text(format!("SETTINGS · {}", self.tweak_reports.len()))
                                        .text_size(11.0)
                                        .font_semibold()
                                        .text_color(theme.secondary_text),
                                ),
                        )
                        .child(
                            div()
                                .flex_grow(1.0)
                                .min_h(0.0)
                                .flex_col()
                                .overflow_y_scroll()
                                .children(settings),
                        ),
                )
                .child(self.render_tweak_detail(
                    cx,
                    selected,
                    &self.tweak_reports[selected],
                    theme,
                )),
        )
    }

    fn render_codex_tweak_header(&self, theme: Theme) -> Element {
        let count = self.tweak_reports.len();
        let (availability, availability_color) = if self.scanning {
            ("Inspecting", theme.warning)
        } else if count == 0 {
            ("Unavailable", theme.danger)
        } else {
            ("Available", theme.success)
        };
        div()
            .id("provider-codex")
            .h(66.0)
            .px(4.0)
            .flex()
            .items_center()
            .justify_between()
            .gap_5()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .min_w(0.0)
                    .child(
                        div()
                            .w(40.0)
                            .h(40.0)
                            .rounded(12.0)
                            .border(1.0, theme.separator)
                            .bg(theme.sidebar)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(chatgpt_icon(30.0, theme.dark)),
                    )
                    .child(
                        div()
                            .flex_col()
                            .gap_1()
                            .min_w(0.0)
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(text("Codex preferences").text_xl().font_semibold())
                                    .child(status_badge(
                                        availability,
                                        availability_color,
                                        theme,
                                    )),
                            )
                            .child(
                                text("Explicit, reversible settings · never changed by automatic cleanup")
                                    .text_size(12.0)
                                    .text_color(theme.secondary_text),
                            ),
                    ),
            )
            .child(
                div()
                    .flex_col()
                    .items_end()
                    .gap_1()
                    .child(
                        text(format!(
                            "{count} {}",
                            if count == 1 { "preference" } else { "preferences" }
                        ))
                        .text_sm()
                        .font_semibold(),
                    )
                    .child(
                        text("Scanned with Overview")
                            .text_size(12.0)
                            .text_color(theme.secondary_text),
                    ),
            )
    }

    fn render_tweak_row(
        &self,
        cx: &mut ViewContext<'_, Self>,
        index: usize,
        report: &TweakReport,
        selected: bool,
        theme: Theme,
    ) -> Element {
        let select = cx.listener(format!("select-tweak-{index}"), move |this, cx| {
            this.selected_tweak = Some(index);
            cx.invalidate();
        });
        let (status, color) = match report.status {
            TweakStatus::NeedsChange => ("Allowed", theme.secondary_text),
            TweakStatus::Satisfied => ("Blocked", theme.success),
            TweakStatus::Blocked => ("Attention", theme.danger),
        };
        button()
            .id(format!("select-tweak-{index}"))
            .w_full()
            .min_h(70.0)
            .p_3()
            .flex()
            .items_center()
            .gap_3()
            .border_bottom(1.0, theme.separator)
            .cursor_pointer()
            .when(selected, |row| row.bg(theme.selection))
            .when(!selected, |row| {
                row.hover(|style| style.bg(theme.control_hover))
            })
            .on_click(select)
            .child(
                div()
                    .w(34.0)
                    .h(34.0)
                    .flex_none()
                    .rounded(10.0)
                    .border(1.0, theme.separator)
                    .bg(theme.sidebar)
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(animated_keyboard_icon(
                        18.0,
                        report.status == TweakStatus::Satisfied,
                        theme.text,
                    )),
            )
            .child(
                div()
                    .min_w(0.0)
                    .flex_grow(1.0)
                    .flex_col()
                    .gap_1()
                    .child(
                        text("Pet keyboard shortcut")
                            .text_size(13.0)
                            .font_semibold(),
                    )
                    .child(text(status).text_size(11.0).text_color(color)),
            )
            .child(text("›").text_lg().text_color(theme.secondary_text))
    }

    fn render_tweak_detail(
        &self,
        cx: &mut ViewContext<'_, Self>,
        index: usize,
        report: &TweakReport,
        theme: Theme,
    ) -> Element {
        let enabled = report.status == TweakStatus::Satisfied;
        let action = if enabled {
            report.revert_action.clone()
        } else {
            report.action.clone()
        };
        let can_change = action.is_some();
        let change = cx.listener(
            format!("change-tweak-{index}"),
            move |this, cx: &mut EventContext| {
                if this.busy() {
                    return;
                }
                #[cfg(debug_assertions)]
                if this.preview_mode {
                    let next = if this.tweak_reports[index].status == TweakStatus::Satisfied {
                        TweakStatus::NeedsChange
                    } else {
                        TweakStatus::Satisfied
                    };
                    this.tweak_reports[index] = mock_codex_report(next);
                    this.toasts.push(
                        Toast::new("Preview preference updated")
                            .description("The mock Codex state changed in memory only.")
                            .kind(ToastKind::Success)
                            .duration(Duration::from_secs(4)),
                        Instant::now(),
                    );
                    cx.invalidate();
                    return;
                }
                let Some(action) = action.clone() else {
                    return;
                };
                this.pending_apply = Some(PendingApply::Tweak(CleanupPlan {
                    actions: vec![action],
                    excluded_review_findings: 0,
                    reclaimable_bytes: 0,
                }));
                cx.invalidate();
            },
        );

        let (status, color) = match report.status {
            TweakStatus::NeedsChange => ("Allowed", theme.secondary_text),
            TweakStatus::Satisfied => ("Blocked", theme.success),
            TweakStatus::Blocked => ("Needs attention", theme.danger),
        };

        let control = setting_switch(
            enabled,
            "Block the Codex Pet keyboard shortcut",
            change,
            !can_change || self.busy(),
            theme,
        );
        let path = report.path.display().to_string();
        let restart = if report.restart_required {
            "Restart Codex after changing"
        } else {
            "No pending restart"
        };

        div()
            .id("codex-pet-setting")
            .flex_grow(1.0)
            .min_w(0.0)
            .h_full()
            .flex_col()
            .child(
                div()
                    .p_5()
                    .flex()
                    .items_start()
                    .justify_between()
                    .gap_5()
                    .child(
                        div()
                            .flex()
                            .items_start()
                            .gap_3()
                            .min_w(0.0)
                            .flex_grow(1.0)
                            .child(
                                div()
                                    .w(42.0)
                                    .h(42.0)
                                    .rounded(12.0)
                                    .border(1.0, theme.separator)
                                    .bg(theme.sidebar)
                                    .text_color(theme.text)
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(animated_keyboard_icon(21.0, enabled, theme.text)),
                            )
                            .child(
                                div()
                                    .flex_col()
                                    .gap_1()
                                    .min_w(0.0)
                                    .child(
                                        text("Pet keyboard shortcut")
                                            .text_size(16.0)
                                            .font_semibold(),
                                    )
                                    .child(
                                        text(report.description.as_str())
                                            .text_sm()
                                            .text_color(theme.secondary_text),
                                    )
                                    .child(text(report.detail.as_str()).text_size(12.0).text_color(color)),
                            ),
                    )
                    .child(
                        div()
                            .flex_col()
                            .items_end()
                            .gap_1()
                            .child(control)
                            .child(
                                text(if self.applying { "Updating…" } else { status })
                                    .text_size(11.0)
                                    .font_medium()
                                    .text_color(color),
                            ),
                    ),
            )
            .child(
                div()
                    .id("codex-pet-metadata")
                    .border_top(1.0, theme.separator)
                    .bg(theme.sidebar)
                    .p_5()
                    .flex_col()
                    .gap_5()
                    .child(
                        div()
                            .flex_col()
                            .gap_2()
                            .child(text("What this changes").text_size(13.0).font_semibold())
                            .child(
                                text("Only the Pet activation binding is changed. All other Codex keybindings and settings stay byte-for-byte untouched.")
                                    .text_size(13.0)
                                    .line_height(20.0)
                                    .text_color(theme.secondary_text),
                            ),
                    )
                    .child(
                        div()
                            .grid()
                            .grid_cols(2)
                            .gap_5()
                    .child(tweak_meta(
                        IconName::Power,
                        "Behavior",
                        "Only the Pet activation binding",
                        false,
                        theme,
                    ))
                    .child(tweak_meta(
                        IconName::FileBraces,
                        "Configuration",
                        &path,
                        true,
                        theme,
                    ))
                    .child(tweak_meta(
                        if report.restart_required {
                            IconName::RotateCcw
                        } else {
                            IconName::ShieldCheck
                        },
                        "Safety",
                        restart,
                        false,
                        theme,
                    )),
                    ),
            )
    }
}
