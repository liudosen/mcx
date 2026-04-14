use crate::error::AppError;
use crate::logging::LoggingConfig;
use crate::routes::admin::auth::check_admin;
use crate::routes::ApiResponse;
use crate::state::AppState;
use axum::{
    extract::{Query, State},
    http::HeaderMap,
    Json,
};
use chrono::{DateTime, FixedOffset, Utc};
use serde::{Deserialize, Serialize};
use std::{
    collections::VecDeque,
    fs::File,
    io::{BufRead, BufReader},
    path::{Component, Path, PathBuf},
    sync::Arc,
};

#[derive(Debug, Deserialize)]
pub struct LogQuery {
    pub limit: Option<usize>,
    pub kind: Option<LogKind>,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LogKind {
    App,
    Error,
}

#[derive(Debug, Serialize)]
pub struct LogLine {
    pub level: String,
    pub line: String,
}

#[derive(Debug, Serialize)]
pub struct LogViewerData {
    pub source: String,
    pub path: String,
    pub exists: bool,
    pub size_bytes: u64,
    pub modified_at: Option<String>,
    pub lines: Vec<LogLine>,
}

pub async fn get_recent_logs(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<LogQuery>,
) -> Result<Json<ApiResponse<LogViewerData>>, AppError> {
    check_admin(&state, &headers).await?;

    let limit = query.limit.unwrap_or(200).clamp(10, 1000);
    let kind = query.kind.unwrap_or(LogKind::App);
    let log_path = resolve_log_path(kind);
    let source = if std::env::var("LOG_DIR").is_ok() {
        format!(
            "env:LOG_DIR/{:?} -> {}",
            kind,
            display_normalized_path(&log_path).display()
        )
    } else {
        format!(
            "default/{:?} -> {}",
            kind,
            display_normalized_path(&log_path).display()
        )
    };

    let data = read_recent_lines(log_path, source, limit)?;
    Ok(Json(ApiResponse::success(data)))
}

fn resolve_log_path(kind: LogKind) -> PathBuf {
    let config = LoggingConfig::from_env();
    let root = match kind {
        LogKind::App => config.log_dir.join("app"),
        LogKind::Error => config.log_dir.join("error"),
    };

    let base_dir = if root.is_relative() {
        std::env::current_dir()
            .map(|cwd| cwd.join(&root))
            .unwrap_or(root)
    } else {
        root
    };

    let file_prefix = match kind {
        LogKind::App => "app.log",
        LogKind::Error => "error.log",
    };

    if let Some(latest) = resolve_latest_daily_log(&base_dir, file_prefix) {
        return latest;
    }

    base_dir.join(file_prefix)
}

fn resolve_latest_daily_log(log_root: &PathBuf, file_prefix: &str) -> Option<PathBuf> {
    let entries = std::fs::read_dir(log_root).ok()?;
    let mut candidates: Vec<(chrono::NaiveDate, std::time::SystemTime, PathBuf)> = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let dir_name = path.file_name()?.to_str()?;
        let dir_date = chrono::NaiveDate::parse_from_str(dir_name, "%Y-%m-%d").ok()?;
        let dir_entries = std::fs::read_dir(&path).ok()?;
        for file_entry in dir_entries.flatten() {
            let file_path = file_entry.path();
            if !file_path.is_file() {
                continue;
            }

            let Some(file_name) = file_path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };

            if !file_name.starts_with(file_prefix) {
                continue;
            }

            let modified = file_entry
                .metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
            candidates.push((dir_date, modified, file_path));
        }
    }

    candidates
        .into_iter()
        .max_by(|(date_a, modified_a, path_a), (date_b, modified_b, path_b)| {
            date_a
                .cmp(date_b)
                .then_with(|| modified_a.cmp(modified_b))
                .then_with(|| path_a.cmp(path_b))
        })
        .map(|(_, _, path)| path)
}

fn display_normalized_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    normalized
}

fn read_recent_lines(
    log_path: PathBuf,
    source: String,
    limit: usize,
) -> Result<LogViewerData, AppError> {
    if !log_path.exists() {
        return Ok(LogViewerData {
            source,
            path: log_path.display().to_string(),
            exists: false,
            size_bytes: 0,
            modified_at: None,
            lines: vec![],
        });
    }

    let file = File::open(&log_path).map_err(|err| {
        AppError::InternalError(format!(
            "Failed to open log file {}: {}",
            log_path.display(),
            err
        ))
    })?;

    let metadata = file.metadata().map_err(|err| {
        AppError::InternalError(format!(
            "Failed to read log metadata {}: {}",
            log_path.display(),
            err
        ))
    })?;

    let modified_at = metadata.modified().ok().map(|time| {
        let dt_utc: DateTime<Utc> = time.into();
        dt_utc
            .with_timezone(&FixedOffset::east_opt(8 * 3600).expect("valid UTC+8 offset"))
            .format("%Y-%m-%d %H:%M:%S")
            .to_string()
    });

    let reader = BufReader::new(file);
    let mut lines = VecDeque::with_capacity(limit);

    for line in reader.lines() {
        let line = line.map_err(|err| {
            AppError::InternalError(format!(
                "Failed to read log line {}: {}",
                log_path.display(),
                err
            ))
        })?;

        if lines.len() == limit {
            lines.pop_front();
        }

        lines.push_back(LogLine {
            level: detect_level(&line),
            line,
        });
    }

    Ok(LogViewerData {
        source,
        path: log_path.display().to_string(),
        exists: true,
        size_bytes: metadata.len(),
        modified_at,
        lines: lines.into_iter().collect(),
    })
}

fn detect_level(line: &str) -> String {
    let upper = line.to_ascii_uppercase();
    if upper.contains(" ERROR ") || upper.starts_with("ERROR") || upper.contains("] ERROR") {
        "ERROR".to_string()
    } else if upper.contains(" WARN ") || upper.starts_with("WARN") || upper.contains("] WARN") {
        "WARN".to_string()
    } else if upper.contains(" DEBUG ") || upper.starts_with("DEBUG") || upper.contains("] DEBUG") {
        "DEBUG".to_string()
    } else if upper.contains(" TRACE ") || upper.starts_with("TRACE") || upper.contains("] TRACE") {
        "TRACE".to_string()
    } else {
        "INFO".to_string()
    }
}
