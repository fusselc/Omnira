//! Human-readable settings file: %LOCALAPPDATA%\Omnira\config\settings.json.

use crate::errors::AppError;
use crate::paths;
use crate::types::Settings;

pub fn load() -> Settings {
    let path = paths::settings_path();
    match std::fs::read_to_string(&path) {
        Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
        Err(_) => Settings::default(),
    }
}

pub fn save(settings: &Settings) -> Result<(), AppError> {
    let path = paths::settings_path();
    let text = serde_json::to_string_pretty(settings)
        .expect("Settings is always serializable");
    std::fs::write(&path, text)?;
    Ok(())
}
