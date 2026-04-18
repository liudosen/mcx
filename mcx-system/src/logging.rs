use chrono::{DateTime, FixedOffset, NaiveDate, Offset, Utc};
use once_cell::sync::OnceCell;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::{
    fmt::{self, format::Writer, time::FormatTime},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter, Layer,
};

static APP_LOG_GUARD: OnceCell<WorkerGuard> = OnceCell::new();
static ERROR_LOG_GUARD: OnceCell<WorkerGuard> = OnceCell::new();

fn utc_plus_8() -> FixedOffset {
    FixedOffset::east_opt(8 * 3600).unwrap_or_else(|| Utc.fix())
}

fn now_in_utc_plus_8() -> DateTime<FixedOffset> {
    Utc::now().with_timezone(&utc_plus_8())
}

struct UtcPlus8Timer;

impl FormatTime for UtcPlus8Timer {
    fn format_time(&self, w: &mut Writer<'_>) -> std::fmt::Result {
        let now = now_in_utc_plus_8();
        write!(
            w,
            "{}",
            now.to_rfc3339_opts(chrono::SecondsFormat::Micros, true)
        )
    }
}

#[derive(Debug, Clone)]
pub struct LoggingConfig {
    pub log_dir: PathBuf,
    #[allow(dead_code)]
    pub max_file_size: u64,
    pub app_keep_days: u32,
    pub error_keep_days: u32,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            log_dir: PathBuf::from("logs"),
            max_file_size: 50 * 1024 * 1024,
            app_keep_days: 3,
            error_keep_days: 30,
        }
    }
}

impl LoggingConfig {
    pub fn from_env() -> Self {
        let log_dir = std::env::var("LOG_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| get_executable_dir().join("logs"));

        let max_file_size = std::env::var("LOG_MAX_FILE_SIZE")
            .unwrap_or_else(|_| "50".to_string())
            .parse::<u64>()
            .map(|v| v * 1024 * 1024)
            .unwrap_or(50 * 1024 * 1024);

        let app_keep_days = read_keep_days(&["LOG_APP_KEEP_DAYS", "LOG_MAX_AGE_DAYS"], 3);
        let error_keep_days = read_keep_days(&["LOG_ERROR_KEEP_DAYS", "LOG_MAX_AGE_DAYS"], 30);

        Self {
            log_dir,
            max_file_size,
            app_keep_days,
            error_keep_days,
        }
    }
}

fn get_executable_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|parent| parent.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}

fn get_daily_log_dir(log_dir: &Path) -> PathBuf {
    let today = now_in_utc_plus_8().format("%Y-%m-%d").to_string();
    log_dir.join(today)
}

fn read_keep_days(keys: &[&str], default: u32) -> u32 {
    for key in keys {
        if let Ok(value) = std::env::var(key) {
            if let Ok(parsed) = value.parse::<u32>() {
                return parsed;
            }
        }
    }

    default
}

