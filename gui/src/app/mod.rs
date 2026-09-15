mod about;
mod cleanup;
mod overview;
mod presentation;
mod shell;
mod tweaks;
#[cfg(debug_assertions)]
mod ui_lab;

use presentation::*;
#[cfg(debug_assertions)]
use ui_lab::{mock_codex_report, mock_orca_report};

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use crate::design::{
    Theme, category_tile, destructive_button, empty_state, message_panel, panel, primary_button,
    provider_icon, provider_sidebar_row, review_checkbox, setting_switch, sidebar,
    sidebar_icon_row, status_badge, toolbar, toolbar_button,
};
use crate::icons::{IconName, animated_keyboard_icon, chatgpt_icon, icon, system_info_icon};
use clean_any::model::{ApplyStatus, ArtifactKind, CleanupPlan};
#[cfg(debug_assertions)]
use clean_any::model::{CleanupAction, CleanupActionKind, PathSnapshot};
use clean_any::providers::OrcaProvider;
use clean_any::tweaks::DisableCodexPetShortcut;
use clean_any::{
    ApplyReport, Engine, Finding, Ownership, Safety, ScanContext, ScanReport, ScopeKind,
    TweakEngine, TweakReport, TweakStatus,
};
use quickgui::{
    Animation, AnimationExt, AsyncViewContext, ClickListener, Element, EventContext, IntoElement,
    MessageBoxOptions, PathPromptOptions, PromptButton, PromptLevel, Toast, ToastId, ToastKind,
    ToastManager, ToastViewport, Transition, TransitionProperties, View, ViewContext, button, div,
    text,
};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum Page {
    #[default]
    Overview,
    Cleanup,
    Tweaks,
    #[cfg(debug_assertions)]
    UiLab,
    About,
}

#[cfg(debug_assertions)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum UiLabScenario {
    OrcaReady,
    OrcaWarning,
    OrcaEmpty,
    OrcaScanning,
    Overview,
    CodexAllowed,
    CodexBlocked,
    CodexAttention,
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
            Self::AppData => {
                "Electron caches, temporary state, logs, terminal history, backups and owned data."
            }
            Self::Workspaces => {
                "Orca-created Git worktrees, deferred trash and stale provenance records."
            }
            Self::Integrations => {
                "Managed hooks, agent config entries, workspace trust and integration plugins."
            }
            Self::AgentRuntime => {
                "Codex runtime homes, session backfill, verified skills and remote runtime residue."
            }
        }
    }

    const fn supported_types(self) -> usize {
        match self {
            Self::AppData => 6,
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
                    | ArtifactKind::TemporaryState
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

pub(crate) struct CleanerApp {
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
    preview_mode: bool,
    toasts: ToastManager,
    exiting_toasts: HashSet<ToastId>,
    toast_deadline: Option<Instant>,
}

impl CleanerApp {
    pub(crate) fn new() -> Self {
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
            preview_mode: false,
            toasts: ToastManager::new().limit(3),
            exiting_toasts: HashSet::new(),
            toast_deadline: None,
        }
    }

    fn busy(&self) -> bool {
        self.scanning || self.applying
    }

    fn schedule_scan(&mut self) {
        if self.preview_mode {
            self.preview_mode = false;
            self.scanning = false;
            self.applying = false;
            self.report = None;
            self.tweak_reports.clear();
            self.selected_finding = None;
            self.selected_tweak = None;
        }
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

    #[cfg(debug_assertions)]
    fn install_ui_lab_scenario(&mut self, scenario: UiLabScenario) {
        self.preview_mode = true;
        self.pending_scan = false;
        self.scanning = scenario == UiLabScenario::OrcaScanning;
        self.pending_apply = None;
        self.applying = false;
        self.error = None;
        self.last_apply = None;
        self.include_review = false;
        self.included_categories = [true; 4];
        self.display_home = PathBuf::from("/Users/demo");
        self.selected_finding = None;
        self.selected_tweak = None;

        match scenario {
            UiLabScenario::OrcaReady => {
                self.page = Page::Cleanup;
                self.report = Some(mock_orca_report(false));
                self.selected_finding = Some(0);
            }
            UiLabScenario::OrcaWarning => {
                self.page = Page::Cleanup;
                self.report = Some(mock_orca_report(true));
                self.selected_finding = Some(2);
                self.include_review = true;
            }
            UiLabScenario::OrcaEmpty => {
                self.page = Page::Cleanup;
                self.report = Some(ScanReport::default());
            }
            UiLabScenario::OrcaScanning => {
                self.page = Page::Cleanup;
                self.report = None;
            }
            UiLabScenario::Overview => {
                self.page = Page::Overview;
                self.report = Some(mock_orca_report(false));
                self.tweak_reports = vec![mock_codex_report(TweakStatus::Satisfied)];
                self.selected_tweak = Some(0);
            }
            UiLabScenario::CodexAllowed
            | UiLabScenario::CodexBlocked
            | UiLabScenario::CodexAttention => {
                self.page = Page::Tweaks;
                let status = match scenario {
                    UiLabScenario::CodexAllowed => TweakStatus::NeedsChange,
                    UiLabScenario::CodexBlocked => TweakStatus::Satisfied,
                    UiLabScenario::CodexAttention => TweakStatus::Blocked,
                    _ => unreachable!(),
                };
                self.tweak_reports = vec![mock_codex_report(status)];
                self.selected_tweak = Some(0);
            }
        }
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
                .flex_col()
                .child(content.flex_none())
        };
        let page_content = match self.page {
            Page::Overview => scroll_page("page-scroll-overview", self.render_overview(cx, theme)),
            Page::Cleanup => scroll_page("page-scroll-cleanup", self.render_cleanup(cx, theme)),
            Page::Tweaks => scroll_page("page-scroll-tweaks", self.render_tweaks(cx, theme)),
            #[cfg(debug_assertions)]
            Page::UiLab => scroll_page("page-scroll-ui-lab", self.render_ui_lab(cx, theme)),
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
mod tests;
