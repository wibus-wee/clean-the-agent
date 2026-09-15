use super::*;

impl CleanerApp {
    pub(super) fn render_toasts(&self, cx: &mut ViewContext<'_, Self>, theme: Theme) -> Element {
        if self.toasts.is_empty() {
            return div();
        }
        let viewport = ToastViewport::new("app-toasts");
        let mut stack = viewport.viewport_with(div()).w(380.0).flex_col().gap_2();

        for entry in self.toasts.entries().iter().rev() {
            let parts = viewport.toast(entry);
            let toast_id = parts.id();
            let close = cx.listener(parts.close_id(), move |view, cx: &mut EventContext| {
                if view.toasts.dismiss(toast_id) {
                    view.exiting_toasts.remove(&toast_id);
                    view.toast_deadline = None;
                    cx.invalidate();
                }
            });
            let pause = cx.hover_listener(
                parts.root_id(),
                move |view: &mut Self, hovered, cx: &mut EventContext| {
                    let changed = if *hovered {
                        view.toasts.pause(toast_id, Instant::now())
                    } else {
                        view.toasts.resume(toast_id, Instant::now())
                    };
                    if changed {
                        view.toast_deadline = None;
                        cx.invalidate();
                    }
                },
            );
            let (glyph, color) = match parts.kind() {
                ToastKind::Success => (IconName::CircleCheck, theme.success),
                ToastKind::Warning => (IconName::Info, theme.warning),
                ToastKind::Error => (IconName::CircleAlert, theme.danger),
                ToastKind::Loading => (IconName::LoaderCircle, theme.secondary_text),
                ToastKind::Info => (IconName::Info, theme.secondary_text),
            };
            let mut copy = div().min_w(0.0).flex_grow(1.0).flex_col().gap_1().child(
                parts.title_with(
                    text(entry.toast().title().to_string())
                        .text_size(13.0)
                        .font_semibold(),
                ),
            );
            if let Some(description) = entry.toast().description_text() {
                copy = copy.child(
                    parts.description_with(
                        text(description.to_string())
                            .text_size(12.0)
                            .line_height(17.0)
                            .text_color(theme.secondary_text),
                    ),
                );
            }
            let close_button = parts.close_with(
                button()
                    .w(24.0)
                    .h(24.0)
                    .flex_none()
                    .rounded(7.0)
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(17.0)
                    .text_color(theme.secondary_text)
                    .opacity(0.55)
                    .group_hover(|style| style.opacity(1.0))
                    .hover(|style| style.bg(theme.control_hover).text_color(theme.text))
                    .on_click(close)
                    .child("×"),
            );
            let glyph = icon(glyph, 19.0, color);
            let glyph = match parts.kind() {
                ToastKind::Loading => glyph.with_animation(
                    format!("toast-spinner-{}", toast_id.value()),
                    Animation::new(Duration::from_millis(900)).repeat(),
                    |glyph, phase| glyph.rotate_degrees(phase * 360.0),
                ),
                ToastKind::Success => glyph.with_animation(
                    format!("toast-success-{}", toast_id.value()),
                    Animation::new(Duration::from_millis(240))
                        .with_easing(quickgui::ease_out_quint()),
                    |glyph, phase| glyph.opacity(phase).translate(0.0, 4.0 * (1.0 - phase)),
                ),
                _ => glyph,
            };
            let root = parts
                .root_with(
                    div()
                        .group()
                        .w_full()
                        .min_h(64.0)
                        .p_3()
                        .rounded(14.0)
                        .border(1.0, theme.separator)
                        .bg(theme.control)
                        .shadow_md()
                        .flex()
                        .items_center()
                        .gap_3()
                        .on_hover(pause),
                )
                .child(
                    div()
                        .w(20.0)
                        .h(20.0)
                        .flex_none()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(glyph),
                )
                .child(parts.content_with(copy))
                .child(close_button);
            let root = parts.key_with(cx, root, Self::toast_manager);
            let entering = div().w_full().child(root).with_animation(
                format!("toast-enter-{}", toast_id.value()),
                Animation::new(Duration::from_millis(320)).with_easing(quickgui::ease_in_out),
                |toast, phase| toast.opacity(phase).translate(32.0 * (1.0 - phase), 0.0),
            );
            let exiting = self.exiting_toasts.contains(&toast_id);
            let positioner = div()
                .w_full()
                .opacity(if exiting { 0.0 } else { 1.0 })
                .translate(if exiting { 24.0 } else { 0.0 }, 0.0)
                .transition(
                    Transition::new(Duration::from_millis(200))
                        .with_properties(
                            TransitionProperties::OPACITY | TransitionProperties::TRANSFORM,
                        )
                        .with_easing(quickgui::ease_out_quint()),
                )
                .child(entering);
            stack = stack.child(parts.positioner_with(positioner));
        }

        viewport.portal_with(div().absolute().right(20.0).bottom(20.0).child(stack))
    }

