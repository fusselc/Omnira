//! Advanced Diagnostics: status snapshots and redacted export
//! (docs/data-ownership-and-storage.md section 7).

use std::path::Path;

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
        if let Some(idx) = lower.find(r"\\users\\") {
            let after = idx + r"\\users\\".len();
            out.push_str(&rest[..after]);
            let tail = &rest[after..];
            let end = tail.find(r"\\").unwrap_or(tail.len());
            out.push_str("<user>");
            rest = &tail[end..];
        } else if let Some(idx) = lower.find(r"\users\") {
            let after = idx + r"\users\".len();
            out.push_str(&rest[..after]);
            let tail = &rest[after..];
            let end = tail.find('\\').unwrap_or(tail.len());
            out.push_str("<user>");
            rest = &tail[end..];
        } else {
            out.push_str(rest);
            break;
        }
    }
    out
}

/// Write a diagnostics report and return its path. Redacted by default;
/// `include_paths` is the explicit opt-in.
///
/// When `output_dir` is `None`, writes to `%LOCALAPPDATA%\Omnira\diagnostics\`.
pub fn export(
    runtime: &RuntimeManager,
    include_paths: bool,
    output_dir: Option<&Path>,
) -> Result<String, AppError> {
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
    let dir = match output_dir {
        Some(dir) => dir.to_path_buf(),
        None => paths::diagnostics_dir(),
    };
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(filename);
    std::fs::write(&path, report)?;
    logging::info("diagnostics.export", &format!("redacted={}", !include_paths));
    Ok(path.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    struct TempExportDir(PathBuf);

    impl TempExportDir {
        fn new() -> Self {
            let base = std::env::temp_dir().join(format!(
                "omnira-diagnostics-test-{}",
                uuid::Uuid::new_v4()
            ));
            std::fs::create_dir_all(&base).expect("create temp export dir");
            Self(base)
        }
    }

    impl Drop for TempExportDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn windows_profile_username() -> String {
        if let Ok(name) = std::env::var("USERNAME") {
            if !name.is_empty() {
                return name;
            }
        }
        paths::app_data_root()
            .parent()
            .and_then(|p| p.parent())
            .and_then(|p| p.parent())
            .and_then(|p| p.file_name())
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "unknown".to_string())
    }

    #[test]
    fn test_redact_line() {
        assert_eq!(
            redact_line(r"C:\Users\alice\AppData\Local\Temp"),
            r"C:\Users\<user>\AppData\Local\Temp"
        );
        assert_eq!(
            redact_line(r"C:\\Users\\alice\\AppData\\Local\\Temp"),
            r"C:\\Users\\<user>\\AppData\\Local\\Temp"
        );
        assert_eq!(
            redact_line(r"some log line C:\Users\bob\logs\test.log here"),
            r"some log line C:\Users\<user>\logs\test.log here"
        );
        assert_eq!(
            redact_line(r#"some JSON: { "path": "C:\\Users\\bob\\file.txt" }"#),
            r#"some JSON: { "path": "C:\\Users\\<user>\\file.txt" }"#
        );
    }

    #[test]
    fn test_diagnostics_export() {
        let username = windows_profile_username();
        let temp_dir = TempExportDir::new();
        let runtime_mgr = RuntimeManager::default();
        let path_str =
            export(&runtime_mgr, false, Some(&temp_dir.0)).expect("export should succeed");
        let path = Path::new(&path_str);
        assert!(path.is_file());
        assert!(
            path.starts_with(&temp_dir.0),
            "export must write to the isolated test directory"
        );
        let content = std::fs::read_to_string(path).expect("read export file");
        assert!(
            !content.contains(&username),
            "username {username:?} must be redacted"
        );
        assert!(content.contains("<user>"), "redaction token must be present");
    }
}
