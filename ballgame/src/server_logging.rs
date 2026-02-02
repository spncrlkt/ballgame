//! Server-side file logging setup
//!
//! Initializes tracing to write to both console and a timestamped log file.
//! Must be called BEFORE Bevy's DefaultPlugins are added.

use std::fs;
use std::path::Path;
use chrono::Local;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Log directory for server logs
const LOG_DIR: &str = "logs";

/// Initialize logging to both console and file.
/// Returns the path to the log file.
///
/// This MUST be called before Bevy's DefaultPlugins are added,
/// and you must disable Bevy's LogPlugin to avoid conflicts.
pub fn init_server_logging(server_mode: bool) -> Option<String> {
    // Only set up file logging in server mode
    if !server_mode {
        return None;
    }

    // Ensure log directory exists
    if let Err(e) = fs::create_dir_all(LOG_DIR) {
        eprintln!("Warning: Failed to create log directory: {}", e);
        return None;
    }

    // Create timestamped log file name
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let log_filename = format!("server_{}.log", timestamp);
    let log_path = Path::new(LOG_DIR).join(&log_filename);

    // Create file appender
    let file_appender = RollingFileAppender::new(Rotation::NEVER, LOG_DIR, &log_filename);

    // Set up filter - info level by default, can be overridden with RUST_LOG
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,wgpu=warn,naga=warn,bevy_render=warn"));

    // Create console layer (matches Bevy's default format)
    let console_layer = fmt::layer()
        .with_target(true)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false);

    // Create file layer with more detail
    let file_layer = fmt::layer()
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .with_ansi(false)
        .with_writer(file_appender);

    // Initialize subscriber with both layers
    tracing_subscriber::registry()
        .with(filter)
        .with(console_layer)
        .with(file_layer)
        .init();

    Some(log_path.to_string_lossy().to_string())
}
