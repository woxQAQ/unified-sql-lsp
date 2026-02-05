// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! E2E test logging module
//!
//! Buffers debug output in memory and writes to log file only on test failure.
//! This keeps terminal output clean while preserving detailed diagnostic information.

use std::fs::{File, create_dir_all};
use std::io::BufWriter;
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::SystemTime;

const MAX_BUFFER_SIZE: usize = 10_000_000; // 10MB limit
const LOG_DIR: &str = "target/e2e-logs";

/// Find workspace root by searching upward for a Cargo.toml with [workspace] section
fn find_workspace_root() -> std::io::Result<PathBuf> {
    // Start from CARGO_MANIFEST_DIR (the package being tested)
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").map_err(|_| {
        std::io::Error::new(std::io::ErrorKind::NotFound, "CARGO_MANIFEST_DIR not set")
    })?;
    let mut current = PathBuf::from(manifest_dir);

    loop {
        let cargo_toml = current.join("Cargo.toml");
        if cargo_toml.exists() {
            // Check if it has [workspace] section
            if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
                if content.contains("[workspace]") {
                    return Ok(current.to_path_buf());
                }
            }
        }

        // Move to parent directory
        match current.parent() {
            Some(parent) if !parent.as_os_str().is_empty() => {
                current = parent.to_path_buf();
            }
            _ => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Workspace root not found",
                ));
            }
        }
    }
}

/// Global logging state
static LOGGING_STATE: OnceLock<LoggingState> = OnceLock::new();

/// Logging state manager
pub struct LoggingState {
    /// Buffer for all debug output before first failure
    buffer: Mutex<Vec<String>>,

    /// Log file writer (created on first failure)
    log_file: Mutex<Option<BufWriter<File>>>,

    /// Whether we've flushed buffer to disk yet
    flushed: AtomicBool,

    /// Log file path for display
    log_path: Mutex<Option<PathBuf>>,
}

impl LoggingState {
    /// Create a new logging state
    pub fn new() -> Self {
        Self {
            buffer: Mutex::new(Vec::new()),
            log_file: Mutex::new(None),
            flushed: AtomicBool::new(false),
            log_path: Mutex::new(None),
        }
    }

    /// Record a log line (buffered in memory until first failure)
    pub fn record(&self, line: String) {
        if self.flushed.load(Ordering::Relaxed) {
            // Already flushed - write directly to file
            self.append_to_file(line);
        } else {
            // Still buffering - check size limit
            let mut buffer = self.buffer.lock().unwrap();
            let current_size: usize = buffer.iter().map(|s| s.len()).sum();

            if current_size > MAX_BUFFER_SIZE {
                // Buffer too large - flush early
                drop(buffer);
                let _ = self.flush_to_file_internal();
                self.append_to_file(line);
            } else {
                buffer.push(line);
            }
        }
    }

    /// Flush buffer to file (called on first test failure)
    fn flush_to_file_internal(&self) -> std::io::Result<()> {
        if self.flushed.load(Ordering::Relaxed) {
            return Ok(()); // Already flushed
        }

        // Find workspace root by searching upward from CARGO_MANIFEST_DIR
        let workspace_root = find_workspace_root().unwrap_or_else(|_| PathBuf::from("."));

        // Create log directory in workspace root
        let log_dir = workspace_root.join(LOG_DIR);
        create_dir_all(&log_dir)?;

        // Generate timestamped filename
        let timestamp = format_timestamp();
        let filename = format!("e2e-fail-{}.log", timestamp);
        let log_path = log_dir.join(filename);

        // Create file
        let file = File::create(&log_path)?;
        let mut writer = BufWriter::new(file);

        // Write buffered content with timestamps
        let buffer = self.buffer.lock().unwrap();
        for line in buffer.iter() {
            let timestamped = format_timestamped_line(line);
            use std::io::Write;
            writeln!(writer, "{}", timestamped)?;
        }
        drop(buffer);

        // Store writer and path
        let mut log_file = self.log_file.lock().unwrap();
        *log_file = Some(writer);
        *self.log_path.lock().unwrap() = Some(log_path.clone());
        drop(log_file);

        self.flushed.store(true, Ordering::Relaxed);
        Ok(())
    }

    /// Append line directly to file (after flush)
    fn append_to_file(&self, line: String) {
        let mut log_file = self.log_file.lock().unwrap();
        if let Some(ref mut writer) = *log_file {
            use std::io::Write;
            let timestamped = format_timestamped_line(&line);
            let _ = writeln!(writer, "{}", timestamped);
            let _ = writer.flush();
        }
    }

    /// Get the log file path (if created)
    pub fn get_log_path(&self) -> Option<PathBuf> {
        self.log_path.lock().unwrap().clone()
    }
}

/// Format current time as timestamp (YYYYMMDD-HHMMSS)
fn format_timestamp() -> String {
    use std::time::UNIX_EPOCH;
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();

    let secs = duration.as_secs();
    let datetime = chrono::DateTime::from_timestamp(secs as i64, 0).unwrap_or_default();

    datetime.format("%Y%m%d-%H%M%S").to_string()
}

/// Format a log line with timestamp
fn format_timestamped_line(line: &str) -> String {
    use std::time::UNIX_EPOCH;
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();

    let secs = duration.as_secs();
    let datetime = chrono::DateTime::from_timestamp(secs as i64, 0).unwrap_or_default();

    format!("[{}] {}", datetime.format("%Y-%m-%d %H:%M:%S"), line)
}

/// Initialize the global logging state
pub fn initialize() {
    LOGGING_STATE.get_or_init(|| LoggingState::new());
}

/// Record a log line
pub fn record(line: String) {
    if let Some(state) = LOGGING_STATE.get() {
        state.record(line);
    }
}

/// Flush buffer to file (call this on first test failure)
pub fn flush_to_file() -> anyhow::Result<()> {
    if let Some(state) = LOGGING_STATE.get() {
        state
            .flush_to_file_internal()
            .map_err(|e| anyhow::anyhow!("Failed to flush log to file: {}", e))?;
    }
    Ok(())
}

/// Get log file path for display
pub fn log_path() -> Option<PathBuf> {
    LOGGING_STATE.get()?.get_log_path()
}

/// Check if logging is enabled
pub fn is_enabled() -> bool {
    LOGGING_STATE.get().is_some()
}

/// Check if we should show in terminal (always false for debug output)
pub fn should_show_in_terminal() -> bool {
    false
}

/// Debug logging macro that buffers output and writes to file only on test failure.
///
/// # Usage
///
/// ```rust,ignore
/// use crate::debug_log;
///
/// debug_log!("Test message: {}", value);
/// debug_log!("!!! CLIENT: Received response");
/// ```
///
/// This replaces `eprintln!` for debug output to keep terminal clean.
#[macro_export]
macro_rules! debug_log {
    ($($arg:tt)*) => {
        if $crate::logging::is_enabled() {
            let msg = format!($($arg)*);
            $crate::logging::record(msg);
        }
    };
}

// Re-export OnceLock
use std::sync::OnceLock;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_timestamp() {
        let timestamp = format_timestamp();
        // Format: YYYYMMDD-HHMMSS (8 digits + 1 dash + 6 digits = 15 chars)
        assert!(timestamp.len() == 15);
        assert!(timestamp.contains('-'));
    }

    #[test]
    fn test_format_timestamped_line() {
        let line = "Test message";
        let timestamped = format_timestamped_line(line);
        assert!(timestamped.starts_with("["));
        assert!(timestamped.ends_with("] Test message"));
    }
}
