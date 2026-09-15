use super::*;

impl CleanerApp {
    pub(super) fn render_cleanup(&self, cx: &mut ViewContext<'_, Self>, theme: Theme) -> Element {
        let viewport = cx.size();
        let compact = viewport.width < 1080.0;
        let toggle_review = cx.listener("toggle-review", |this, cx: &mut EventContext| {
            if !this.busy() {
                this.include_review = !this.include_review;
                this.confirming_cleanup = false;
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

        let mut content = div()
            .w_full()
            .p(if compact { 16.0 } else { 20.0 })
            .flex_col()
            .gap(if compact { 12.0 } else { 16.0 });
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
                    "All 16 supported Orca artifact types were checked. No cleanable artifacts were found in this folder.",
                    true,
                    theme,
                )
                .id("empty-state-panel"),
            );
        }

        let plan = self.cleanup_plan(report);
        let planned_ids = plan
            .actions
            .iter()
            .map(|action| action.id.as_str())
            .collect::<HashSet<_>>();
        let user_excluded = report
            .findings
            .iter()
            .filter(|finding| {
                let included_by_scope = CleanupCategory::ALL
                    .iter()
                    .find(|category| category.contains(finding.kind))
                    .is_none_or(|category| self.category_is_included(*category));
                let included_by_review = finding.safety == Safety::Automatic
                    || (finding.safety == Safety::ReviewRequired && self.include_review);
                included_by_scope
                    && included_by_review
                    && finding
                        .action
                        .as_ref()
                        .is_some_and(|action| !planned_ids.contains(action.id.as_str()))
            })
            .count();
        let plan_for_click = plan.clone();
        let contains_mock_actions = plan_for_click
            .actions
            .iter()
            .any(|action| action.id.starts_with("mock-action-"));
        let clean = cx.listener("apply-cleanup", move |this, cx: &mut EventContext| {
            if this.busy() || plan_for_click.actions.is_empty() {
                return;
            }
            if this.preview_mode || contains_mock_actions {
                this.toasts.push(
                    Toast::new("Preview action captured")
                        .description(format!(
                            "{} mock cleanup actions were left in memory; no files changed.",
                            plan_for_click.actions.len()
                        ))
                        .kind(ToastKind::Success)
                        .duration(Duration::from_secs(4)),
                    Instant::now(),
                );
                cx.invalidate();
                return;
            }
            if !this.confirming_cleanup {
                this.confirming_cleanup = true;
                cx.invalidate();
                return;
            }
            this.confirming_cleanup = false;
            this.pending_apply = Some(PendingApply::Cleanup(plan_for_click.clone()));
            cx.invalidate();
        });
        let cancel_cleanup = cx.listener("cancel-cleanup", |this, cx: &mut EventContext| {
            this.confirming_cleanup = false;
            cx.invalidate();
        });

        content
            .child(cleanup_plan_summary(report, &plan, user_excluded, theme))
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
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .when(self.confirming_cleanup, |actions| {
                                actions.child(toolbar_button(
                                    "Cancel",
                                    cancel_cleanup,
                                    self.busy(),
                                    theme,
                                ))
                            })
                            .child(destructive_button(
                                if self.applying {
                                    "Cleaning…".to_owned()
                                } else if plan.actions.is_empty() {
                                    "Nothing to Clean".to_owned()
                                } else if self.confirming_cleanup {
                                    format!("Confirm Permanent Cleanup ({})", plan.actions.len())
                                } else {
                                    format!("Clean {} Items…", plan.actions.len())
                                },
                                clean,
                                self.busy() || plan.actions.is_empty(),
                                theme,
                            )),
                    ),
            )
            .child(
                div()
                    .id("findings-browser")
                    .h(if compact { 420.0 } else { 468.0 })
                    .min_h(360.0)
                    .flex()
                    .border(1.0, theme.separator)
                    .rounded(crate::design::PANEL_RADIUS)
                    .overflow_hidden()
                    .child(self.render_finding_list(cx, report, &plan, compact, theme))
                    .child(self.render_finding_detail(report, &plan, compact, theme)),
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
                        this.confirming_cleanup = false;
                        cx.invalidate();
                    }
                },
            );
            let disclosure = cx.listener(
                format!("show-category-rules-{}", category.index()),
                move |this, cx: &mut EventContext| {
                    this.expanded_category = if this.expanded_category == Some(category) {
                        None
                    } else {
                        Some(category)
                    };
                    cx.invalidate();
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
                crate::design::CategoryTile {
                    title: category.title(),
                    detail: category.detail(),
                    meta,
                    rules: category.rules(),
                    selected: self.category_is_included(category),
                    expanded: self.expanded_category == Some(category),
                    theme,
                },
                toggle,
                disclosure,
            )
            .when(category.index() > 0, |tile| {
                tile.border_top(1.0, theme.separator)
            })
        });

        panel(theme)
            .id("provider-orca")
            .w_full()
            .overflow_hidden()
            .flex_col()
            .child(
                div()
                    .p_3()
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
                                provider_icon(icon, 48.0, 14.0).id("orca-provider-icon"),
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
                                        text("Current or selected macOS home")
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
                                text("16 artifact types")
                                    .text_size(12.0)
                                    .text_color(theme.secondary_text),
                            ),
                    ),
            )
            .child(
                div()
                    .p_3()
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
                                text("These switches control what can be cleaned. All supported categories are still scanned.")
                                    .text_size(13.0)
                                    .text_color(theme.secondary_text),
                            ),
                    )
                    .child(
                        div()
                            .w_full()
                            .overflow_hidden()
                            .rounded(12.0)
                            .border(1.0, theme.separator)
                            .flex_col()
                            .children(cards),
                    ),
            )
    }

    fn render_finding_list(
        &self,
        cx: &mut ViewContext<'_, Self>,
        report: &ScanReport,
        plan: &CleanupPlan,
        compact: bool,
        theme: Theme,
    ) -> Element {
        let planned_ids = plan
            .actions
            .iter()
            .map(|action| action.id.as_str())
            .collect::<HashSet<_>>();
        let mut sections = Vec::new();
        for group in FindingGroup::ALL {
            let indices = report
                .findings
                .iter()
                .enumerate()
                .filter_map(|(index, finding)| {
                    let included_by_scope = CleanupCategory::ALL
                        .iter()
                        .find(|category| category.contains(finding.kind))
                        .is_none_or(|category| self.category_is_included(*category));
                    let included_in_plan = finding
                        .action
                        .as_ref()
                        .is_some_and(|action| planned_ids.contains(action.id.as_str()));
                    let eligible_for_selection = included_by_scope
                        && (finding.safety == Safety::Automatic
                            || (finding.safety == Safety::ReviewRequired && self.include_review));
                    let excluded_by_selection =
                        finding.action.is_some() && eligible_for_selection && !included_in_plan;
                    (finding_group(
                        finding,
                        included_by_scope,
                        included_in_plan,
                        excluded_by_selection,
                    ) == group)
                        .then_some(index)
                })
                .collect::<Vec<_>>();
            if indices.is_empty() {
                continue;
            }
            sections.push(
                div()
                    .id(group.id())
                    .h(30.0)
                    .px_3()
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_between()
                    .border_bottom(1.0, theme.separator)
                    .bg(theme.control)
                    .child(
                        text(group.title())
                            .text_size(10.0)
                            .font_semibold()
                            .text_color(theme.secondary_text),
                    )
                    .child(
                        text(indices.len().to_string())
                            .text_size(10.0)
                            .font_medium()
                            .text_color(theme.secondary_text),
                    ),
            );
            for index in indices {
                let finding = &report.findings[index];
                let included_by_scope = CleanupCategory::ALL
                    .iter()
                    .find(|category| category.contains(finding.kind))
                    .is_none_or(|category| self.category_is_included(*category));
                let included_in_plan = finding
                    .action
                    .as_ref()
                    .is_some_and(|action| planned_ids.contains(action.id.as_str()));
                let selection = finding.action.as_ref().map(|action| {
                    let enabled = included_by_scope
                        && (finding.safety == Safety::Automatic
                            || (finding.safety == Safety::ReviewRequired && self.include_review))
                        && !self.busy();
                    let selection_ids =
                        cleanup_selection_closure(report, &action.id, !included_in_plan);
                    let toggle = cx.listener(
                        format!("toggle-finding-{}", finding.id),
                        move |this, cx: &mut EventContext| {
                            if this.busy() || !enabled {
                                return;
                            }
                            if included_in_plan {
                                this.excluded_cleanup_actions
                                    .extend(selection_ids.iter().cloned());
                            } else {
                                for id in &selection_ids {
                                    this.excluded_cleanup_actions.remove(id);
                                }
                            }
                            this.confirming_cleanup = false;
                            cx.invalidate();
                        },
                    );
                    (included_in_plan, enabled, toggle)
                });
                let select = cx.listener(
                    format!("select-finding-{index}"),
                    move |this, cx: &mut EventContext| {
                        this.selected_finding = Some(index);
                        cx.invalidate();
                    },
                );
                sections.push(finding_row(
                    finding,
                    group,
                    self.selected_finding == Some(index),
                    selection,
                    select,
                    theme,
                ));
            }
        }

        div()
            .id("finding-list")
            .w(if compact { 280.0 } else { 340.0 })
            .h_full()
            .min_w(if compact { 260.0 } else { 320.0 })
            .flex_none()
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
                    .flex_col()
                    .when(report.findings.is_empty(), |list| {
                        list.child(empty_state(
                            "No supported artifacts found",
                            "Try another folder or scan again later.",
                            theme,
                        ))
                    })
                    .children(sections),
            )
    }

    fn render_finding_detail(
        &self,
        report: &ScanReport,
        plan: &CleanupPlan,
        compact: bool,
        theme: Theme,
    ) -> Element {
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

        let type_label = artifact_label(finding.kind);
        let ownership = ownership_label(finding.ownership);
        let scope = scope_label(finding.scope);
        let size = if finding.reclaimable_bytes > 0 {
            human_bytes(finding.reclaimable_bytes)
        } else {
            "Not estimated".to_owned()
        };
        let included = finding
            .action
            .as_ref()
            .is_some_and(|action| plan.actions.iter().any(|item| item.id == action.id));
        let included_by_scope = CleanupCategory::ALL
            .iter()
            .find(|category| category.contains(finding.kind))
            .is_none_or(|category| self.category_is_included(*category));
        let eligible_for_selection = included_by_scope
            && (finding.safety == Safety::Automatic
                || (finding.safety == Safety::ReviewRequired && self.include_review));
        let group = finding_group(
            finding,
            included_by_scope,
            included,
            finding.action.is_some() && eligible_for_selection && !included,
        );
        let (consequence, consequence_detail) = action_consequence(finding);
        let consequence_heading = if included {
            "What will happen"
        } else {
            "What would happen"
        };
        let consequence_copy = if included || finding.action.is_none() {
            format!("{consequence}. {consequence_detail}")
        } else {
            format!(
                "This item is not in the current cleanup plan. If selected: {consequence}. {consequence_detail}"
            )
        };
        let path = finding.path.display().to_string();
        let metadata = format!("{type_label} · {ownership} · {scope} · {size}");

        div()
            .id("finding-detail")
            .flex_grow(1.0)
            .min_w(0.0)
            .h_full()
            .overflow_y_scroll()
            .px(if compact { 12.0 } else { 16.0 })
            .py(12.0)
            .flex_col()
            .child(
                div()
                    .flex_none()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .min_w(0.0)
                            .flex_col()
                            .gap_1()
                            .child(
                                text(finding.description.as_str())
                                    .text_size(if compact { 17.0 } else { 18.0 })
                                    .line_height(if compact { 22.0 } else { 24.0 })
                                    .font_semibold(),
                            )
                            .child(
                                text("Detected by Orca")
                                    .text_size(12.0)
                                    .text_color(theme.secondary_text),
                            ),
                    )
                    .child(
                        div()
                            .id("finding-facts")
                            .min_w(0.0)
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(safety_badge(group, finding.safety, theme))
                            .child(
                                text(metadata)
                                    .min_w(0.0)
                                    .text_size(11.0)
                                    .text_color(theme.secondary_text)
                                    .truncate(),
                            ),
                    ),
            )
            .child(
                finding_detail_section(
                    IconName::Power,
                    consequence_heading,
                    &consequence_copy,
                    false,
                    theme,
                )
                .id("finding-consequence"),
            )
            .child(
                finding_detail_section(
                    IconName::ShieldCheck,
                    "Why it was found",
                    recognition_reason(finding),
                    false,
                    theme,
                )
                .id("finding-recognition"),
            )
            .child(finding_detail_section(
                IconName::FileBraces,
                "Exact location",
                &path,
                true,
                theme,
            ))
    }
}

fn cleanup_selection_closure(report: &ScanReport, action_id: &str, selecting: bool) -> Vec<String> {
    let actions = report
        .findings
        .iter()
        .filter_map(|finding| finding.action.as_ref())
        .collect::<Vec<_>>();
    let mut affected = HashSet::from([action_id.to_owned()]);

    loop {
        let before = affected.len();
        for action in &actions {
            if selecting {
                if affected.contains(&action.id) {
                    affected.extend(action.depends_on.iter().cloned());
                }
            } else if action
                .depends_on
                .iter()
                .any(|dependency| affected.contains(dependency))
            {
                affected.insert(action.id.clone());
            }
        }
        if affected.len() == before {
            break;
        }
    }

    affected.into_iter().collect()
}
