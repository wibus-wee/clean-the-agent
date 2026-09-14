use std::collections::BTreeSet;
use std::path::PathBuf;

use crate::provider::{Platform, ScanContext};

pub(crate) fn orca_data_dirs(context: &ScanContext) -> Vec<PathBuf> {
    let mut dirs: BTreeSet<PathBuf> = context.orca_data_dirs.iter().cloned().collect();
    dirs.insert(context.home.join(".orca"));
    if context.honor_environment {
        if let Some(path) = context.environment_path("ORCA_USER_DATA") {
            dirs.insert(path);
        }
    }
    match context.platform {
        Platform::MacOs => {
            dirs.insert(context.home.join("Library/Application Support/Orca"));
        }
        Platform::Windows => {
            let roaming = context
                .honor_environment
                .then(|| context.environment_path("APPDATA"))
                .flatten()
                .unwrap_or_else(|| context.home.join("AppData/Roaming"));
            let local = context
                .honor_environment
                .then(|| context.environment_path("LOCALAPPDATA"))
                .flatten()
                .unwrap_or_else(|| context.home.join("AppData/Local"));
            dirs.insert(roaming.join("Orca"));
            dirs.insert(local.join("Orca"));
        }
        Platform::Linux => {
            let config = context
                .honor_environment
                .then(|| context.environment_path("XDG_CONFIG_HOME"))
                .flatten()
                .unwrap_or_else(|| context.home.join(".config"));
            let data = context
                .honor_environment
                .then(|| context.environment_path("XDG_DATA_HOME"))
                .flatten()
                .unwrap_or_else(|| context.home.join(".local/share"));
            dirs.insert(config.join("Orca"));
            dirs.insert(data.join("Orca"));
        }
    }
    dirs.into_iter().collect()
}
