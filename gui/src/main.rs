mod design;
mod icons;

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use clean_any::model::{ApplyStatus, ArtifactKind, CleanupPlan};
use clean_any::providers::OrcaProvider;
use clean_any::tweaks::DisableCodexPetShortcut;
use clean_any::{
    ApplyReport, Engine, Finding, Safety, ScanContext, ScanReport, TweakEngine, TweakReport,
    TweakStatus,
};
use design::{
    Theme, category_tile, destructive_button, detail_group, empty_state, message_panel, panel,
    primary_button, provider_icon, provider_sidebar_row, review_checkbox, setting_switch, sidebar,
    sidebar_icon_row, status_badge, toolbar, toolbar_button,
};
use icons::{IconName, animated_keyboard_icon, chatgpt_icon, icon, system_info_icon};
use quickgui::{
    Animation, AnimationExt, App, AppInfo, Application, AsyncViewContext, ClickListener, Element,
    EventContext, IntoElement, MessageBoxOptions, PathPromptOptions, PromptButton, PromptLevel,
    TitleBarStyle, Toast, ToastId, ToastKind, ToastManager, ToastViewport, Transition,
    TransitionProperties, View, ViewContext, WindowOptions, button, div, text,
};

fn main() -> Result<(), quickgui::AppError> {
    Application::new()
        .app_info(
            AppInfo::new(
                "Clean the Agent",
                env!("CARGO_PKG_VERSION"),
                "dev.cleantheagent.app",
            )
            .expect("valid application identity"),
        )
        .on_reopen(|has_visible_windows, cx| {
            if !has_visible_windows {
                open_window(cx);
            }
        })
        .run(open_window)
}

fn open_window(cx: &mut App) {
    cx.open_window(
        WindowOptions::new("Clean the Agent")
            .size(1120.0, 740.0)
            .minimum_size(920.0, 620.0)
            .title_bar_style(TitleBarStyle::HiddenInset)
            .traffic_light_position(18.0, 22.0),
        CleanerApp::new(),
    );
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum Page {
    #[default]
    Overview,
    Cleanup,
    Tweaks,
    About,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CleanupCategory {
    AppData,
    Workspaces,
    Integrations,
    AgentRuntime,
}

impl CleanupCategory {
    const ALL: [Self; 4] = [
        Self::AppData,
        Self::Workspaces,
        Self::Integrations,
        Self::AgentRuntime,
    ];

    const fn index(self) -> usize {
        match self {
            Self::AppData => 0,
            Self::Workspaces => 1,
            Self::Integrations => 2,
            Self::AgentRuntime => 3,
        }
    }

    const fn title(self) -> &'static str {
        match self {
            Self::AppData => "App Data",
            Self::Workspaces => "Workspaces",
            Self::Integrations => "Integrations",
            Self::AgentRuntime => "Agent Runtime",
        }
    }

    const fn detail(self) -> &'static str {
        match self {
            Self::AppData => "Caches, logs, terminal history, backups and owned data.",
            Self::Workspaces => "Worktrees, worktree trash and stale workspace metadata.",
            Self::Integrations => "Managed hooks, config changes, trust entries and plugins.",
            Self::AgentRuntime => "Local, WSL and SSH runtime state, including installed skills.",
        }
    }

    const fn supported_types(self) -> usize {
        match self {
            Self::AppData => 5,
            Self::Workspaces => 3,
            Self::Integrations => 4,
            Self::AgentRuntime => 3,
        }
    }

    const fn contains(self, kind: ArtifactKind) -> bool {
        match self {
            Self::AppData => matches!(
                kind,
                ArtifactKind::OwnedData
                    | ArtifactKind::Cache
                    | ArtifactKind::Log
                    | ArtifactKind::UserHistory
                    | ArtifactKind::Backup
            ),
            Self::Workspaces => matches!(
                kind,
                ArtifactKind::Worktree | ArtifactKind::WorktreeTrash | ArtifactKind::OrphanedState
            ),
            Self::Integrations => matches!(
                kind,
                ArtifactKind::ManagedHook
                    | ArtifactKind::ConfigMutation
                    | ArtifactKind::TrustEntry
                    | ArtifactKind::Plugin
            ),
            Self::AgentRuntime => matches!(
                kind,
                ArtifactKind::RuntimeState
                    | ArtifactKind::SkillPlacement
                    | ArtifactKind::RemoteRuntime
            ),
        }
    }
}

enum PendingApply {
    Cleanup(CleanupPlan),
    Tweak(CleanupPlan),
}

struct CleanerApp {
    page: Page,
    selected_home: Option<PathBuf>,
    display_home: PathBuf,
    include_review: bool,
    included_categories: [bool; 4],
    report: Option<ScanReport>,
    selected_finding: Option<usize>,
    selected_tweak: Option<usize>,
    tweak_reports: Vec<TweakReport>,
    error: Option<String>,
    last_apply: Option<(String, ApplyReport)>,
    pending_scan: bool,
    scanning: bool,
    pending_apply: Option<PendingApply>,
    applying: bool,
    toasts: ToastManager,
    exiting_toasts: HashSet<ToastId>,
    toast_deadline: Option<Instant>,
}

impl CleanerApp {
    fn new() -> Self {
        let default_context = build_context(None);
        let display_home = default_context
            .as_ref()
            .map_or_else(|_| PathBuf::from("Home"), |context| context.home.clone());
        let tweak_reports = default_context
            .as_ref()
            .map_or_else(|_| Vec::new(), inspect_tweaks);

        Self {
            page: Page::Overview,
            selected_home: None,
            display_home,
            include_review: false,
            included_categories: [true; 4],
            report: None,
            selected_finding: None,
            selected_tweak: (!tweak_reports.is_empty()).then_some(0),
            tweak_reports,
            error: None,
            last_apply: None,
            pending_scan: true,
            scanning: false,
            pending_apply: None,
            applying: false,
            toasts: ToastManager::new().limit(3),
            exiting_toasts: HashSet::new(),
            toast_deadline: None,
        }
    }

    fn busy(&self) -> bool {
        self.scanning || self.applying
    }

    fn schedule_scan(&mut self) {
        if !self.busy() {
            self.error = None;
            self.last_apply = None;
            self.pending_scan = true;
        }
    }

    fn refresh_tweaks(&mut self) -> Result<(), String> {
        let context = build_context(self.selected_home.as_deref())?;
        let tweaks = inspect_tweaks(&context);
        self.selected_tweak = (!tweaks.is_empty()).then_some(
            self.selected_tweak
                .unwrap_or(0)
                .min(tweaks.len().saturating_sub(1)),
        );
        self.tweak_reports = tweaks;
        Ok(())
    }

    fn category_is_included(&self, category: CleanupCategory) -> bool {
        self.included_categories[category.index()]
    }

    fn cleanup_plan(&self, report: &ScanReport) -> CleanupPlan {
        let scoped_report = ScanReport {
            findings: report
                .findings
                .iter()
                .filter(|finding| {
                    CleanupCategory::ALL
                        .iter()
                        .find(|category| category.contains(finding.kind))
                        .is_none_or(|category| self.category_is_included(*category))
                })
                .cloned()
                .collect(),
            warnings: Vec::new(),
        };
        Engine::plan(&scoped_report, self.include_review)
    }