    pub(super) fn render_sidebar(&self, cx: &mut ViewContext<'_, Self>, theme: Theme) -> Element {
        let orca = orca_icon();
        let show_overview = cx.listener("show-overview", |this, cx: &mut EventContext| {
            this.page = Page::Overview;
            cx.invalidate();
        });
        let show_cleanup = cx.listener("show-cleanup", |this, cx: &mut EventContext| {
            this.page = Page::Cleanup;
            cx.invalidate();
        });
        let show_tweaks = cx.listener("show-tweaks", |this, cx: &mut EventContext| {
            this.page = Page::Tweaks;
            cx.invalidate();
        });
        let show_about = cx.listener("show-about", |this, cx: &mut EventContext| {
            this.page = Page::About;
            cx.invalidate();
        });
        #[cfg(debug_assertions)]
        let show_ui_lab = cx.listener("show-ui-lab", |this, cx: &mut EventContext| {
            this.scanning = false;
            this.applying = false;
            this.page = Page::UiLab;
            cx.invalidate();
        });
        let about_icon = system_info_icon(18.0, theme.text, theme.dark);
        let footer = div().px_2().py_2().flex_col().gap_1();
        #[cfg(debug_assertions)]
        let footer = footer.child(sidebar_icon_row(
            "UI Lab · DEBUG",
            icon(IconName::FileBraces, 18.0, theme.text),
            self.page == Page::UiLab,
            show_ui_lab,
            theme,
        ));
        let footer = footer.child(sidebar_icon_row(
            "About Clean the Agent",
            about_icon,
            self.page == Page::About,
            show_about,
            theme,
        ));

        sidebar()
            .bg(theme.sidebar)
            .border_right(1.0, theme.separator)
            .child(
                div()
                    .flex_grow(1.0)
                    .min_h(0.0)
                    .overflow_hidden()
                    .flex_col()
                    .gap_1()
                    .padding(crate::design::TOOLBAR_HEIGHT + 8.0, 12.0, 0.0, 12.0)
                    .child(sidebar_icon_row(
                        "Overview",
                        icon(IconName::LayoutDashboard, 18.0, theme.text),
                        self.page == Page::Overview,
                        show_overview,
                        theme,
                    ))
                    .child(div().h(10.0))
                    .child(
                        text("PROVIDERS")
                            .px_3()
                            .py_2()
                            .text_size(11.0)
                            .font_semibold()
                            .text_color(theme.secondary_text),
                    )
                    .child(provider_sidebar_row(
                        "Orca",
                        orca,
                        self.page == Page::Cleanup,
                        show_cleanup,
                        theme,
                    ))
                    .child(sidebar_icon_row(
                        "Codex",
                        chatgpt_icon(24.0, theme.dark),
                        self.page == Page::Tweaks,
                        show_tweaks,
                        theme,
                    )),
            )
            .child(footer)
    }

    pub(super) fn render_toolbar(&self, cx: &mut ViewContext<'_, Self>, theme: Theme) -> Element {
        let scan = cx.listener("scan", |this, cx: &mut EventContext| {
            this.schedule_scan();
            cx.invalidate();
        });

        let title = match self.page {
            Page::Overview => "Overview",
            Page::Cleanup => "Orca",
            Page::Tweaks => "Codex",
            #[cfg(debug_assertions)]
            Page::UiLab => "UI Lab",
            Page::About => "About",
        };

        toolbar()
            .justify_between()
            .gap_4()
            .border_bottom(1.0, theme.separator)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(text(title).text_sm().font_semibold())
                    .when(self.preview_mode, |title| {
                        title.child(status_badge("UI preview", theme.warning, theme))
                    }),
            )
            .when(
                matches!(self.page, Page::Overview | Page::Cleanup),
                |toolbar| {
                    toolbar.child(div().flex().items_center().app_region_no_drag().child(
                        primary_button(
                            if self.preview_mode {
                                "Run Real Scan"
                            } else if self.scanning {
                                "Scanning…"
                            } else if self.page == Page::Overview {
                                "Scan All"
                            } else {
                                "Rescan"
                            },
                            scan,
                            self.busy() && !self.preview_mode,
                            theme,
                        ),
                    ))
                },
            )
    }
}
