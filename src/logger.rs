//! Internal logging system for TCT
//!
//! This module provides structured logging capabilities using proper stderr/stdout streams
//! with appropriate log levels and conditional output based on debug/verbose flags.

use std::sync::atomic::{AtomicBool, Ordering};

/// Global debug flag for controlling debug output
static DEBUG_ENABLED: AtomicBool = AtomicBool::new(false);

/// Global verbose flag for detailed output
static VERBOSE_ENABLED: AtomicBool = AtomicBool::new(false);

/// Log levels for structured output
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LogLevel {
    /// Error messages (always shown)
    Error,
    /// Warning messages (always shown)
    Warn,
    /// Info messages (shown unless quiet)
    Info,
    /// Debug messages (shown only when debug enabled)
    Debug,
    /// Verbose messages (shown only when verbose enabled)
    Verbose,
}

/// Logger for TCT operations
pub struct Logger;

impl Logger {
    /// Initialize logging based on environment variables
    pub fn init() {
        // Check RUST_LOG environment variable
        if let Ok(rust_log) = std::env::var("RUST_LOG") {
            if rust_log.to_lowercase().contains("debug") {
                Self::enable_debug();
            }
            if rust_log.to_lowercase().contains("trace") {
                Self::enable_verbose();
            }
        }

        // Check TCT-specific environment variables
        if std::env::var("TCT_DEBUG").is_ok() {
            Self::enable_debug();
        }
        if std::env::var("TCT_VERBOSE").is_ok() {
            Self::enable_verbose();
        }
    }

    /// Enable debug output
    pub fn enable_debug() {
        DEBUG_ENABLED.store(true, Ordering::Relaxed);
    }

    /// Enable verbose output
    pub fn enable_verbose() {
        VERBOSE_ENABLED.store(true, Ordering::Relaxed);
    }

    /// Check if debug is enabled
    pub fn is_debug_enabled() -> bool {
        DEBUG_ENABLED.load(Ordering::Relaxed)
    }

    /// Check if verbose is enabled
    pub fn is_verbose_enabled() -> bool {
        VERBOSE_ENABLED.load(Ordering::Relaxed)
    }

    /// Log an error message
    pub fn error(message: &str) {
        use std::io::{self, Write};
        let _ = writeln!(io::stderr(), "❌ ERROR: {}", message);
    }

    /// Log a warning message
    pub fn warn(message: &str) {
        use std::io::{self, Write};
        let _ = writeln!(io::stderr(), "⚠️  WARN: {}", message);
    }

    /// Log an info message
    pub fn info(message: &str) {
        println!("ℹ️  INFO: {}", message);
    }

    /// Log a debug message (only shown when debug enabled)
    pub fn debug(message: &str) {
        if Self::is_debug_enabled() {
            use std::io::{self, Write};
            let _ = writeln!(io::stderr(), "🔍 DEBUG: {}", message);
        }
    }

    /// Log a verbose message (only shown when verbose enabled)
    pub fn verbose(message: &str) {
        if Self::is_verbose_enabled() {
            use std::io::{self, Write};
            let _ = writeln!(io::stderr(), "📝 VERBOSE: {}", message);
        }
    }

    /// Log with emoji prefix (for specific debugging contexts)
    pub fn debug_with_emoji(emoji: &str, message: &str) {
        if Self::is_debug_enabled() {
            use std::io::{self, Write};
            let _ = writeln!(io::stderr(), "{} {}", emoji, message);
        }
    }

    /// Log Eq trait analysis (specialized debug category)
    pub fn eq_analysis(message: &str) {
        if Self::is_debug_enabled() {
            use std::io::{self, Write};
            let _ = writeln!(io::stderr(), "🎯 EQ: {}", message);
        }
    }

    /// Log type mapping analysis (specialized debug category)
    pub fn type_mapping(message: &str) {
        if Self::is_debug_enabled() {
            use std::io::{self, Write};
            let _ = writeln!(io::stderr(), "🔧 TYPE: {}", message);
        }
    }

    /// Log schema analysis (specialized debug category)
    pub fn schema_analysis(message: &str) {
        if Self::is_debug_enabled() {
            use std::io::{self, Write};
            let _ = writeln!(io::stderr(), "📊 SCHEMA: {}", message);
        }
    }
}

/// Macros for convenient logging
#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {
        $crate::logger::Logger::error(&format!($($arg)*))
    };
}

/// Logs a warning message
///
/// Only displayed when log level is WARNING or higher.
#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => {
        $crate::logger::Logger::warn(&format!($($arg)*))
    };
}

/// Logs an informational message
///
/// Only displayed when log level is INFO or higher.
#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {
        $crate::logger::Logger::info(&format!($($arg)*))
    };
}

/// Logs a debug message
///
/// Only displayed when log level is DEBUG or higher.
#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => {
        $crate::logger::Logger::debug(&format!($($arg)*))
    };
}

/// Logs a verbose debug message
///
/// Only displayed when log level is VERBOSE.
#[macro_export]
macro_rules! log_verbose {
    ($($arg:tt)*) => {
        $crate::logger::Logger::verbose(&format!($($arg)*))
    };
}

/// Logs equality/trait analysis information
///
/// Only displayed when log level is EQ_ANALYSIS or higher.
#[macro_export]
macro_rules! log_eq_analysis {
    ($($arg:tt)*) => {
        $crate::logger::Logger::eq_analysis(&format!($($arg)*))
    };
}

/// Logs type mapping information
///
/// Only displayed when log level is TYPE_MAPPING or higher.
#[macro_export]
macro_rules! log_type_mapping {
    ($($arg:tt)*) => {
        $crate::logger::Logger::type_mapping(&format!($($arg)*))
    };
}

/// Logs schema analysis information
///
/// Only displayed when log level is SCHEMA_ANALYSIS or higher.
#[macro_export]
macro_rules! log_schema_analysis {
    ($($arg:tt)*) => {
        $crate::logger::Logger::schema_analysis(&format!($($arg)*))
    };
}