    fn start_background_work(&mut self, cx: &ViewContext<'_, Self>) {
        if self.pending_scan && !self.busy() {
            self.pending_scan = false;
            self.scanning = true;
            let selected_home = self.selected_home.clone();
            if let Err(error) = cx.spawn_background(
                move || scan_home(selected_home.as_deref()),
                |view, result, cx| {
                    view.scanning = false;
                    match result {
                        Ok(Ok((display_home, report, tweaks))) => {
                            view.display_home = display_home;
                            view.selected_finding = (!report.findings.is_empty()).then_some(0);
                            view.selected_tweak = (!tweaks.is_empty()).then_some(
                                view.selected_tweak
                                    .unwrap_or(0)
                                    .min(tweaks.len().saturating_sub(1)),
                            );
                            view.report = Some(report);
                            view.tweak_reports = tweaks;
                            view.error = None;
                        }
                        Ok(Err(error)) => view.error = Some(error),
                        Err(error) => view.error = Some(error.to_string()),
                    }
                    cx.invalidate();
                },
            ) {
                self.scanning = false;
                self.error = Some(error.to_string());
            }
        }

        if let Some(operation) = self.pending_apply.take() {
            self.applying = true;
            self.error = None;
            self.last_apply = None;
            let label = match &operation {
                PendingApply::Cleanup(_) => "Cleanup",
                PendingApply::Tweak(_) => "Tweak",
            }
            .to_owned();
            let plan = match operation {
                PendingApply::Cleanup(plan) | PendingApply::Tweak(plan) => plan,
            };
            let toast_id = self.toasts.promise(
                if label == "Tweak" {
                    "Updating Codex preference…"
                } else {
                    "Cleaning selected items…"
                },
                Instant::now(),
            );
            if let Err(error) = cx.spawn_background(
                move || Engine::apply(&plan),
                move |view, result, cx| {
                    view.applying = false;
                    match result {
                        Ok(report) => {
                            let counts = apply_counts(&report);
                            let is_tweak = label == "Tweak";
                            view.toasts.resolve(
                                toast_id,
                                apply_toast(&label, counts),
                                Instant::now(),
                            );
                            view.last_apply = (counts.skipped > 0 || counts.failed > 0)
                                .then_some((label, report));
                            if is_tweak {
                                if let Err(error) = view.refresh_tweaks() {
                                    view.error = Some(error);
                                }
                            } else {
                                view.pending_scan = true;
                            }
                        }
                        Err(error) => {
                            let detail = error.to_string();
                            view.toasts.resolve(
                                toast_id,
                                Toast::new(if label == "Tweak" {
                                    "Codex preference could not be updated"
                                } else {
                                    "Cleanup could not finish"
                                })
                                .description(detail.clone())
                                .kind(ToastKind::Error)
                                .duration(Duration::from_secs(8)),
                                Instant::now(),
                            );
                            view.error = Some(detail);
                        }
                    }
                    cx.invalidate();
                },
            ) {
                self.applying = false;
                let detail = error.to_string();
                self.toasts.resolve(
                    toast_id,
                    Toast::new("The operation could not start")
                        .description(detail.clone())
                        .kind(ToastKind::Error)
                        .duration(Duration::from_secs(8)),
                    Instant::now(),
                );
                self.error = Some(detail);
            }
        }
    }

    fn toast_manager(view: &mut Self) -> &mut ToastManager {
        &mut view.toasts
    }

    fn schedule_toast_expiry(&mut self, cx: &ViewContext<'_, Self>) {
        let Some(deadline) = self
            .toasts
            .entries()
            .iter()
            .filter(|entry| !self.exiting_toasts.contains(&entry.id()))
            .filter_map(|entry| entry.deadline())
            .min()
        else {
            self.toast_deadline = None;
            return;
        };
        if self.toast_deadline == Some(deadline) {
            return;
        }
        self.toast_deadline = Some(deadline);
        match cx.spawn(move |task_cx: AsyncViewContext<CleanerApp>| async move {
            let _ = task_cx.sleep_until(deadline).await;
            let expired = task_cx
                .update(move |view, cx| {
                    if view.toast_deadline != Some(deadline) {
                        return Vec::new();
                    }
                    view.toast_deadline = None;
                    let expired = view
                        .toasts
                        .entries()
                        .iter()
                        .filter(|entry| entry.deadline().is_some_and(|due| due <= deadline))
                        .map(|entry| entry.id())
                        .collect::<Vec<_>>();
                    if !expired.is_empty() {
                        view.exiting_toasts.extend(expired.iter().copied());
                        cx.invalidate();
                    }
                    expired
                })
                .await
                .unwrap_or_default();
            if expired.is_empty() {
                return;
            }
            let _ = task_cx
                .sleep_until(deadline + Duration::from_millis(200))
                .await;
            let _ = task_cx
                .update(move |view, cx| {
                    for id in expired {
                        view.exiting_toasts.remove(&id);
                        view.toasts.dismiss(id);
                    }
                    cx.invalidate();
                })
                .await;
        }) {
            Ok(task) => task.detach(),
            Err(_) => self.toast_deadline = None,
        }
    }
}

impl View for CleanerApp {
    fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
        self.start_background_work(cx);
        self.schedule_toast_expiry(cx);
        let theme = Theme::from_context(cx);
        let scroll_page = |id: &'static str, content: Element| {
            div()
                .id(id)
                .w_full()
                .flex_grow(1.0)
                .min_h(0.0)
                .overflow_y_scroll()
                .child(content)
        };
        let page_content = match self.page {
            Page::Overview => scroll_page("page-scroll-overview", self.render_overview(cx, theme)),
            Page::Cleanup => scroll_page("page-scroll-cleanup", self.render_cleanup(cx, theme)),
            Page::Tweaks => scroll_page("page-scroll-tweaks", self.render_tweaks(cx, theme)),
            Page::About => div()
                .id("page-static-about")
                .w_full()
                .flex_grow(1.0)
                .min_h(0.0)
                .overflow_hidden()
                .child(self.render_about(theme)),
        };

        div()
            .size_full()
            .min_h(0.0)
            .overflow_hidden()
            .flex()
            .font_family("Geist")
            .bg(theme.window)
            .text_color(theme.text)
            .child(self.render_sidebar(cx, theme))
            .child(
                div()
                    .flex_grow(1.0)
                    .min_w(0.0)
                    .min_h(0.0)
                    .h_full()
                    .overflow_hidden()
                    .flex_col()
                    .child(self.render_toolbar(cx, theme))
                    .child(page_content),
            )
            .child(self.render_toasts(cx, theme))
    }
}

