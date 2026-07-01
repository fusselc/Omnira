//! Advanced Diagnostics: status snapshots and redacted export
//! (docs/data-ownership-and-storage.md section 7).

use crate::errors::AppError;
use crate::logging;
use crate::paths;
use crate::runtime::RuntimeManager;
use crate::types::DiagnosticsSnapshot;

pub fn snapshot(runtime: &RuntimeManager) -> DiagnosticsSnapshot {
    let status = runtime.status();
    DiagnosticsSnapshot {
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        recent_errors: status.last_error.clone().into_iter().collect(),
        runtime: status,
        data_dir: paths::app_data_root().display().to_string(),
        config_path: paths::settings_path().display().to_string(),
        db_path: paths::db_path().display().to_string(),
        log_dir: paths::log_dir().display().to_string(),
        recent_log_lines: logging::recent_lines(200),
    }
}

/// Mask the user's account name in Windows paths, e.g.
/// `C:\Users\alice\...` -> `C:\Users\<user>\...`.
fn redact_line(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut rest = line;
    loop {
        let lower = rest.to_ascii_lowercase();
        let Some(idx) = lower.find("\\users\\") else {
            out.push_str(rest);
            break;
        };
        let after = idx + "\\users\\".len();
        out.push_str(&rest[..after]);
        let tail = &rest[after..];
        let end = tail.find('\\').unwrap_or(tail.len());
        out.push_str("<user>");
        rest = &tail[end..];
    }
    out
}

/// Write a diagnostics report to %LOCALAPPDATA%\Omnira\diagnostics\ and return
/// its path. Redacted by default; `include_paths` is the explicit opt-in.
pub fn export(runtime: &RuntimeManager, include_paths: bool) -> Result<String, AppError> {
    let snap = snapshot(runtime);
    let mut report = serde_json::to_string_pretty(&snap)
        .expect("DiagnosticsSnapshot is always serializable");

    if !include_paths {
        report = report
            .lines()
            .map(redact_line)
            .collect::<Vec<_>>()
            .join("\n");
    }

    let filename = format!(
        "omnira-diagnostics-{}.json",
        chrono::Local::now().format("%Y%m%d-%H%M%S")
    );
    let path = paths::diagnostics_dir().join(filename);
    std::fs::write(&path, report)?;
    logging::info("diagnostics.export", &format!("redacted={}", !include_paths));
    Ok(path.display().to_string())
}
