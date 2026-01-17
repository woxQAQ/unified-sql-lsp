//! Statistics collection for performance profiling
//!
//! Thread-local storage for timing data with percentile calculation
//!
//! # Placeholder
//!
//! This is a temporary placeholder. Full implementation coming in Task 4.

use std::time::Duration;

/// Collector for timing data
///
/// # Placeholder
pub struct TimingCollector;

impl TimingCollector {
    /// Record a timing measurement
    pub fn record(_name: &'static str, _duration: Duration) {
        // Placeholder - will be implemented in Task 4
    }

    /// Placeholder new function for tests
    pub fn new() -> Self {
        Self
    }

    /// Placeholder report function for tests
    pub fn report(&self) -> TimingReport {
        TimingReport { scopes: std::collections::HashMap::new() }
    }
}

/// Statistics for a single scope (placeholder)
#[derive(Debug, Clone)]
pub struct TimingStats;

/// Report containing statistics for all scopes
#[derive(Debug, Clone)]
pub struct TimingReport {
    pub scopes: std::collections::HashMap<&'static str, ()>,
}