impl CleanerApp {
    fn render_toasts(&self, cx: &mut ViewContext<'_, Self>, theme: Theme) -> Element {
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

    fn render_sidebar(&self, cx: &mut ViewContext<'_, Self>, theme: Theme) -> Element {
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
        let about_icon = system_info_icon(18.0, theme.text, theme.dark);

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
                    .padding(design::TOOLBAR_HEIGHT + 8.0, 12.0, 0.0, 12.0)
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
            .child(div().px_2().py_2().child(sidebar_icon_row(
                "About Clean the Agent",
                about_icon,
                self.page == Page::About,
                show_about,
                theme,
            )))
    }

    fn render_toolbar(&self, cx: &mut ViewContext<'_, Self>, theme: Theme) -> Element {
        let scan = cx.listener("scan", |this, cx: &mut EventContext| {
            this.schedule_scan();
            cx.invalidate();
        });

        let title = match self.page {
            Page::Overview => "Overview",
            Page::Cleanup => "Orca",
            Page::Tweaks => "Codex",
            Page::About => "About",
        };

        toolbar()
            .justify_between()
            .gap_4()
            .border_bottom(1.0, theme.separator)
            .child(text(title).text_sm().font_semibold())
            .when(
                matches!(self.page, Page::Overview | Page::Cleanup),
                |toolbar| {
                    toolbar.child(div().flex().items_center().app_region_no_drag().child(
                        primary_button(
                            if self.scanning {
                                "Scanning…"
                            } else if self.page == Page::Overview {
                                "Scan All"
                            } else {
                                "Rescan"
                            },
                            scan,
                            self.busy(),
                            theme,
                        ),
                    ))
                },
            )
    }

    fn render_overview(&self, cx: &mut ViewContext<'_, Self>, theme: Theme) -> Element {
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
                                                text("15 cleanup artifact types across local, WSL and SSH environments")
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

    fn render_cleanup(&self, cx: &mut ViewContext<'_, Self>, theme: Theme) -> Element {
        let toggle_review = cx.listener("toggle-review", |this, cx: &mut EventContext| {
            if !this.busy() {
                this.include_review = !this.include_review;
                cx.invalidate();
            }
        });
        let choose_home = cx.listener("choose-home", |this, cx: &mut EventContext| {
            let options = PathPromptOptions::new()
                .files(false)
                .directories(true)
                .can_create_directories(false)
                .title("Choose Another Home Folder")
                .prompt("Scan This Home")
                .directory(&this.display_home)
                .message(
                    "Use this when Orca artifacts belong to another macOS or mounted home folder. The scan is read-only.",
                );
            match cx.prompt_for_paths(options) {
                Ok(response) => match cx.spawn(|task_cx: AsyncViewContext<CleanerApp>| async move {
                    if let Ok(Some(paths)) = response.await
                        && let Some(home) = paths.into_iter().next()
                    {
                        let _ = task_cx
                            .update(move |view, cx| {
                                view.selected_home = Some(home);
                                view.schedule_scan();
                                cx.invalidate();
                            })
                            .await;
                    }
                }) {
                    Ok(task) => task.detach(),
                    Err(error) => this.error = Some(error.to_string()),
                },
                Err(error) => this.error = Some(error.to_string()),
            }
            cx.invalidate();
        });

        let mut content = div().w_full().p_5().flex_col().gap_4();
        content = content.child(self.render_category_selector(cx, theme));
        content = content.child(status_strip(self, choose_home, theme));

        if let Some(error) = &self.error {
            content = content.child(message_panel(
                "The operation could not finish",
                error,
                theme.danger,
                theme,
            ));
        }
        if let Some((label, report)) = &self.last_apply {
            content = content.child(apply_result_panel(label, report, theme));
        }

        let Some(report) = &self.report else {
            return content.child(
                scan_outcome_panel(
                    if self.scanning {
                        "Scanning your Mac…"
                    } else {
                        "Ready when you are"
                    },
                    "Checking all selected categories is read-only. Nothing changes until you review and confirm a cleanup.",
                    false,
                    theme,
                )
                .id("empty-state-panel"),
            );
        };

        if !report.warnings.is_empty() {
            content = content.child(warnings_panel(&report.warnings, theme));
        }

        if report.findings.is_empty() {
            return content.child(
                scan_outcome_panel(
                    "You’re all clear",
                    "All 15 supported Orca artifact types were checked. No cleanable artifacts were found in this folder.",
                    true,
                    theme,
                )
                .id("empty-state-panel"),
            );
        }

        let plan = self.cleanup_plan(report);
        let plan_for_click = plan.clone();
        let clean = cx.listener("apply-cleanup", move |this, cx: &mut EventContext| {
            if this.busy() || plan_for_click.actions.is_empty() {
                return;
            }
            let action_count = plan_for_click.actions.len();
            let detail = if this.include_review {
                format!(
                    "This permanently applies {action_count} cleanup actions, including items that require review. This can’t be undone."
                )
            } else {
                format!(
                    "This permanently applies {action_count} verified cleanup actions. This can’t be undone."
                )
            };
            let options = MessageBoxOptions::new(format!(
                "Clean {action_count} {}?",
                if action_count == 1 { "Item" } else { "Items" }
            ))
            .level(PromptLevel::Warning)
            .detail(detail)
            .buttons([PromptButton::cancel("Cancel"), PromptButton::ok("Clean")])
            .default_button(1)
            .cancel_button(0);
            let plan = plan_for_click.clone();
            match cx.message_box(options) {
                Ok(response) => match cx.spawn(
                    |task_cx: AsyncViewContext<CleanerApp>| async move {
                        if let Ok(answer) = response.await
                            && answer.button == 1
                        {
                            let _ = task_cx
                                .update(move |view, cx| {
                                    view.pending_apply = Some(PendingApply::Cleanup(plan));
                                    cx.invalidate();
                                })
                                .await;
                        }
                    },
                ) {
                    Ok(task) => task.detach(),
                    Err(error) => this.error = Some(error.to_string()),
                },
                Err(error) => this.error = Some(error.to_string()),
            }
            cx.invalidate();
        });

        content
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_4()
                    .child(review_checkbox(
                        self.include_review,
                        toggle_review,
                        self.busy(),
                        theme,
                    ))
                    .child(destructive_button(
                        if self.applying {
                            "Cleaning…".to_owned()
                        } else if plan.actions.is_empty() {
                            "Nothing to Clean".to_owned()
                        } else {
                            format!("Clean {} Items…", plan.actions.len())
                        },
                        clean,
                        self.busy() || plan.actions.is_empty(),
                        theme,
                    )),
            )
            .child(
                div()
                    .h(468.0)
                    .min_h(360.0)
                    .flex()
                    .border(1.0, theme.separator)
                    .rounded(design::PANEL_RADIUS)
                    .overflow_hidden()
                    .child(self.render_finding_list(cx, report, theme))
                    .child(self.render_finding_detail(report, theme)),
            )
    }

    fn render_category_selector(&self, cx: &mut ViewContext<'_, Self>, theme: Theme) -> Element {
        let icon = orca_icon();
        let enabled = CleanupCategory::ALL
            .iter()
            .filter(|category| self.category_is_included(**category))
            .count();
        let (provider_status, provider_status_color) = if self.scanning {
            ("Scanning", theme.warning)
        } else if self.report.is_some() {
            ("Scanned", theme.success)
        } else {
            ("Ready", theme.secondary_text)
        };
        let cards = CleanupCategory::ALL.into_iter().map(|category| {
            let toggle = cx.listener(
                format!("toggle-category-{}", category.index()),
                move |this, cx: &mut EventContext| {
                    if !this.busy() {
                        let index = category.index();
                        this.included_categories[index] = !this.included_categories[index];
                        cx.invalidate();
                    }
                },
            );
            let meta = self.report.as_ref().map_or_else(
                || format!("{} artifact types", category.supported_types()),
                |report| {
                    let findings = report
                        .findings
                        .iter()
                        .filter(|finding| category.contains(finding.kind))
                        .collect::<Vec<_>>();
                    let bytes = findings
                        .iter()
                        .map(|finding| finding.reclaimable_bytes)
                        .sum();
                    format!(
                        "{} found · {} · {} types",
                        findings.len(),
                        human_bytes(bytes),
                        category.supported_types()
                    )
                },
            );
            category_tile(
                category.title(),
                category.detail(),
                meta,
                self.category_is_included(category),
                toggle,
                theme,
            )
        });

        panel(theme)
            .id("provider-orca")
            .w_full()
            .overflow_hidden()
            .flex_col()
            .child(
                div()
                    .p_4()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_4()
                    .child(
                        div()
                            .min_w(0.0)
                            .flex()
                            .items_center()
                            .gap_3()
                            .child(
                                provider_icon(icon, 56.0, 16.0).id("orca-provider-icon"),
                            )
                            .child(
                                div()
                                    .min_w(0.0)
                                    .flex_col()
                                    .gap_1()
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap_2()
                                            .child(text("Orca").text_size(18.0).font_semibold())
                                            .child(status_badge(
                                                provider_status,
                                                provider_status_color,
                                                theme,
                                            )),
                                    )
                                    .child(
                                        text("Agent development environment")
                                            .text_size(13.0)
                                            .text_color(theme.secondary_text),
                                    )
                                    .child(
                                        text("Local macOS · WSL homes · SSH mounts")
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
                                text(format!("{enabled} of 4 scopes enabled"))
                                    .text_size(13.0)
                                    .font_medium(),
                            )
                            .child(
                                text("15 artifact types")
                                    .text_size(12.0)
                                    .text_color(theme.secondary_text),
                            ),
                    ),
            )
            .child(
                div()
                    .p_4()
                    .border_top(1.0, theme.separator)
                    .bg(theme.window)
                    .flex_col()
                    .gap_3()
                    .child(
                        div()
                            .flex_col()
                            .gap_1()
                            .child(text("Cleanup Scope").text_size(14.0).font_semibold())
                            .child(
                                text("Choose which Orca capability domains may contribute actions to the cleanup plan.")
                                    .text_size(13.0)
                                    .text_color(theme.secondary_text),
                            ),
                    )
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .flex_wrap()
                            .gap_3()
                            .children(cards),
                    ),
            )
    }

