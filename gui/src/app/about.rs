use super::*;

impl CleanerApp {
    pub(super) fn render_about(&self, cx: &mut ViewContext<'_, Self>, theme: Theme) -> Element {
        let available = self.update_status.available().cloned();
        let update_action = cx.listener("update-action", move |this, cx: &mut EventContext| {
            let Some(update) = available.clone() else {
                this.schedule_update_check();
                cx.invalidate();
                return;
            };
            let options = MessageBoxOptions::new(format!("Install version {}?", update.version))
                .level(PromptLevel::Info)
                .detail(
                    "The signed update will replace this application and restart it. If the app is running from the disk image, move it to Applications first.",
                )
                .buttons([
                    PromptButton::cancel("Cancel"),
                    PromptButton::ok("Install and Restart"),
                ])
                .default_button(1)
                .cancel_button(0);
            match cx.message_box(options) {
                Ok(response) => match cx.spawn(
                    |task_cx: AsyncViewContext<CleanerApp>| async move {
                        if let Ok(answer) = response.await
                            && answer.button == 1
                        {
                            let _ = task_cx
                                .update(move |view, cx| {
                                    view.pending_update_install = Some(update);
                                    cx.invalidate();
                                })
                                .await;
                        }
                    },
                ) {
                    Ok(task) => task.detach(),
                    Err(error) => this.update_status = UpdateStatus::Failed(error.to_string()),
                },
                Err(error) => this.update_status = UpdateStatus::Failed(error.to_string()),
            }
            cx.invalidate();
        });

        div()
            .id("about-page")
            .size_full()
            .px_4()
            .flex_col()
            .justify_center()
            .child(
                div()
                    .id("about-content")
                    .w_full()
                    .max_w(620.0)
                    .mx_auto()
                    .flex_col()
                    .gap_3()
                    .child(
                        div()
                            .id("about-hero")
                            .flex_none()
                            .flex_col()
                            .items_center()
                            .text_center()
                            .gap_2()
                            .child(
                                provider_icon(app_icon(), 80.0, 20.0).id("about-app-icon"),
                            )
                            .child(text("Clean the Agent").text_2xl().font_semibold())
                            .child(
                                text("Clean up after developer agents without treating your state as garbage.")
                                    .max_w(520.0)
                                    .text_size(13.0)
                                    .line_height(19.0)
                                    .text_color(theme.secondary_text),
                            )
                            .child(
                                text(concat!(
                                    "Version ",
                                    env!("CARGO_PKG_VERSION"),
                                    "  ·  macOS 14+  ·  MIT"
                                ))
                                .text_size(11.0)
                                .font_medium()
                                .text_color(theme.secondary_text),
                            ),
                    )
                    .child(
                        div()
                            .id("about-coverage")
                            .w_full()
                            .flex_none()
                            .flex_col()
                            .border_top(1.0, theme.separator)
                            .border_bottom(1.0, theme.separator)
                            .child(about_provider_row(
                                provider_icon(orca_icon(), 30.0, 9.0),
                                "Orca",
                                "Cleanup provider",
                                "16 artifact types",
                                theme,
                            ))
                            .child(
                                about_provider_row(
                                    div()
                                        .w(30.0)
                                        .h(30.0)
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .child(chatgpt_icon(25.0, theme.dark)),
                                    "Codex",
                                    "Preference provider",
                                    "Explicitly reversible",
                                    theme,
                                )
                                .border_top(1.0, theme.separator),
                            ),
                    )
                    .child(
                        div()
                            .id("about-safety")
                            .w_full()
                            .flex_none()
                            .flex_col()
                            .child(
                                text("Careful by default")
                                    .mb(8.0)
                                    .text_size(13.0)
                                    .font_semibold(),
                            )
                            .child(
                                div()
                                    .border_top(1.0, theme.separator)
                                    .border_bottom(1.0, theme.separator)
                                    .flex_col()
                                    .child(about_principle(
                                        IconName::Info,
                                        "Read-only discovery",
                                        "Scanning only inspects state; it never changes anything.",
                                        false,
                                        theme,
                                    ))
                                    .child(about_principle(
                                        IconName::ShieldCheck,
                                        "Proven ownership",
                                        "Automatic cleanup requires exact provider evidence.",
                                        true,
                                        theme,
                                    ))
                                    .child(about_principle(
                                        IconName::FileBraces,
                                        "Preserved configuration",
                                        "Recipes leave unrelated settings byte-for-byte untouched.",
                                        true,
                                        theme,
                                    ))
                                    .child(about_principle(
                                        IconName::RotateCcw,
                                        "Change guards",
                                        "Apply fails closed if a target changed after scanning.",
                                        true,
                                        theme,
                                    )),
                            ),
                    )
                    .child(
                        div()
                            .id("about-application")
                            .w_full()
                            .h(48.0)
                            .flex_none()
                            .px_2()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_3()
                            .child(
                                div()
                                    .min_w(0.0)
                                    .flex_col()
                                    .child(
                                        text("dev.cleantheagent.app · QuickGUI 0.1.4 · Rust")
                                            .font_family("Geist Mono")
                                            .text_size(11.0)
                                            .text_color(theme.secondary_text),
                                    )
                                    .child(
                                        text(format!(
                                            "{} updates · {}",
                                            if updater::beta_channel() {
                                                "Beta"
                                            } else {
                                                "Stable"
                                            },
                                            updater::update_status_text(&self.update_status)
                                        ))
                                        .text_size(10.0)
                                        .text_color(theme.secondary_text)
                                        .truncate(),
                                    ),
                            )
                            .child(
                                button()
                                    .h(26.0)
                                    .px_2()
                                    .flex_none()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(7.0)
                                    .border(1.0, theme.separator)
                                    .bg(theme.control)
                                    .text_size(11.0)
                                    .font_medium()
                                    .text_color(theme.text)
                                    .app_region_no_drag()
                                    .disabled(self.update_status.busy())
                                    .when(self.update_status.busy(), |button| {
                                        button.opacity(0.55)
                                    })
                                    .when(!self.update_status.busy(), |button| {
                                        button.cursor_pointer().hover(|style| {
                                            style.bg(theme.control_hover)
                                        })
                                    })
                                    .on_click(update_action)
                                    .child(updater::update_action_label(&self.update_status)),
                            ),
                    ),
            )
    }
}
