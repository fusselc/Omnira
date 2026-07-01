//! Windows data paths (docs/data-ownership-and-storage.md).
//! Everything lives under %LOCALAPPDATA%\Omnira\.

use std::path::PathBuf;

pub fn app_data_root() -> PathBuf {
    dirs::data_local_dir()
        .expect("LOCALAPPDATA must exist on Windows")
        .join("Omnira")
}

pub fn config_dir() -> PathBuf {
    app_data_root().join("config")
}

pub fn settings_path() -> PathBuf {
    config_dir().join("settings.json")
}

pub fn data_dir() -> PathBuf {
    app_data_root().join("data")
}

pub fn db_path() -> PathBuf {
    data_dir().join("omnira.db")
}

pub fn log_dir() -> PathBuf {
    app_data_root().join("logs")
}

pub fn diagnostics_dir() -> PathBuf {
    app_data_root().join("diagnostics")
}

/// Create all data directories if missing. Called once at startup.
pub fn ensure_dirs() -> std::io::Result<()> {
    for dir in [config_dir(), data_dir(), log_dir(), diagnostics_dir()] {
        std::fs::create_dir_all(dir)?;
    }
    Ok(())
}
