use std::env;
use std::path::PathBuf;

use directories::BaseDirs;
use thiserror::Error;

use crate::model::ScanReport;
use crate::model::ScopeKind;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Platform {
    Linux,
    MacOs,
    Windows,
}

impl Platform {
    #[must_use]
    pub const fn current() -> Self {
        if cfg!(target_os = "windows") {
            Self::Windows
        } else if cfg!(target_os = "macos") {
            Self::MacOs
        } else {
            Self::Linux
        }
    }
}

#[derive(Clone, Debug)]
pub struct ScanContext {
    pub home: PathBuf,
    pub platform: Platform,
    pub orca_data_dirs: Vec<PathBuf>,
    pub workspace_roots: Vec<PathBuf>,
    pub honor_environment: bool,
    pub additional_homes: Vec<HomeScope>,
    pub stale_after_days: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HomeScope {
    pub home: PathBuf,
    pub kind: ScopeKind,
}

impl ScanContext {
    /// Builds a scan context from explicit roots and platform directories.
    ///
    /// An explicit home disables environment-derived paths so an alternate
    /// home scan cannot escape into the current user's configuration.
    ///
    /// # Errors
    ///
    /// Returns [`ProviderError::HomeUnavailable`] when no explicit home was
    /// supplied and the operating system does not expose one.
    pub fn from_environment(
        home_override: Option<PathBuf>,
        orca_data_dirs: Vec<PathBuf>,
        workspace_roots: Vec<PathBuf>,
    ) -> Result<Self, ProviderError> {
        let honor_environment = home_override.is_none();
        let home = match home_override {
            Some(path) => path,
            None => BaseDirs::new()
                .map(|dirs| dirs.home_dir().to_owned())
                .ok_or(ProviderError::HomeUnavailable)?,
        };
        Ok(Self {
            home,
            platform: Platform::current(),
            orca_data_dirs,
            workspace_roots,
            honor_environment,
            additional_homes: Vec::new(),
            stale_after_days: 30,
        })
    }

    #[must_use]
    pub fn environment_path(&self, key: &str) -> Option<PathBuf> {
        env::var_os(key)
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
    }

    #[must_use]
    pub fn home_scopes(&self) -> Vec<HomeScope> {
        let mut scopes = vec![HomeScope {
            home: self.home.clone(),
            kind: ScopeKind::Local,
        }];
        for scope in &self.additional_homes {
            if !scopes.iter().any(|existing| existing == scope) {
                scopes.push(scope.clone());
            }
        }
        scopes
    }
}

#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("could not determine the user's home directory")]
    HomeUnavailable,
    #[error("provider detection failed: {0}")]
    Detection(String),
}

pub trait Provider: Send + Sync {
    fn id(&self) -> &'static str;
    fn display_name(&self) -> &'static str;

    /// Detects provider-attributed state without mutating it. Remediation is
    /// represented as data and is only executed later by the shared engine.
    ///
    /// # Errors
    ///
    /// Returns a provider error when detection cannot produce a trustworthy
    /// report. Recoverable per-path inspection problems belong in report
    /// warnings instead.
    fn detect(&self, context: &ScanContext) -> Result<ScanReport, ProviderError>;
}
