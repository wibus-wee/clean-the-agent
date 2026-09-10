mod configs;
mod integrations;
mod owned;
mod paths;
mod runtime;
mod skills;
mod state;
mod support;
mod trust;
mod worktrees;

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::model::ScanReport;
use crate::provider::{Provider, ProviderError, ScanContext};

#[derive(Default)]
pub struct OrcaProvider;

impl Provider for OrcaProvider {
    fn id(&self) -> &'static str {
        "orca"
    }

    fn display_name(&self) -> &'static str {
        "Orca"
    }

    fn detect(&self, context: &ScanContext) -> Result<ScanReport, ProviderError> {
        let mut report = ScanReport::default();
        let data_dirs = paths::orca_data_dirs(context);
        let hints = state::inspect(&data_dirs, &mut report);

        owned::inspect(context, &data_dirs, &mut report);
        runtime::inspect(context, &data_dirs, &mut report);
        let warnings_before_configs = report.warnings.len();
        configs::inspect(context, &mut report);
        trust::inspect(context, &data_dirs, &hints, &mut report);
        let configs_were_readable = report.warnings.len() == warnings_before_configs;
        integrations::inspect(context, configs_were_readable, &mut report);
        skills::inspect(context, &data_dirs, &mut report);
        worktrees::inspect_attributed(&hints, &mut report);

        let mut roots: BTreeMap<PathBuf, crate::model::ScopeKind> = context
            .workspace_roots
            .iter()
            .cloned()
            .map(|path| (path, crate::model::ScopeKind::Local))
            .collect();
        roots.extend(
            hints
                .workspace_roots
                .iter()
                .cloned()
                .map(|path| (path, crate::model::ScopeKind::Local)),
        );
        for scope in context.home_scopes() {
            roots.insert(scope.home.join("orca/workspaces"), scope.kind);
        }
        worktrees::inspect_trash(&roots, &mut report);
        Ok(report)
    }
}
