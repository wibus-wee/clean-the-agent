use super::*;

impl CleanerApp {
    pub(super) fn render_about(&self, theme: Theme) -> Element {
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
                    .gap_4()
                    .child(
                        div()
                            .id("about-hero")
                            .flex_none()
                            .flex_col()
                            .items_center()
                            .text_center()
                            .gap_2()
                            .child(
                                provider_icon(app_icon(), 84.0, 21.0).id("about-app-icon"),
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
                            .flex_none()
                            .flex()
                            .items_center()
                            .justify_center()
                            .gap_2()
                            .text_size(11.0)
                            .text_color(theme.secondary_text)
                            .child(text("dev.cleantheagent.app").font_family("Geist Mono"))
                            .child("·")
                            .child("QuickGUI 0.1.4 · Rust"),
                    ),
            )
    }
}
