//! Statistics collection for performance profiling
//!
//! Thread-local storage for timing data with percentile calculation

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::time::Duration;

thread_local! {
    /// Thread-local storage for timing data
    static TIMINGS: RefCell<HashMap<&'static str, Vec<Duration>>> =
        RefCell::new(HashMap::new());
}

/// Collector for timing data
///
/// # Example
///
/// ```ignore
/// // Record a timing
/// TimingCollector::record("parse_cst", Duration::from_micros(500));
///
/// // Generate report
/// let report = TimingCollector::report();
/// report.print_summary();
/// ```
pub struct TimingCollector;

impl TimingCollector {
    /// Record a timing measurement
    ///
    /// This is called automatically by ScopedTimer when dropped.
    pub fn record(name: &'static str, duration: Duration) {
        TIMINGS.with(|timings| {
            timings.borrow_mut()
                .entry(name)
                .or_insert_with(Vec::new)
                .push(duration);
        });
    }

    /// Generate a report of all recorded timings
    ///
    /// Returns a TimingReport with calculated statistics (min, max, avg, p95, p99).
    pub fn report() -> TimingReport {
        let mut scopes = HashMap::new();

        TIMINGS.with(|timings| {
            let timings = timings.borrow();
            for (name, durations) in timings.iter() {
                if !durations.is_empty() {
                    scopes.insert(*name, TimingStats::from_durations(durations));
                }
            }
        });

        TimingReport { scopes }
    }

    /// Clear all recorded timings
    pub fn clear() {
        TIMINGS.with(|timings| {
            timings.borrow_mut().clear();
        });
    }
}

/// Statistics for a single scope
#[derive(Debug, Clone)]
pub struct TimingStats {
    /// Number of times this scope was executed
    pub count: usize,
    /// Total time spent in this scope
    pub total: Duration,
    /// Minimum execution time
    pub min: Duration,
    /// Maximum execution time
    pub max: Duration,
    /// Average execution time
    pub avg: Duration,
    /// 95th percentile execution time
    pub p95: Duration,
    /// 99th percentile execution time
    pub p99: Duration,
}

impl TimingStats {
    /// Calculate statistics from a slice of durations
    fn from_durations(durations: &[Duration]) -> Self {
        assert!(!durations.is_empty(), "Cannot calculate stats for empty slice");

        let mut sorted: Vec<_> = durations.iter().copied().collect();
        sorted.sort();

        let count = sorted.len();
        let total: Duration = sorted.iter().sum();
        let min = sorted[0];
        let max = sorted[count - 1];
        let avg = total / count as u32;

        let p95_idx = (count as f64 * 0.95) as usize;
        let p95 = sorted[p95_idx.min(count - 1)];

        let p99_idx = (count as f64 * 0.99) as usize;
        let p99 = sorted[p99_idx.min(count - 1)];

        Self {
            count,
            total,
            min,
            max,
            avg,
            p95,
            p99,
        }
    }
}

/// Report containing statistics for all scopes
#[derive(Debug, Clone)]
pub struct TimingReport {
    pub scopes: HashMap<&'static str, TimingStats>,
}

impl TimingReport {
    /// Print a formatted summary of all timings
    pub fn print_summary(&self) {
        println!("\n=== Performance Profile ===");

        let sorted_scopes: BTreeMap<&_, &_> = self.scopes.iter().collect();
        for (scope, stats) in sorted_scopes {
            println!(
                "{:30} | calls: {:6} | avg: {:8.2?} | p95: {:8.2?} | p99: {:8.2?}",
                scope, stats.count, stats.avg, stats.p95, stats.p99
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timing_stats_calculation() {
        let durations = vec![
            Duration::from_micros(100),
            Duration::from_micros(200),
            Duration::from_micros(300),
            Duration::from_micros(400),
            Duration::from_micros(500),
        ];

        let stats = TimingStats::from_durations(&durations);

        assert_eq!(stats.count, 5);
        assert_eq!(stats.min, Duration::from_micros(100));
        assert_eq!(stats.max, Duration::from_micros(500));
        assert_eq!(stats.avg, Duration::from_micros(300));
    }

    #[test]
    fn test_timing_collector() {
        TimingCollector::clear();

        TimingCollector::record("test1", Duration::from_micros(100));
        TimingCollector::record("test1", Duration::from_micros(200));
        TimingCollector::record("test2", Duration::from_micros(300));

        let report = TimingCollector::report();

        assert_eq!(report.scopes.len(), 2);
        assert!(report.scopes.contains_key("test1"));
        assert!(report.scopes.contains_key("test2"));
        assert_eq!(report.scopes["test1"].count, 2);
        assert_eq!(report.scopes["test2"].count, 1);
    }
}
