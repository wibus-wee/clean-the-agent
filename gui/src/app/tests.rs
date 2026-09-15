use super::{
    ApplyCounts, ArtifactKind, CleanerApp, CleanupCategory, FindingGroup, Page, ScanReport,
    TweakReport, TweakStatus, UiLabScenario, action_consequence, apply_toast, finding_group,
    human_bytes, humanize_debug, mock_orca_report,
};
use quickgui::{Application, ToastKind, WindowOptions};

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
        ArtifactKind::TemporaryState,
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
fn cleanup_action_kinds_are_explained_as_user_visible_consequences() {
    let report = mock_orca_report(false);

    let (title, detail) = action_consequence(&report.findings[0]);
    assert_eq!(title, "Permanently delete this directory");
    assert!(detail.contains("Nothing is moved to Trash"));

    let (title, detail) = action_consequence(&report.findings[2]);
    assert_eq!(title, "Remove this Git worktree");
    assert!(detail.contains("clean and still registered"));

    let (title, detail) = action_consequence(&report.findings[3]);
    assert_eq!(title, "Edit this configuration file");
    assert!(detail.contains("2 Orca-managed entries"));
    assert!(detail.contains("Other settings stay unchanged"));
}

#[test]
fn findings_are_grouped_by_cleanup_consequence_instead_of_detector_safety() {
    let report = mock_orca_report(false);
    let default_plan = clean_any::Engine::plan(&report, false);
    let reviewed_plan = clean_any::Engine::plan(&report, true);
    let in_plan = |index: usize, plan: &clean_any::CleanupPlan| {
        report.findings[index]
            .action
            .as_ref()
            .is_some_and(|action| plan.actions.iter().any(|item| item.id == action.id))
    };

    assert_eq!(
        finding_group(&report.findings[0], true, in_plan(0, &default_plan), false),
        FindingGroup::PermanentDelete
    );
    assert_eq!(
        finding_group(&report.findings[3], true, in_plan(3, &default_plan), false),
        FindingGroup::NeedsReview
    );
    assert_eq!(
        finding_group(&report.findings[3], true, in_plan(3, &reviewed_plan), false),
        FindingGroup::ConfigurationEdit
    );
    assert_eq!(
        finding_group(&report.findings[0], false, false, false),
        FindingGroup::ExcludedByScope
    );
    assert_eq!(
        finding_group(&report.findings[0], true, false, true),
        FindingGroup::ExcludedBySelection
    );
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

#[test]
fn excluding_a_finding_removes_only_its_action_from_the_plan() {
    let report = mock_orca_report(false);
    let mut app = CleanerApp::new();
    let action_id = report.findings[0]
        .action
        .as_ref()
        .expect("automatic finding action")
        .id
        .clone();

    assert_eq!(app.cleanup_plan(&report).actions.len(), 4);
    app.excluded_cleanup_actions.insert(action_id.clone());
    let plan = app.cleanup_plan(&report);

    assert_eq!(plan.actions.len(), 3);
    assert!(plan.actions.iter().all(|action| action.id != action_id));
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
    assert!(scan.y >= 0.0);
    assert!(scan.y + scan.height <= crate::design::TOOLBAR_HEIGHT);
    assert!(choose.y >= status.y);
    assert!(choose.y + choose.height <= status.y + 60.0);
    assert!(choose.x + choose.width <= status.x + status.width - 12.0);
    assert_eq!(provider.x, 252.0);
    assert_eq!(provider.width, 848.0);
    assert_eq!(provider_icon.width, 48.0);
    assert_eq!(provider_icon.height, 48.0);
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
        let coverage = visual
            .element_bounds("about-coverage")
            .expect("coverage section bounds");
        let safety = visual
            .element_bounds("about-safety")
            .expect("safety section bounds");
        let application = visual
            .element_bounds("about-application")
            .expect("application section bounds");

        assert_eq!(page.x, 232.0);
        assert_eq!(page.width, 688.0);
        assert_eq!(sidebar.height, 620.0);
        assert!(
            page.height <= sidebar.height - crate::design::TOOLBAR_HEIGHT,
            "about page {} must fit inside content viewport {}",
            page.height,
            sidebar.height - crate::design::TOOLBAR_HEIGHT
        );
        assert_eq!(app_icon.width, 80.0);
        assert_eq!(app_icon.height, 80.0);
        assert!(app_icon.x > page.x + 250.0);
        assert!(app_icon.x + app_icon.width < page.x + page.width - 250.0);
        assert_eq!(coverage.x, safety.x);
        assert_eq!(safety.x, application.x);
        assert_eq!(coverage.width, safety.width);
        assert_eq!(safety.width, application.width);
        assert!(coverage.y < safety.y);
        assert!(safety.y < application.y);
        assert!(
            visual.element_bounds("about-columns").is_err(),
            "About must remain a single-column page"
        );
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
    assert!(page.height <= 620.0 - crate::design::TOOLBAR_HEIGHT);
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
    assert!(app_icon.y >= page.y);
    assert!(app_icon.y + app_icon.height <= page.y + page.height);
}

#[cfg(all(target_os = "macos", debug_assertions))]
#[test]
fn ui_lab_loads_an_isolated_orca_scan_preview() {
    let mut app = CleanerApp::new();
    app.pending_scan = false;
    let (mut context, view) = Application::new()
        .into_test_context(WindowOptions::default().size(1120.0, 740.0), app)
        .expect("visual test context");
    let window = view.window_handle();

    context.click(window, "show-ui-lab").expect("open UI Lab");
    if let Ok(path) = std::env::var("CLEAN_THE_AGENT_UI_LAB_SNAPSHOT") {
        context
            .capture_screenshot(window)
            .expect("capture UI Lab")
            .write_png(path)
            .expect("write UI Lab snapshot");
    }
    context
        .click(window, "ui-lab-orca-ready")
        .expect("load Orca preview");
    let (page, preview_mode, findings, ready) = context
        .update(view, |view, _cx| {
            let report = view.report.as_ref().expect("mock report");
            (
                view.page,
                view.preview_mode,
                report.findings.len(),
                view.cleanup_plan(report).actions.len(),
            )
        })
        .expect("inspect Orca preview");
    assert_eq!(page, Page::Cleanup);
    assert!(preview_mode);
    assert_eq!(findings, 8);
    assert_eq!(ready, 4);

    context
        .click(window, "show-category-rules-2")
        .expect("expand Integration rules");
    assert_eq!(
        context
            .update(view, |view, _cx| view.expanded_category)
            .expect("inspect category disclosure"),
        Some(CleanupCategory::Integrations)
    );
    context
        .click(window, "show-category-rules-2")
        .expect("collapse Integration rules");

    context
        .click(window, "toggle-finding-mock-cache")
        .expect("exclude one finding");
    let (actions, user_excluded) = context
        .update(view, |view, _cx| {
            let report = view.report.as_ref().expect("mock report");
            (
                view.cleanup_plan(report).actions.len(),
                view.excluded_cleanup_actions.contains("mock-action-cache"),
            )
        })
        .expect("inspect per-finding selection");
    assert_eq!(actions, 3);
    assert!(user_excluded);
    assert!(
        context
            .contains_element(window, FindingGroup::ExcludedBySelection.id())
            .unwrap()
    );
    context
        .click(window, "toggle-finding-mock-cache")
        .expect("restore one finding");
    assert_eq!(
        context
            .update(view, |view, _cx| {
                let report = view.report.as_ref().expect("mock report");
                view.cleanup_plan(report).actions.len()
            })
            .expect("inspect restored selection"),
        4
    );

    context
        .click(window, "apply-cleanup")
        .expect("exercise mock cleanup action");
    let (pending_apply, toast_count, preview_mode) = context
        .update(view, |view, _cx| {
            (
                view.pending_apply.is_some(),
                view.toasts.entries().len(),
                view.preview_mode,
            )
        })
        .expect("inspect preview isolation");
    assert!(
        !pending_apply,
        "mock actions must never reach Engine::apply"
    );
    assert_eq!(toast_count, 1);
    assert!(preview_mode);

    context
        .simulate_retained_scroll(
            window,
            "page-scroll-cleanup",
            quickgui::Vector::new(0.0, -10_000.0),
        )
        .expect("scroll to the consequence-based findings");
    assert!(
        context.contains_element(window, "cleanup-plan").unwrap(),
        "a populated scan must expose a cleanup plan summary"
    );
    assert!(
        context
            .contains_element(window, FindingGroup::PermanentDelete.id())
            .unwrap()
    );
    assert!(
        context
            .contains_element(window, FindingGroup::NeedsReview.id())
            .unwrap()
    );
    assert!(
        context
            .contains_element(window, "finding-consequence")
            .unwrap()
    );
    assert!(
        context
            .contains_element(window, "finding-recognition")
            .unwrap()
    );

    if let Ok(path) = std::env::var("CLEAN_THE_AGENT_UI_LAB_ORCA_SNAPSHOT") {
        context
            .capture_screenshot(window)
            .expect("capture Orca preview")
            .write_png(path)
            .expect("write Orca preview snapshot");
    }
}

#[cfg(all(target_os = "macos", debug_assertions))]
#[test]
fn ui_lab_review_preview_moves_configuration_changes_into_the_plan() {
    let mut app = CleanerApp::new();
    app.pending_scan = false;
    let (mut context, view) = Application::new()
        .into_test_context(WindowOptions::default().size(1120.0, 740.0), app)
        .expect("visual test context");
    let window = view.window_handle();

    context.click(window, "show-ui-lab").expect("open UI Lab");
    context
        .click(window, "ui-lab-orca-warning")
        .expect("load review preview");
    let (include_review, actions, selected_kind) = context
        .update(view, |view, _cx| {
            let report = view.report.as_ref().expect("mock report");
            (
                view.include_review,
                view.cleanup_plan(report).actions.len(),
                view.selected_finding
                    .and_then(|index| report.findings.get(index))
                    .map(|finding| finding.kind),
            )
        })
        .expect("inspect review plan");
    assert!(include_review);
    assert_eq!(actions, 8);
    assert_eq!(selected_kind, Some(ArtifactKind::ConfigMutation));

    context
        .simulate_retained_scroll(
            window,
            "page-scroll-cleanup",
            quickgui::Vector::new(0.0, -10_000.0),
        )
        .expect("scroll to reviewed cleanup plan");
    assert!(
        context
            .contains_element(window, FindingGroup::ConfigurationEdit.id())
            .unwrap()
    );
    assert!(
        !context
            .contains_element(window, FindingGroup::NeedsReview.id())
            .unwrap()
    );

    if let Ok(path) = std::env::var("CLEAN_THE_AGENT_UI_LAB_REVIEW_SNAPSHOT") {
        context
            .capture_screenshot(window)
            .expect("capture reviewed cleanup plan")
            .write_png(path)
            .expect("write reviewed cleanup plan snapshot");
    }
}

#[cfg(target_os = "macos")]
#[test]
fn orca_findings_use_a_compact_inspector_at_minimum_width() {
    let mut app = CleanerApp::new();
    app.install_ui_lab_scenario(UiLabScenario::OrcaReady);
    let (mut context, view) = Application::new()
        .into_test_context(WindowOptions::default().size(920.0, 620.0), app)
        .expect("compact visual test context");
    let window = view.window_handle();
    context
        .simulate_retained_scroll(
            window,
            "page-scroll-cleanup",
            quickgui::Vector::new(0.0, -10_000.0),
        )
        .expect("scroll compact preview to findings");
    let mut visual = context.visual(window).expect("compact visual context");
    let browser = visual
        .element_bounds("findings-browser")
        .expect("findings browser bounds");
    let list = visual
        .element_bounds("finding-list")
        .expect("finding list bounds");
    let detail = visual
        .element_bounds("finding-detail")
        .expect("finding detail bounds");
    let facts = visual
        .element_bounds("finding-facts")
        .expect("finding metadata bounds");
    let selected = visual
        .element_bounds("select-finding-0")
        .expect("selected finding bounds");

    assert_eq!(list.width, 280.0);
    assert_eq!(list.x + list.width, detail.x);
    assert_eq!(list.width + detail.width, browser.width - 2.0);
    assert!(detail.width >= 340.0);
    assert!(facts.x >= detail.x);
    assert!(facts.x + facts.width <= detail.x + detail.width);
    assert!((80.0..=84.0).contains(&selected.height));

    if let Ok(path) = std::env::var("CLEAN_THE_AGENT_COMPACT_SNAPSHOT") {
        visual
            .capture_screenshot()
            .expect("capture compact findings inspector")
            .write_png(path)
            .expect("write compact findings snapshot");
    }
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