    fn render_finding_list(
        &self,
        cx: &mut ViewContext<'_, Self>,
        report: &ScanReport,
        theme: Theme,
    ) -> Element {
        let rows = report.findings.iter().enumerate().map(|(index, finding)| {
            let select = cx.listener(
                format!("select-finding-{index}"),
                move |this, cx: &mut EventContext| {
                    this.selected_finding = Some(index);
                    cx.invalidate();
                },
            );
            finding_row(finding, self.selected_finding == Some(index), select, theme)
        });

        div()
            .w(390.0)
            .h_full()
            .min_w(300.0)
            .flex_col()
            .border_right(1.0, theme.separator)
            .child(
                div()
                    .h(36.0)
                    .px_3()
                    .flex()
                    .items_center()
                    .bg(theme.control)
                    .border_bottom(1.0, theme.separator)
                    .child(
                        text(format!("{} Findings", report.findings.len()))
                            .text_sm()
                            .font_semibold(),
                    ),
            )
            .child(
                div()
                    .flex_grow(1.0)
                    .min_h(0.0)
                    .overflow_y_scroll()
                    .when(report.findings.is_empty(), |list| {
                        list.child(empty_state(
                            "No supported artifacts found",
                            "Try another folder or scan again later.",
                            theme,
                        ))
                    })
                    .children(rows),
            )
    }

