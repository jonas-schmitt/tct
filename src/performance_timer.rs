//! Lightweight timing aggregation used by local performance reports.

use rustc_hash::FxHashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Simple performance timing utility
///
/// Automatically measures elapsed time from creation to drop,
/// reporting operations that take longer than 1ms.
pub struct PerformanceTimer {
    /// Name of the operation being timed
    pub name: &'static str,
    /// Start time of the operation
    pub start: Instant,
}

impl PerformanceTimer {
    /// Creates a new performance timer with the given name
    ///
    /// The timer starts immediately and reports elapsed time on drop.
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            start: Instant::now(),
        }
    }
}

impl Drop for PerformanceTimer {
    fn drop(&mut self) {
        let elapsed = self.start.elapsed();
        // Only report slow operations (> 1ms) to reduce noise
        if elapsed > Duration::from_millis(1) {
            eprintln!("⏱️  [{}] {:?}", self.name, elapsed);
        }

        // Also accumulate stats
        PERF_STATS.add_timing(self.name, elapsed);
    }
}

/// Global performance statistics
pub struct PerfStats {
    timings: Mutex<FxHashMap<String, (u64, Duration)>>, // (count, total_time)
}

impl Default for PerfStats {
    fn default() -> Self {
        Self::new()
    }
}

impl PerfStats {
    /// Creates a new performance statistics collector
    pub fn new() -> Self {
        Self {
            timings: Mutex::new(FxHashMap::default()),
        }
    }

    /// Adds a timing measurement for the given operation
    ///
    /// Accumulates both count and total duration for statistical reporting.
    pub fn add_timing(&self, name: &str, duration: Duration) {
        let mut timings = self
            .timings
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let entry = timings
            .entry(name.to_string())
            .or_insert((0, Duration::ZERO));
        entry.0 += 1;
        entry.1 += duration;
    }

    /// Reports the top 10 slowest operations to stderr
    ///
    /// Shows total time, call count, and average time per call.
    pub fn report(&self) {
        let timings = self
            .timings
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if timings.is_empty() {
            return;
        }

        eprintln!("\n📊 PERFORMANCE REPORT:");
        eprintln!("=====================");

        let mut sorted: Vec<_> = timings.iter().collect();
        sorted.sort_by_key(|(_, (_, total))| std::cmp::Reverse(*total));

        for (name, (count, total)) in sorted.iter().take(10) {
            let avg = *total / *count as u32;
            eprintln!(
                "  {:30} | {:6} calls | {:10.3?} total | {:8.3?} avg",
                name, count, total, avg
            );
        }
        eprintln!();
    }

    /// Clears all accumulated timing statistics
    pub fn clear(&self) {
        self.timings
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clear();
    }
}

lazy_static::lazy_static! {
    /// Global performance statistics singleton
    ///
    /// Automatically accumulates timings from all PerformanceTimer instances.
    pub static ref PERF_STATS: PerfStats = PerfStats::new();
}

/// Macro for easy timing
#[macro_export]
macro_rules! time_operation {
    ($name:expr) => {
        // DISABLED FOR PERFORMANCE TESTING - let _timer = $crate::performance_timer::PerformanceTimer::new($name);
    };
    ($name:expr, $code:block) => {{
        let _timer = $crate::performance_timer::PerformanceTimer::new($name);
        $code
    }};
}

/// Report all accumulated performance stats
pub fn report_performance_stats() {
    PERF_STATS.report();
}

/// Clear all performance stats
pub fn clear_performance_stats() {
    PERF_STATS.clear();
}
