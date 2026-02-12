use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum DpkgError {
    #[error("Configuration file not found\n  Path: {path}\n  Hint: Create the file or specify a different path with --config")]
    ConfigNotFound { path: PathBuf },

    #[error("Configuration error at line {line}: {message}")]
    ConfigParse { line: usize, message: String },

    #[error("Permission denied: {0}\n  Hint: Run with sudo or check your permissions")]
    PermissionDenied(String),

    #[error("Package installation failed: {0}")]
    InstallFailed(String),

    #[error("AUR packages found but yay is not installed\n  Hint: Install yay: git clone https://aur.archlinux.org/yay.git && cd yay && makepkg -si")]
    YayNotFound,

    #[error("Network error: {0}")]
    #[allow(dead_code)]
    NetworkError(String),

    #[error("User cancelled operation")]
    UserCancelled,
}

impl DpkgError {
    pub fn exit_code(&self) -> i32 {
        match self {
            DpkgError::ConfigNotFound { .. } | DpkgError::ConfigParse { .. } => 1,
            DpkgError::PermissionDenied(_) => 2,
            DpkgError::InstallFailed(_) => 3,
            DpkgError::YayNotFound => 4,
            DpkgError::NetworkError(_) => 5,
            DpkgError::UserCancelled => 6,
        }
    }
}
