use std::collections::BTreeSet;
use std::path::PathBuf;

use crate::provider::{Platform, ScanContext};

pub(crate) fn orca_data_dirs(context: &ScanContext) -> Vec<PathBuf> {
    let mut dirs: BTreeSet<PathBuf> = context.orca_data_dirs.iter().cloned().collect();
    dirs.insert(context.home.join(".orca"));
    if context.honor_environment
        && let Some(path) = context.environment_path("ORCA_USER_DATA_PATH")
    {
        dirs.insert(path);
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
            dirs.insert(config.join("orca"));
        }
    }
    dirs.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::Platform;

    #[test]
    fn linux_uses_orcas_xdg_config_path() {
        let context = ScanContext {
            home: PathBuf::from("/home/alice"),
            platform: Platform::Linux,
            orca_data_dirs: Vec::new(),
            workspace_roots: Vec::new(),
            honor_environment: false,
            additional_homes: Vec::new(),
            stale_after_days: 30,
        };

        assert_eq!(
            orca_data_dirs(&context),
            vec![
                PathBuf::from("/home/alice/.config/orca"),
                PathBuf::from("/home/alice/.orca"),
            ]
        );
    }
}