pub fn init_logging(config: Option<LoggingConfig>) {
    let config = config.unwrap_or_else(LoggingConfig::from_env);

    let app_root = config.log_dir.join("app");
    let error_root = config.log_dir.join("error");
    let app_daily_log_dir = get_daily_log_dir(&app_root);
    let error_daily_log_dir = get_daily_log_dir(&error_root);

    let file_logging_enabled = match std::fs::create_dir_all(&app_daily_log_dir)
        .and_then(|_| std::fs::create_dir_all(&error_daily_log_dir))
    {
        Ok(_) => true,
        Err(err) => {
            eprintln!(
                "Logging file setup failed, console-only logging enabled: {}",
                err
            );
            false
        }
    };

    if file_logging_enabled {
        cleanup_old_logs(&app_root, config.app_keep_days);
        cleanup_old_logs(&error_root, config.error_keep_days);
    }

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,welfare_store_api=debug"));

    let console_layer = fmt::layer()
        .with_span_events(FmtSpan::FULL)
        .with_timer(UtcPlus8Timer)
        .with_target(true)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_file(true)
        .with_line_number(true)
        .with_level(true)
        .with_ansi(true);

    let registry = tracing_subscriber::registry()
        .with(env_filter)
        .with(console_layer);

    if file_logging_enabled {
        let app_file_appender =
            RollingFileAppender::new(Rotation::DAILY, &app_daily_log_dir, "app.log");
        let (app_non_blocking, app_guard) = tracing_appender::non_blocking(app_file_appender);
        let _ = APP_LOG_GUARD.set(app_guard);

        let error_file_appender =
            RollingFileAppender::new(Rotation::DAILY, &error_daily_log_dir, "error.log");
        let (error_non_blocking, error_guard) = tracing_appender::non_blocking(error_file_appender);
        let _ = ERROR_LOG_GUARD.set(error_guard);

        registry
            .with(
                fmt::layer()
                    .with_span_events(FmtSpan::FULL)
                    .with_timer(UtcPlus8Timer)
                    .with_target(true)
                    .with_thread_ids(false)
                    .with_thread_names(false)
                    .with_file(true)
                    .with_line_number(true)
                    .with_level(true)
                    .with_ansi(false)
                    .with_writer(app_non_blocking),
            )
            .with(
                fmt::layer()
                    .with_span_events(FmtSpan::FULL)
                    .with_timer(UtcPlus8Timer)
                    .with_target(true)
                    .with_thread_ids(false)
                    .with_thread_names(false)
                    .with_file(true)
                    .with_line_number(true)
                    .with_level(true)
                    .with_ansi(false)
                    .with_writer(error_non_blocking)
                    .with_filter(LevelFilter::ERROR),
            )
            .init();

        cleanup_old_logs(&app_root, config.app_keep_days);
        cleanup_old_logs(&error_root, config.error_keep_days);
        spawn_periodic_cleanup(config.clone());
    } else {
        registry.init();
    }

    tracing::info!("Logging system initialized");
    tracing::info!("Log root: {}", config.log_dir.display());
    tracing::info!("App log dir: {}", app_daily_log_dir.display());
    tracing::info!("Error log dir: {}", error_daily_log_dir.display());
}

fn spawn_periodic_cleanup(config: LoggingConfig) {
    if config.app_keep_days == 0 && config.error_keep_days == 0 {
        return;
    }

    let app_root = config.log_dir.join("app");
    let error_root = config.log_dir.join("error");
    let app_keep_days = config.app_keep_days;
    let error_keep_days = config.error_keep_days;

    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(24 * 60 * 60));
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        loop {
            ticker.tick().await;
            cleanup_old_logs(&app_root, app_keep_days);
            cleanup_old_logs(&error_root, error_keep_days);
        }
    });
}

fn cleanup_old_logs(log_root: &Path, max_age_days: u32) {
    if max_age_days == 0 {
        return;
    }

    let cutoff = now_in_utc_plus_8() - chrono::Duration::days(max_age_days as i64);

    if let Ok(entries) = std::fs::read_dir(log_root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };

            let Ok(dir_date) = NaiveDate::parse_from_str(dir_name, "%Y-%m-%d") else {
                continue;
            };

            let Some(naive_dt) = dir_date.and_hms_opt(0, 0, 0) else {
                continue;
            };

            let dir_dt =
                chrono::DateTime::<FixedOffset>::from_naive_utc_and_offset(naive_dt, utc_plus_8());

            if dir_dt < cutoff {
                tracing::info!("Cleaning expired log directory: {}", path.display());
                if let Err(err) = std::fs::remove_dir_all(&path) {
                    tracing::warn!(
                        "Failed to remove expired log directory {}: {}",
                        path.display(),
                        err
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_executable_dir() {
        let dir = get_executable_dir();
        assert!(dir.exists() || !dir.as_os_str().is_empty());
    }

    #[test]
    fn test_logging_config_default() {
        let config = LoggingConfig::default();
        assert_eq!(config.app_keep_days, 3);
        assert_eq!(config.error_keep_days, 30);
        assert_eq!(config.max_file_size, 50 * 1024 * 1024);
    }

    #[test]
    fn test_get_daily_log_dir() {
        let base = PathBuf::from("logs");
        let daily = get_daily_log_dir(&base);
        let today = now_in_utc_plus_8().format("%Y-%m-%d").to_string();
        assert!(daily.to_str().unwrap().ends_with(&today));
    }

    #[test]
    fn test_utc_plus_8_offset() {
        assert_eq!(utc_plus_8().local_minus_utc(), 8 * 3600);
    }
}
