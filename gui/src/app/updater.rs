use super::*;

use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use quickgui::{AvailableUpdate, UpdateClient, UpdateInstallOptions};

const UPDATE_PUBLIC_KEY: &str = "RWTqQoEXz17+ylukrN57OdRSe7HY3F42mUBRe8/xERyyJVzacIGrn+Et";
const STABLE_MANIFEST_ROOT: &str =
    "https://github.com/wibus-wee/clean-the-agent/releases/latest/download";
const BETA_MANIFEST_ROOT: &str =
    "https://github.com/wibus-wee/clean-the-agent/releases/download/updater-beta";

#[derive(Clone, Debug)]
pub(super) enum UpdateStatus {
    Idle,
    Checking,
    Current,
    Available(AvailableUpdate),
    Installing(String),
    Failed(String),
}

impl UpdateStatus {
    pub(super) fn available(&self) -> Option<&AvailableUpdate> {
        match self {
            Self::Available(update) => Some(update),
            _ => None,
        }
    }

    pub(super) fn busy(&self) -> bool {
        matches!(self, Self::Checking | Self::Installing(_))
    }
}

pub(super) fn beta_channel() -> bool {
    env!("CARGO_PKG_VERSION").contains('-')
}

pub(super) fn update_status_text(status: &UpdateStatus) -> String {
    match status {
        UpdateStatus::Idle => "Signed updates from GitHub Releases".to_owned(),
        UpdateStatus::Checking => "Checking for updates…".to_owned(),
        UpdateStatus::Current => "You’re up to date".to_owned(),
        UpdateStatus::Available(update) => format!("Version {} is available", update.version),
        UpdateStatus::Installing(version) => format!("Installing version {version}…"),
        UpdateStatus::Failed(error) => format!("Update failed: {error}"),
    }
}

pub(super) fn update_action_label(status: &UpdateStatus) -> String {
    match status {
        UpdateStatus::Checking => "Checking…".to_owned(),
        UpdateStatus::Available(update) => format!("Install v{}", update.version),
        UpdateStatus::Installing(_) => "Installing…".to_owned(),
        _ => "Check for Updates".to_owned(),
    }
}

fn update_manifest_url() -> String {
    let root = if beta_channel() {
        BETA_MANIFEST_ROOT
    } else {
        STABLE_MANIFEST_ROOT
    };
    format!("{root}/latest-{}.json", quickgui::default_update_target())
}

fn update_client() -> Result<UpdateClient, String> {
    UpdateClient::new(env!("CARGO_PKG_VERSION"), UPDATE_PUBLIC_KEY)
        .map_err(|error| error.to_string())
}

fn check_for_update() -> Result<Option<AvailableUpdate>, String> {
    update_client()?
        .check(&update_manifest_url())
        .map_err(|error| error.to_string())
}

fn install_update(update: &AvailableUpdate) -> Result<(), String> {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_nanos();
    let staging = std::env::temp_dir().join(format!(
        "clean-the-agent-update-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(&staging).map_err(|error| error.to_string())?;

    let result = (|| {
        let client = update_client()?;
        let artifact = client
            .download_and_stage(update, &staging)
            .map_err(|error| error.to_string())?;
        client
            .install_staged(update, artifact, UpdateInstallOptions::new())
            .map_err(|error| error.to_string())?;
        Ok(())
    })();

    let _ = fs::remove_dir_all(staging);
    result
}

impl CleanerApp {
    pub(super) fn schedule_update_check(&mut self) {
        if !self.update_status.busy() {
            self.pending_update_check = true;
        }
    }

    pub(super) fn start_update_work(&mut self, cx: &ViewContext<'_, Self>) {
        if self.pending_update_check && !self.update_status.busy() {
            self.pending_update_check = false;
            self.update_status = UpdateStatus::Checking;
            if let Err(error) = cx.spawn_background(check_for_update, |view, result, cx| {
                view.update_status = match result {
                    Ok(Ok(Some(update))) => {
                        view.toasts.push(
                            Toast::new(format!("Version {} is available", update.version))
                                .description("Open About to review and install the signed update.")
                                .kind(ToastKind::Info)
                                .duration(Duration::from_secs(8)),
                            Instant::now(),
                        );
                        UpdateStatus::Available(update)
                    }
                    Ok(Ok(None)) => UpdateStatus::Current,
                    Ok(Err(error)) => UpdateStatus::Failed(error),
                    Err(error) => UpdateStatus::Failed(error.to_string()),
                };
                cx.invalidate();
            }) {
                self.update_status = UpdateStatus::Failed(error.to_string());
            }
        }

        if let Some(update) = self.pending_update_install.take() {
            let version = update.version.clone();
            self.update_status = UpdateStatus::Installing(version.clone());
            if let Err(error) = cx.spawn_background(
                move || install_update(&update),
                move |view, result, cx| {
                    match result {
                        Ok(Ok(())) => {
                            if let Err(error) = cx.relaunch() {
                                view.update_status = UpdateStatus::Failed(error.to_string());
                            }
                        }
                        Ok(Err(error)) => view.update_status = UpdateStatus::Failed(error),
                        Err(error) => view.update_status = UpdateStatus::Failed(error.to_string()),
                    }
                    cx.invalidate();
                },
            ) {
                self.update_status = UpdateStatus::Failed(error.to_string());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn updater_public_key_is_accepted() {
        UpdateClient::new("1.0.0", UPDATE_PUBLIC_KEY).expect("embedded updater key is valid");
    }

    #[test]
    fn manifest_name_matches_the_runtime_target() {
        assert!(update_manifest_url().ends_with(&format!(
            "latest-{}.json",
            quickgui::default_update_target()
        )));
    }
}
