use super::*;

impl CleanerApp {
    pub(super) fn render_overview(&self, cx: &mut ViewContext<'_, Self>, theme: Theme) -> Element {
        let show_orca = cx.listener("overview-orca", |this, cx: &mut EventContext| {
            this.page = Page::Cleanup;
            cx.invalidate();
        });
        let show_codex = cx.listener("overview-codex", |this, cx: &mut EventContext| {
            this.page = Page::Tweaks;
            cx.invalidate();
        });
        let (findings, ready, reclaimable) = self.report.as_ref().map_or((0, 0, 0), |report| {
            let plan = self.cleanup_plan(report);
            (
                report.findings.len(),
                plan.actions.len(),
                plan.reclaimable_bytes,
            )
        });
        let configured = self
            .tweak_reports
            .iter()
            .filter(|report| report.status == TweakStatus::Satisfied)
            .count();
        let blocked = self
            .tweak_reports
            .iter()
            .filter(|report| report.status == TweakStatus::Blocked)
            .count();
        let scan_state = if self.scanning {
            "Scanning"
        } else {
            "Up to date"
        };
        let scan_color = if self.scanning {
            theme.warning
        } else {
            theme.success
        };
        let mut content = div()
            .id("overview-page")
            .w_full()
            .p_5()
            .flex_col()
            .gap_4()
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
                            .child(text("System overview").text_2xl().font_semibold())
                            .child(
                                text("One read-only scan across cleanup artifacts and preference recipes.")
                                    .text_sm()
                                    .text_color(theme.secondary_text),
                            ),
                    )
                    .child(status_badge(scan_state, scan_color, theme)),
            )
            .child(
                panel(theme)
                    .id("overview-summary")
                    .h(88.0)
                    .flex()
                    .overflow_hidden()
                    .child(overview_metric("Providers", "2".to_owned(), false, theme))
                    .child(overview_metric(
                        "Findings",
                        findings.to_string(),
                        true,
                        theme,
                    ))
                    .child(overview_metric("Ready", ready.to_string(), true, theme))
                    .child(overview_metric(
                        "Reclaimable",
                        human_bytes(reclaimable),
                        true,
                        theme,
                    ))
                    .child(overview_metric(
                        "Preferences",
                        format!("{configured}/{}", self.tweak_reports.len()),
                        true,
                        theme,
                    )),
            )
            .child(
                div()
                    .flex_col()
                    .gap_2()
                    .child(text("Providers").text_size(14.0).font_semibold())
                    .child(
                        text("Open a provider to review what the unified scan found and choose any action.")
                            .text_size(13.0)
                            .text_color(theme.secondary_text),
                    ),
            )
            .child(
                panel(theme)
                    .overflow_hidden()
                    .flex_col()
                    .child(
                        button()
                            .id("overview-orca")
                            .w_full()
                            .h(82.0)
                            .p_4()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_4()
                            .cursor_pointer()
                            .hover(|style| style.bg(theme.control_hover))
                            .on_click(show_orca)
                            .child(
                                div()
                                    .min_w(0.0)
                                    .flex()
                                    .items_center()
                                    .gap_3()
                                    .child(provider_icon(orca_icon(), 42.0, 12.0))
                                    .child(
                                        div()
                                            .min_w(0.0)
                                            .flex_col()
                                            .gap_1()
                                            .child(text("Orca").text_size(15.0).font_semibold())
                                            .child(
                                                text("16 cleanup artifact types across local, WSL and SSH environments")
                                                    .text_size(12.0)
                                                    .text_color(theme.secondary_text)
                                                    .truncate(),
                                            ),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_3()
                                    .child(status_badge(
                                        if self.scanning { "Scanning" } else { "Checked" },
                                        scan_color,
                                        theme,
                                    ))
                                    .child(
                                        text(format!("{findings} found · {ready} ready"))
                                            .text_size(12.0)
                                            .text_color(theme.secondary_text),
                                    )
                                    .child(text("›").text_xl().text_color(theme.secondary_text)),
                            ),
                    )
                    .child(
                        button()
                            .id("overview-codex")
                            .w_full()
                            .h(82.0)
                            .p_4()
                            .border_top(1.0, theme.separator)
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_4()
                            .cursor_pointer()
                            .hover(|style| style.bg(theme.control_hover))
                            .on_click(show_codex)
                            .child(
                                div()
                                    .min_w(0.0)
                                    .flex()
                                    .items_center()
                                    .gap_3()
                                    .child(
                                        div()
                                            .w(42.0)
                                            .h(42.0)
                                            .rounded(12.0)
                                            .border(1.0, theme.separator)
                                            .bg(theme.sidebar)
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .child(chatgpt_icon(31.0, theme.dark)),
                                    )
                                    .child(
                                        div()
                                            .min_w(0.0)
                                            .flex_col()
                                            .gap_1()
                                            .child(text("Codex").text_size(15.0).font_semibold())
                                            .child(
                                                text("Explicit, reversible preference recipes that are never auto-applied")
                                                    .text_size(12.0)
                                                    .text_color(theme.secondary_text)
                                                    .truncate(),
                                            ),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_3()
                                    .child(status_badge(
                                        if blocked > 0 { "Needs attention" } else { "Checked" },
                                        if blocked > 0 { theme.danger } else { scan_color },
                                        theme,
                                    ))
                                    .child(
                                        text(format!(
                                            "{} settings · {configured} configured",
                                            self.tweak_reports.len()
                                        ))
                                        .text_size(12.0)
                                        .text_color(theme.secondary_text),
                                    )
                                    .child(text("›").text_xl().text_color(theme.secondary_text)),
                            ),
                    ),
            )
            .child(
                div()
                    .p_3()
                    .rounded(12.0)
                    .bg(theme.sidebar)
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(icon(IconName::ShieldCheck, 16.0, theme.secondary_text))
                    .child(
                        text("Scan All only reads state. Cleanup and preference changes remain separate, reviewed actions.")
                            .text_size(12.0)
                            .text_color(theme.secondary_text),
                    ),
            );
        if let Some(error) = &self.error {
            content = content.child(message_panel(
                "The unified scan could not finish",
                error,
                theme.danger,
                theme,
            ));
        }
        content
    }
}