    fn render_finding_detail(&self, report: &ScanReport, theme: Theme) -> Element {
        let Some(finding) = self
            .selected_finding
            .and_then(|index| report.findings.get(index))
        else {
            return div().flex_grow(1.0).h_full().child(empty_state(
                "Select a finding",
                "Details and cleanup evidence appear here.",
                theme,
            ));
        };

        div()
            .flex_grow(1.0)
            .min_w(0.0)
            .h_full()
            .overflow_y_scroll()
            .p_5()
            .flex_col()
            .gap_4()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_3()
                    .child(
                        text(humanize_debug(&format!("{:?}", finding.kind)))
                            .text_xl()
                            .font_semibold(),
                    )
                    .child(safety_badge(finding.safety, theme)),
            )
            .child(text(finding.description.as_str()).line_height(22.0))
            .child(detail_group(
                "Location",
                &finding.path.display().to_string(),
                theme,
            ))
            .child(detail_group(
                "Why Clean the Agent Recognized It",
                &finding.evidence,
                theme,
            ))
            .child(detail_group(
                "Scope",
                &humanize_debug(&format!("{:?}", finding.scope)),
                theme,
            ))
            .when(finding.reclaimable_bytes > 0, |detail| {
                detail.child(detail_group(
                    "Estimated Size",
                    &human_bytes(finding.reclaimable_bytes),
                    theme,
                ))
            })
    }

    fn render_tweaks(&self, cx: &mut ViewContext<'_, Self>, theme: Theme) -> Element {
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

    fn render_about(&self, theme: Theme) -> Element {
        div()
            .id("about-page")
            .w_full()
            .padding(12.0, 16.0, 12.0, 16.0)
            .flex_col()
            .gap_4()
            .child(
                div()
                    .id("about-hero")
                    .flex()
                    .items_center()
                    .gap_4()
                    .child(
                        provider_icon(app_icon(), 88.0, 22.0).id("about-app-icon"),
                    )
                    .child(
                        div()
                            .min_w(0.0)
                            .flex_col()
                            .gap_2()
                            .child(text("Clean the Agent").text_3xl().font_semibold())
                            .child(
                                text("Clean up after developer agents without treating your state as garbage.")
                                .text_size(14.0)
                                .line_height(21.0)
                                .text_color(theme.secondary_text),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(status_badge(
                                        concat!("Version ", env!("CARGO_PKG_VERSION")),
                                        theme.secondary_text,
                                        theme,
                                    ))
                                    .child(status_badge(
                                        "2 providers",
                                        theme.secondary_text,
                                        theme,
                                    ))
                                    .child(status_badge(
                                        "MIT Licensed",
                                        theme.secondary_text,
                                        theme,
                                    )),
                            ),
                    ),
            )
            .child(
                div()
                    .id("about-columns")
                    .grid()
                    .grid_cols(2)
                    .gap_4()
                    .child(
                        div()
                            .id("about-left-column")
                            .min_w(0.0)
                            .flex_col()
                            .gap_4()
                            .child(
                                div()
                                    .flex_col()
                                    .gap_2()
                                    .child(text("Coverage").text_size(14.0).font_semibold())
                                    .child(
                                        panel(theme)
                                            .overflow_hidden()
                                            .flex_col()
                                            .child(about_provider_row(
                                                provider_icon(orca_icon(), 38.0, 11.0),
                                                "Orca cleanup",
                                                "15 artifact types",
                                                "Artifacts",
                                                theme,
                                            ))
                                            .child(
                                                about_provider_row(
                                                    div()
                                                        .w(38.0)
                                                        .h(38.0)
                                                        .rounded(11.0)
                                                        .border(1.0, theme.separator)
                                                        .bg(theme.sidebar)
                                                        .flex()
                                                        .items_center()
                                                        .justify_center()
                                                        .child(chatgpt_icon(30.0, theme.dark)),
                                                    "Codex preferences",
                                                    "Explicit, reversible recipes",
                                                    "Preferences",
                                                    theme,
                                                )
                                                .border_top(1.0, theme.separator),
                                            ),
                                    ),
                            )
                            .child(
                                div()
                                    .flex_col()
                                    .gap_2()
                                    .child(text("Application").text_size(14.0).font_semibold())
                                    .child(
                                        panel(theme)
                                            .overflow_hidden()
                                            .grid()
                                            .grid_cols(2)
                                            .child(about_info_cell(
                                                "Version",
                                                env!("CARGO_PKG_VERSION"),
                                                false,
                                                false,
                                                theme,
                                            ))
                                            .child(about_info_cell(
                                                "Bundle identifier",
                                                "dev.cleantheagent.app",
                                                true,
                                                false,
                                                theme,
                                            ))
                                            .child(about_info_cell(
                                                "Interface",
                                                "QuickGUI 0.1.4 · Rust",
                                                false,
                                                true,
                                                theme,
                                            ))
                                            .child(about_info_cell(
                                                "Requires",
                                                "macOS 14 or later",
                                                true,
                                                true,
                                                theme,
                                            ))
                                            .child(about_info_cell(
                                                "Appearance",
                                                "Follows macOS",
                                                false,
                                                true,
                                                theme,
                                            ))
                                            .child(about_info_cell(
                                                "License", "MIT", true, true, theme,
                                            )),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .id("about-right-column")
                            .min_w(0.0)
                            .flex_col()
                            .gap_2()
                            .child(text("Safety by design").text_size(14.0).font_semibold())
                            .child(
                                panel(theme)
                                    .overflow_hidden()
                                    .flex_col()
                                    .child(about_principle(
                                        IconName::Info,
                                        "Read-only discovery",
                                        "Scan All only inspects state; it never changes anything.",
                                        false,
                                        false,
                                        theme,
                                    ))
                                    .child(about_principle(
                                        IconName::ShieldCheck,
                                        "Proven ownership",
                                        "Automatic cleanup requires exact provider evidence.",
                                        false,
                                        true,
                                        theme,
                                    ))
                                    .child(about_principle(
                                        IconName::FileBraces,
                                        "Preserved configuration",
                                        "Recipes leave unrelated settings byte-for-byte untouched.",
                                        false,
                                        true,
                                        theme,
                                    ))
                                    .child(about_principle(
                                        IconName::RotateCcw,
                                        "Change guards",
                                        "Every apply fails closed if its target changed after scanning.",
                                        false,
                                        true,
                                        theme,
                                    )),
                            ),
                    ),
            )
    }
}

fn about_provider_row(
    glyph: Element,
    title: &str,
    detail: &str,
    kind: &str,
    theme: Theme,
) -> Element {
    div()
        .h(58.0)
        .px_4()
        .flex()
        .items_center()
        .justify_between()
        .gap_4()
        .child(
            div()
                .min_w(0.0)
                .flex()
                .items_center()
                .gap_3()
                .child(glyph)
                .child(
                    div()
                        .min_w(0.0)
                        .flex_col()
                        .gap_1()
                        .child(text(title).text_size(13.0).font_semibold())
                        .child(
                            text(detail)
                                .text_size(12.0)
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

fn about_principle(
    glyph: IconName,
    title: &str,
    detail: &str,
    left_border: bool,
    top_border: bool,
    theme: Theme,
) -> Element {
    div()
        .min_h(92.0)
        .p_4()
        .when(left_border, |item| item.border_left(1.0, theme.separator))
        .when(top_border, |item| item.border_top(1.0, theme.separator))
        .flex()
        .items_start()
        .gap_3()
        .child(
            div()
                .w(30.0)
                .h(30.0)
                .flex_none()
                .rounded(9.0)
                .bg(theme.sidebar)
                .flex()
                .items_center()
                .justify_center()
                .child(icon(glyph, 16.0, theme.secondary_text)),
        )
        .child(
            div()
                .min_w(0.0)
                .flex_col()
                .gap_1()
                .child(text(title).text_size(13.0).font_semibold())
                .child(
                    text(detail)
                        .text_size(12.0)
                        .line_height(17.0)
                        .text_color(theme.secondary_text),
                ),
        )
}

fn about_info_cell(
    label: &str,
    value: &str,
    left_border: bool,
    top_border: bool,
    theme: Theme,
) -> Element {
    div()
        .min_w(0.0)
        .h(60.0)
        .px_3()
        .when(left_border, |cell| cell.border_left(1.0, theme.separator))
        .when(top_border, |cell| cell.border_top(1.0, theme.separator))
        .flex_col()
        .justify_center()
        .gap_1()
        .child(text(label).text_size(11.0).text_color(theme.secondary_text))
        .child(text(value).text_size(12.0).font_medium().truncate())
}

fn tweak_meta(glyph: IconName, label: &str, value: &str, monospace: bool, theme: Theme) -> Element {
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

fn app_icon() -> quickgui::Image {
    static ICON: OnceLock<quickgui::Image> = OnceLock::new();
    ICON.get_or_init(|| {
        quickgui::Image::decode(include_bytes!("../resources/app/icon.png"))
            .expect("bundled application icon must be a valid PNG")
    })
    .clone()
}

fn orca_icon() -> quickgui::Image {
    static ICON: OnceLock<quickgui::Image> = OnceLock::new();
    ICON.get_or_init(|| {
        quickgui::Image::decode(include_bytes!("../resources/providers/orca.png"))
            .expect("bundled Orca provider icon must be a valid PNG")
    })
    .clone()
}

fn status_strip(app: &CleanerApp, choose_home: ClickListener<CleanerApp>, theme: Theme) -> Element {
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
                .h(60.0)
                .px_4()
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
                .h(86.0)
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

fn status_value(label: &str, value: String, separated: bool, theme: Theme) -> Element {
    div()
        .h_full()
        .min_w(0.0)
        .flex_grow(1.0)
        .flex_basis(0.0)
        .px_4()
        .flex_col()
        .justify_center()
        .gap_2()
        .when(separated, |item| item.border_left(1.0, theme.separator))
        .child(
            text(label)
                .text_size(12.0)
                .font_medium()
                .text_color(theme.secondary_text),
        )
        .child(text(value).text_size(18.0).font_semibold().truncate())
}

fn overview_metric(label: &str, value: String, separated: bool, theme: Theme) -> Element {
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

fn scan_outcome_panel(title: &str, detail: &str, success: bool, theme: Theme) -> Element {
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

fn finding_row<V>(
    finding: &Finding,
    selected: bool,
    listener: quickgui::ClickListener<V>,
    theme: Theme,
) -> Element {
    let title = humanize_debug(&format!("{:?}", finding.kind));
    button()
        .w_full()
        .min_h(64.0)
        .px_3()
        .py_2()
        .flex_col()
        .items_start()
        .gap_1()
        .border_bottom(1.0, theme.separator)
        .when(selected, |row| {
            row.bg(theme.selection).text_color(theme.selection_text)
        })
        .when(!selected, |row| row.hover(|style| style.bg(theme.control)))
        .cursor_pointer()
        .on_click(listener)
        .child(text(title).text_sm().font_medium())
        .child(
            text(finding.path.display().to_string())
                .text_xs()
                .font_family("Geist Mono")
                .text_color(if selected {
                    theme.selection_text
                } else {
                    theme.secondary_text
                })
                .truncate(),
        )
}

fn safety_badge(safety: Safety, theme: Theme) -> Element {
    let (label, color) = match safety {
        Safety::Automatic => ("Automatic", theme.success),
        Safety::ReviewRequired => ("Review Required", theme.warning),
        Safety::Informational => ("Information", theme.secondary_text),
    };
    status_badge(label, color, theme)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ApplyCounts {
    applied: usize,
    skipped: usize,
    failed: usize,
}

fn apply_counts(report: &ApplyReport) -> ApplyCounts {
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

fn apply_toast(label: &str, counts: ApplyCounts) -> Toast {
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

fn apply_result_panel(label: &str, report: &ApplyReport, theme: Theme) -> Element {
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

fn warnings_panel(warnings: &[String], theme: Theme) -> Element {
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

fn build_context(home: Option<&Path>) -> Result<ScanContext, String> {
    ScanContext::from_environment(home.map(Path::to_path_buf), Vec::new(), Vec::new())
        .map_err(|error| error.to_string())
}

fn scan_home(home: Option<&Path>) -> Result<(PathBuf, ScanReport, Vec<TweakReport>), String> {
    let context = build_context(home)?;
    let engine = Engine::new(vec![Box::new(OrcaProvider)]);
    let report = engine
        .scan(&context, &[])
        .map_err(|error| error.to_string())?;
    let tweaks = inspect_tweaks(&context);
    Ok((context.home.clone(), report, tweaks))
}

fn inspect_tweaks(context: &ScanContext) -> Vec<TweakReport> {
    let engine = TweakEngine::new(vec![Box::new(DisableCodexPetShortcut)]);
    engine
        .summaries()
        .into_iter()
        .filter_map(|summary| engine.inspect(summary.id, context).ok())
        .collect()
}

fn humanize_debug(value: &str) -> String {
    let mut output = String::with_capacity(value.len() + 4);
    for (index, character) in value.chars().enumerate() {
        if index > 0 && character.is_uppercase() {
            output.push(' ');
        }
        output.push(character);
    }
    output
}

fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} {}", UNITS[unit])
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ApplyCounts, ArtifactKind, CleanerApp, CleanupCategory, Page, ScanReport, TweakReport,
        TweakStatus, WindowOptions, apply_toast, human_bytes, humanize_debug,
    };
    use quickgui::{Application, ToastKind};

    #[test]
    fn successful_tweaks_use_transient_success_feedback() {
        let toast = apply_toast(
            "Tweak",
            ApplyCounts {
                applied: 1,
                skipped: 0,
                failed: 0,
            },
        );

        assert_eq!(toast.toast_kind(), ToastKind::Success);
        assert_eq!(toast.title().as_ref(), "Codex preference updated");
        assert_eq!(
            toast.description_text().map(AsRef::as_ref),
            Some("Applied 1 change. Restart Codex to use it.")
        );
        assert!(toast.duration_value().is_some());
    }

    #[test]
    fn formats_byte_counts_for_the_interface() {
        assert_eq!(human_bytes(0), "0 B");
        assert_eq!(human_bytes(1024), "1.0 KB");
        assert_eq!(human_bytes(5 * 1024 * 1024), "5.0 MB");
    }

    #[test]
    fn turns_debug_names_into_readable_labels() {
        assert_eq!(humanize_debug("ConfigMutation"), "Config Mutation");
        assert_eq!(humanize_debug("Wsl"), "Wsl");
    }

    #[test]
    fn cleanup_categories_cover_every_detected_orca_artifact_once() {
        let detected_kinds = [
            ArtifactKind::OwnedData,
            ArtifactKind::Cache,
            ArtifactKind::Log,
            ArtifactKind::RuntimeState,
            ArtifactKind::Worktree,
            ArtifactKind::WorktreeTrash,
            ArtifactKind::ManagedHook,
            ArtifactKind::ConfigMutation,
            ArtifactKind::OrphanedState,
            ArtifactKind::UserHistory,
            ArtifactKind::TrustEntry,
            ArtifactKind::Plugin,
            ArtifactKind::SkillPlacement,
            ArtifactKind::Backup,
            ArtifactKind::RemoteRuntime,
        ];

        assert_eq!(
            CleanupCategory::ALL
                .iter()
                .map(|category| category.supported_types())
                .sum::<usize>(),
            detected_kinds.len()
        );
        for kind in detected_kinds {
            assert_eq!(
                CleanupCategory::ALL
                    .iter()
                    .filter(|category| category.contains(kind))
                    .count(),
                1,
                "{kind:?} must have exactly one cleanup category"
            );
        }
    }

    #[test]
    fn excluding_a_category_removes_its_actions_from_the_plan() {
        use clean_any::model::{CleanupAction, CleanupActionKind, Ownership, ScopeKind};
        use clean_any::{Finding, Safety};
        use std::path::PathBuf;

        let path = PathBuf::from("/tmp/orca-cache");
        let action = CleanupAction {
            id: "cache-action".to_owned(),
            provider: "orca".to_owned(),
            description: "Orca cache".to_owned(),
            path: path.clone(),
            scope: ScopeKind::Local,
            safety: Safety::Automatic,
            reclaimable_bytes: 64,
            kind: CleanupActionKind::RemoveEmptyDirectory,
            depends_on: Vec::new(),
        };
        let report = ScanReport {
            findings: vec![Finding {
                id: "cache-finding".to_owned(),
                provider: "orca".to_owned(),
                kind: ArtifactKind::Cache,
                ownership: Ownership::ProviderOwned,
                safety: Safety::Automatic,
                path,
                scope: ScopeKind::Local,
                description: "Orca cache".to_owned(),
                evidence: "test fixture".to_owned(),
                reclaimable_bytes: 64,
                action: Some(action),
            }],
            warnings: Vec::new(),
        };
        let mut app = CleanerApp::new();

        assert_eq!(app.cleanup_plan(&report).actions.len(), 1);
        app.included_categories[CleanupCategory::AppData.index()] = false;
        assert!(app.cleanup_plan(&report).actions.is_empty());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn an_enabled_tweak_exposes_a_provider_setting_switch() {
        use clean_any::Safety;
        use clean_any::model::{CleanupAction, CleanupActionKind, ScopeKind};
        use std::path::PathBuf;

        let path = PathBuf::from("/tmp/keybindings.json");
        let mut app = CleanerApp::new();
        app.pending_scan = false;
        app.page = Page::Tweaks;
        app.tweak_reports = vec![TweakReport {
            id: "codex.disable-pet-shortcut".to_owned(),
            product: "Codex".to_owned(),
            title: "Disable the Pet keyboard shortcut".to_owned(),
            description: "Prevent keyboard activation".to_owned(),
            status: TweakStatus::Satisfied,
            path: path.clone(),
            detail: "The null Pet binding is active".to_owned(),
            restart_required: true,
            action: None,
            revert_action: Some(CleanupAction {
                id: "restore-pet".to_owned(),
                provider: "codex".to_owned(),
                description: "Restore Pet".to_owned(),
                path,
                scope: ScopeKind::Local,
                safety: Safety::ReviewRequired,
                reclaimable_bytes: 0,
                kind: CleanupActionKind::RemoveEmptyDirectory,
                depends_on: Vec::new(),
            }),
        }];
        let (mut context, view) = Application::new()
            .into_test_context(WindowOptions::default().size(1120.0, 740.0), app)
            .expect("visual test context");
        assert!(
            !context
                .contains_element(
                    view.window_handle(),
                    quickgui::ToastViewport::new("app-toasts").portal_id(),
                )
                .unwrap(),
            "an empty toast layer must not cover the draggable toolbar"
        );
        assert!(
            !context
                .contains_element(view.window_handle(), "scan")
                .unwrap(),
            "Codex preference changes must not rebuild a global scan control"
        );
        assert!(
            !context
                .contains_element(view.window_handle(), "choose-home")
                .unwrap(),
            "home-folder controls belong only to the Orca provider"
        );
        let mut visual = context
            .visual(view.window_handle())
            .expect("visual context");
        let control = visual
            .element_bounds("change-tweak-0")
            .expect("preference switch bounds");
        let provider = visual
            .element_bounds("provider-codex")
            .expect("Codex provider bounds");
        let setting = visual
            .element_bounds("codex-pet-setting")
            .expect("Codex setting bounds");
        let metadata = visual
            .element_bounds("codex-pet-metadata")
            .expect("Codex setting metadata bounds");
        let browser = visual
            .element_bounds("codex-settings-browser")
            .expect("Codex settings browser bounds");
        let selected_row = visual
            .element_bounds("select-tweak-0")
            .expect("selected Codex row bounds");

        assert_eq!(control.width, 44.0);
        assert_eq!(control.height, 26.0);
        assert_eq!(provider.x, 252.0);
        assert_eq!(provider.width, 848.0);
        assert_eq!(browser.x, provider.x);
        assert_eq!(browser.width, provider.width);
        assert_eq!(setting.x, browser.x + 302.0);
        assert_eq!(setting.width, browser.width - 304.0);
        assert_eq!(metadata.width, setting.width);
        assert!(selected_row.x >= browser.x + 1.0);
        assert!(selected_row.x + selected_row.width <= browser.x + browser.width - 1.0);
        if let Ok(path) = std::env::var("CLEAN_THE_AGENT_TWEAK_SNAPSHOT") {
            visual
                .capture_screenshot()
                .expect("capture tweak interface")
                .write_png(path)
                .expect("write tweak interface snapshot");
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn orca_scan_controls_have_clear_separate_scopes() {
        let mut app = CleanerApp::new();
        app.pending_scan = false;
        app.page = Page::Cleanup;
        app.report = Some(ScanReport::default());
        let (mut context, view) = Application::new()
            .into_test_context(WindowOptions::default().size(1120.0, 740.0), app)
            .expect("visual test context");
        let mut visual = context
            .visual(view.window_handle())
            .expect("visual context");
        let choose = visual
            .element_bounds("choose-home")
            .expect("choose-folder button bounds");
        let scan = visual.element_bounds("scan").expect("scan button bounds");
        let status = visual
            .element_bounds("status-strip")
            .expect("status strip bounds");
        let empty = visual
            .element_bounds("empty-state-panel")
            .expect("empty state bounds");
        let provider = visual
            .element_bounds("provider-orca")
            .expect("Orca provider bounds");
        let provider_icon = visual
            .element_bounds("orca-provider-icon")
            .expect("Orca provider icon bounds");

        assert_eq!(choose.height, 36.0);
        assert_eq!(scan.height, 36.0);
        assert!((scan.y - 14.0).abs() <= 1.0);
        assert!(choose.y >= status.y);
        assert!(choose.y + choose.height <= status.y + 60.0);
        assert!(choose.x + choose.width <= status.x + status.width - 16.0);
        assert_eq!(provider.x, 252.0);
        assert_eq!(provider.width, 848.0);
        assert_eq!(provider_icon.width, 56.0);
        assert_eq!(provider_icon.height, 56.0);
        assert_eq!(status.x, 252.0);
        assert_eq!(status.width, 848.0);
        assert_eq!(empty.x, status.x);
        assert_eq!(empty.width, status.width);

        if let Ok(path) = std::env::var("CLEAN_THE_AGENT_SNAPSHOT") {
            visual
                .capture_screenshot()
                .expect("capture interface")
                .write_png(path)
                .expect("write interface snapshot");
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn overview_groups_all_scanned_providers() {
        let mut app = CleanerApp::new();
        app.pending_scan = false;
        app.report = Some(ScanReport::default());
        let (mut context, view) = Application::new()
            .into_test_context(WindowOptions::default().size(1120.0, 740.0), app)
            .expect("visual test context");
        let mut visual = context
            .visual(view.window_handle())
            .expect("visual context");
        let summary = visual
            .element_bounds("overview-summary")
            .expect("overview summary bounds");
        let orca = visual
            .element_bounds("overview-orca")
            .expect("overview Orca row bounds");
        let codex = visual
            .element_bounds("overview-codex")
            .expect("overview Codex row bounds");

        assert_eq!(summary.x, 252.0);
        assert_eq!(summary.width, 848.0);
        assert_eq!(orca.x, summary.x + 1.0);
        assert_eq!(orca.width, summary.width - 2.0);
        assert_eq!(orca.height, 82.0);
        assert_eq!(codex.x, orca.x);
        assert_eq!(codex.y, orca.y + orca.height);
        if let Ok(path) = std::env::var("CLEAN_THE_AGENT_OVERVIEW_SNAPSHOT") {
            visual
                .capture_screenshot()
                .expect("capture overview")
                .write_png(path)
                .expect("write overview snapshot");
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn about_page_fits_the_minimum_window_without_scrolling() {
        let mut app = CleanerApp::new();
        app.pending_scan = false;
        app.page = Page::About;
        app.report = Some(ScanReport::default());
        let (mut context, view) = Application::new()
            .into_test_context(WindowOptions::default().size(920.0, 620.0), app)
            .expect("visual test context");
        {
            let mut visual = context
                .visual(view.window_handle())
                .expect("visual context");
            let page = visual
                .element_bounds("about-page")
                .expect("about page bounds");
            let app_icon = visual
                .element_bounds("about-app-icon")
                .expect("application icon bounds");
            let sidebar = visual
                .element_bounds("app-sidebar")
                .expect("sidebar bounds");

            assert_eq!(page.x, 232.0);
            assert_eq!(page.width, 688.0);
            assert_eq!(sidebar.height, 620.0);
            assert!(
                page.height <= sidebar.height - super::design::TOOLBAR_HEIGHT,
                "about page {} must fit inside content viewport {}",
                page.height,
                sidebar.height - super::design::TOOLBAR_HEIGHT
            );
            assert_eq!(app_icon.width, 88.0);
            assert_eq!(app_icon.height, 88.0);
            assert_eq!(app_icon.x, page.x + 16.0);
            assert!(app_icon.y >= page.y + 16.0);
            assert!(app_icon.y <= page.y + 24.0);
            if let Ok(path) = std::env::var("CLEAN_THE_AGENT_ABOUT_SNAPSHOT") {
                visual
                    .capture_screenshot()
                    .expect("capture about interface")
                    .write_png(path)
                    .expect("write about interface snapshot");
            }
        }
        let accepted_scroll = context
            .simulate_retained_scroll(
                view.window_handle(),
                "page-static-about",
                quickgui::Vector::new(0.0, -10_000.0),
            )
            .expect("probe About scrollability");
        assert!(
            !accepted_scroll,
            "About must not accept scrolling at the minimum window size"
        );
        assert!(
            context
                .retained_scroll_offset(view.window_handle(), "page-static-about")
                .is_err(),
            "About viewport must not retain a scroll offset"
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn about_page_renders_with_macos_dark_semantic_colors() {
        use quickgui::{ColorScheme, SystemColor, SystemColorRole, SystemPreferences};

        let mut app = CleanerApp::new();
        app.pending_scan = false;
        app.page = Page::About;
        let (mut context, view) = Application::new()
            .into_test_context(WindowOptions::default().size(920.0, 620.0), app)
            .expect("visual test context");
        let preferences = SystemPreferences::default()
            .with_color_scheme(ColorScheme::Dark)
            .with_system_color(
                SystemColorRole::WindowBackground,
                Some(SystemColor::rgb(30, 30, 30)),
            )
            .with_system_color(
                SystemColorRole::ControlBackground,
                Some(SystemColor::rgb(30, 30, 30)),
            )
            .with_system_color(
                SystemColorRole::WindowText,
                Some(SystemColor::rgba(255, 255, 255, 216)),
            );
        context
            .simulate_system_preferences_change(preferences)
            .expect("switch test appearance");
        context.advance_frame().expect("render dark appearance");
        let mut visual = context
            .visual(view.window_handle())
            .expect("visual context");

        assert_eq!(
            visual
                .element_bounds("app-sidebar")
                .expect("sidebar bounds")
                .height,
            620.0
        );
        let page = visual
            .element_bounds("about-page")
            .expect("about page bounds");
        assert!(page.height <= 620.0 - super::design::TOOLBAR_HEIGHT);
        if let Ok(path) = std::env::var("CLEAN_THE_AGENT_DARK_SNAPSHOT") {
            visual
                .capture_screenshot()
                .expect("capture dark interface")
                .write_png(path)
                .expect("write dark interface snapshot");
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn changing_pages_does_not_reuse_the_previous_scroll_position() {
        use clean_any::model::{Ownership, ScopeKind};
        use clean_any::{Finding, Safety};
        use quickgui::Vector;
        use std::path::PathBuf;

        let mut app = CleanerApp::new();
        app.pending_scan = false;
        app.page = Page::Cleanup;
        app.report = Some(ScanReport {
            findings: vec![Finding {
                id: "scroll-fixture".to_owned(),
                provider: "orca".to_owned(),
                kind: ArtifactKind::Cache,
                ownership: Ownership::ProviderOwned,
                safety: Safety::Automatic,
                path: PathBuf::from("/tmp/orca-cache"),
                scope: ScopeKind::Local,
                description: "Orca cache".to_owned(),
                evidence: "visual test fixture".to_owned(),
                reclaimable_bytes: 64,
                action: None,
            }],
            warnings: (0..20)
                .map(|index| format!("Scroll fixture warning {index}"))
                .collect(),
        });
        let (mut context, view) = Application::new()
            .into_test_context(WindowOptions::default().size(1120.0, 740.0), app)
            .expect("visual test context");
        let window = view.window_handle();

        assert!(
            context
                .simulate_retained_scroll(window, "page-scroll-cleanup", Vector::new(0.0, -400.0),)
                .expect("scroll Orca page")
        );
        assert!(
            context
                .retained_scroll_offset(window, "page-scroll-cleanup")
                .expect("Orca scroll offset")
                .y
                > 0.0
        );

        context
            .click(window, "show-about")
            .expect("open About page");
        assert!(
            !context
                .simulate_retained_scroll(window, "page-static-about", Vector::new(0.0, -400.0),)
                .expect("probe About viewport")
        );
        let mut visual = context.visual(window).expect("visual context");
        let page = visual
            .element_bounds("about-page")
            .expect("about page bounds");
        let app_icon = visual
            .element_bounds("about-app-icon")
            .expect("application icon bounds");
        assert!(app_icon.y < page.y + 32.0);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn toast_is_anchored_to_the_window_bottom_right() {
        use quickgui::ToastViewport;
        use std::time::{Duration, Instant};

        let mut app = CleanerApp::new();
        app.pending_scan = false;
        let toast_id = app.toasts.push(
            apply_toast(
                "Tweak",
                ApplyCounts {
                    applied: 1,
                    skipped: 0,
                    failed: 0,
                },
            ),
            Instant::now(),
        );
        let viewport = ToastViewport::new("app-toasts");
        let root_id = viewport
            .toast(app.toasts.entry(toast_id).expect("queued toast"))
            .root_id();
        let (mut context, view) = Application::new()
            .into_test_context(WindowOptions::default().size(1120.0, 740.0), app)
            .expect("visual test context");
        let initial_x = context
            .visual(view.window_handle())
            .expect("initial toast frame")
            .element_bounds(root_id)
            .expect("initial toast bounds")
            .x;
        assert_eq!(initial_x, 752.0);
        context
            .advance_time(Duration::from_millis(160))
            .expect("advance toast to midpoint");
        context.advance_frame().expect("render midpoint toast");
        let midpoint_x = context
            .visual(view.window_handle())
            .expect("midpoint toast frame")
            .element_bounds(root_id)
            .expect("midpoint toast bounds")
            .x;
        assert!(midpoint_x > 720.0 && midpoint_x < initial_x);
        context
            .advance_time(Duration::from_millis(160))
            .expect("finish toast entrance animation");
        context.advance_frame().expect("render settled toast");
        {
            let mut visual = context
                .visual(view.window_handle())
                .expect("visual context");
            let toast = visual.element_bounds(root_id).expect("toast bounds");
            let portal = visual
                .element_bounds(viewport.portal_id())
                .expect("toast portal bounds");

            assert_eq!(toast.width, 380.0);
            assert_eq!(toast.x, 720.0);
            assert!(toast.height >= 64.0);
            assert_eq!(toast.y + toast.height, 720.0);
            assert_eq!(portal.width, 380.0);
            assert_eq!(portal.height, toast.height);
            if let Ok(path) = std::env::var("CLEAN_THE_AGENT_TOAST_SNAPSHOT") {
                visual
                    .capture_screenshot()
                    .expect("capture toast interface")
                    .write_png(path)
                    .expect("write toast interface snapshot");
            }
        }
        context
            .advance_time(Duration::from_secs(5))
            .expect("complete toast dismissal animation");
        assert!(
            !context
                .contains_element(view.window_handle(), root_id)
                .unwrap()
        );
    }
}
