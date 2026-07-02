//! Prompt-free local logging (docs/local-security-boundary.md section 6).
//!
//! Log lines record lifecycle events, runtime start/stop, structured error
//! codes, and minimal metadata. Never prompt or response content.

use std::fs::OpenOptions;
use std::io::Write;
use std::sync::Mutex;

use crate::paths;

static LOG_LOCK: Mutex<()> = Mutex::new(());

fn log_file_path() -> std::path::PathBuf {
    let date = chrono::Local::now().format("%Y-%m-%d");
    paths::log_dir().join(format!("omnira-{date}.log"))
}

pub fn log(level: &str, event: &str, detail: &str) {
    let _guard = LOG_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let line = format!(
        "{} [{level}] {event} {detail}\n",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f")
    );
    if let Ok(mut f) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_file_path())
    {
        let _ = f.write_all(line.as_bytes());
    }
}

pub fn info(event: &str, detail: &str) {
    log("INFO", event, detail);
}

pub fn error(event: &str, detail: &str) {
    log("ERROR", event, detail);
}

/// Tail of the most recent log file, for the Diagnostics log viewer.
pub fn recent_lines(max: usize) -> Vec<String> {
    let Ok(text) = std::fs::read_to_string(log_file_path()) else {
        return Vec::new();
    };
    let lines: Vec<&str> = text.lines().collect();
    let start = lines.len().saturating_sub(max);
    lines[start..].iter().map(|s| s.to_string()).collect()
}
