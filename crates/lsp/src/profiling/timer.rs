//! Scoped timing utilities for performance profiling
//!
//! # Example
//!
//! ```ignore
//! use crate::profiling::ScopedTimer;
//!
//! {
//!     let _timer = ScopedTimer::new("my_operation");
//!     // ... do work ...
//! } // Timer automatically records elapsed time when dropped
//! ```

use std::time::Instant;

/// A scoped timer that measures execution time
///
/// Automatically records the elapsed time to the global TimingCollector
/// when dropped. Use the `timed_scope!` macro for more ergonomic usage.
///
/// # Overhead
///
/// When the `profiling` feature is enabled, overhead is approximately:
/// - Construction: ~50ns (one Instant::now() call)
/// - Destruction: ~200ns (recording to thread-local storage)
///
/// When disabled, compiles to nothing (zero overhead).
#[derive(Debug)]
pub struct ScopedTimer {
    name: &'static str,
    start: Instant,
}

impl ScopedTimer {
    /// Create a new scoped timer with the given name
    ///
    /// # Example
    ///
    /// ```ignore
    /// let _timer = ScopedTimer::new("parse_cst");
    /// // ... parsing code ...
    /// ```
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            start: Instant::now(),
        }
    }
}

impl Drop for ScopedTimer {
    fn drop(&mut self) {
        let duration = self.start.elapsed();
        super::stats::TimingCollector::record(self.name, duration);
    }
}

/// Macro for creating scoped timers with ergonomic syntax
///
/// # Example
///
/// ```ignore
/// use crate::profiling::timed_scope;
///
/// fn my_function() {
///     timed_scope!("my_function");
///     // ... code ...
/// } // Timing automatically recorded here
/// ```
#[macro_export]
macro_rules! timed_scope {
    ($name:expr) => {
        let _timer = $crate::profiling::ScopedTimer::new($name);
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profiling::stats::TimingCollector;

    #[test]
    fn test_scoped_timer_timing() {
        // Clear any existing timings
        TimingCollector::clear();

        {
            let _timer = ScopedTimer::new("test_operation");
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        let report = TimingCollector::report();
        assert!(report.scopes.contains_key("test_operation"));
    }
}
